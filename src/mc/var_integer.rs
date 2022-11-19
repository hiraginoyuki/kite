use async_trait::async_trait;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use core::cmp;
use core::slice;
use std::io::{self, ErrorKind, Read, Write};

const MSB: u8 = 0b1000_0000;

/// An error that can happen while parsing bytes as a varint or a varlong.
///
/// The following specification is written for myself to clarify what cases
/// to handle during the parsing process. As such, it might be difficult for
/// someone else to understand. If that is the case, feel free to ask the author.
///
/// ```
/// where
///     0 = (0--- ----) if i != 4,
///         (0000 ----) if i == 4,
///     1 = (1--- ----),
///     Q = (0aaa ----) where sum(a) > 0,
///
/// Ok(VarInt) if {
///     ~ [1--- ----{0-3}, 0--- ----]
///     ~ [1--- ----{ 4 }, 0000 ----]
///     = [0]
///     = [1, 0]
///     = [1, 1, 0]
///     = [1, 1, 1, 0]
///     = [1, 1, 1, 1, 0]
/// }
/// ```
#[derive(Debug, Error)]
pub enum VarIntError {
    /// No byte after a valid CONTINUE_BIT.
    /// ```
    /// ~ [1--- ----{0-4}]
    /// = []
    /// = [1]
    /// = [1, 1]
    /// = [1, 1, 1]
    /// = [1, 1, 1, 1]
    /// ```
    #[error("No byte after a valid CONTINUE_BIT.")]
    NotEnoughData,

    /// CONTINUE_BIT of the last byte is 1.
    /// ```
    /// ~ [1--- ----{ 5 }, ---- ----{*}]
    /// = [1, 1, 1, 1, 1, *]
    /// ```
    #[error("CONTINUE_BIT of the last byte is 1.")]
    BadContinueBit,

    /// Extra bytes follows the data, or the last byte contains extra bits.
    /// ```
    /// ~ [1--- ----{0-3}, 0--- ----, ---- ----{+}]
    /// ~ [1--- ----{ 4 }, 0000 ----, ---- ----{+}]
    /// ~ [1--- ----{ 4 }, 0aaa ----, ---- ----{*}]
    /// = [0, +]
    /// = [1, 0, +]
    /// = [1, 1, 0, +]
    /// = [1, 1, 1, 0, +]
    /// = [1, 1, 1, 1, 0, +]
    /// = [1, 1, 1, 1, Q, *]
    /// ~ [1--- ----{ j }, 1000 0000{ k }, 0000 0000] where 1 <= j and 0 <= (j + k) <= 4
    /// ```
    #[error("Extra bytes follows the data, or the last byte contains extra bits.")]
    RedundantData,

    /// The last few bytes contain only zero.
    #[error("The last few bytes contain only zero.")]
    Loose,
}

// r1:     i32 -> VarInt
// r2: [u8; 5] -> VarInt?
// r3:   &[u8] -> VarInt?
// r4: impl Read?
// r5: impl AsyncRead?

// w1: VarInt -> i32
// w2: VarInt -> [u8; 5]
// w3: VarInt ->&[u8; 5]
// w4: VarInt ->&[u8]
// w5: impl Write
// w6: impl AsyncWrite

#[derive(Debug, Clone)]
pub struct VarInt {
    len: usize,
    inner: [u8; VarInt::MAX_LEN],
}

impl VarInt {
    pub const MAX_LEN: usize = 5;
    pub const LAST_BYTE_MASK: u8 = 0b0000_1111;

    pub const fn len(&self) -> usize {
        self.len
    }
}

impl Default for VarInt {
    fn default() -> Self {
        Self {
            len: 1,
            inner: [0; Self::MAX_LEN],
        }
    }
}

// r1: i32 -> VarInt
impl From<i32> for StrictVarInt {
    fn from(source: i32) -> Self {
        let mut source = source as u32;
        let mut buf = [0u8; Self::MAX_LEN];
        let mut len = 0;

        for (index, byte) in buf.iter_mut().enumerate() {
            *byte = source as u8 & !MSB;
            source >>= 7;
            if source != 0 {
                *byte |= MSB
            } else {
                len = index;
                break;
            }
        }

        StrictVarInt(VarInt { len, inner: buf })
    }
}

// r2: [u8; 5] -> VarInt?
impl TryFrom<[u8; VarInt::MAX_LEN]> for VarInt {
    type Error = VarIntError;

    fn try_from(mut source: [u8; Self::MAX_LEN]) -> Result<Self, Self::Error> {
        todo!()
    }
}

// r3: &[u8] -> VarInt?
impl TryFrom<&[u8]> for VarInt {
    type Error = VarIntError;

    fn try_from(source: &[u8]) -> Result<Self, Self::Error> {
        use VarIntError::*;

        // might remove because it might be duplicate
        if source.len() > VarInt::MAX_LEN {
            return Err(RedundantData);
        }

        todo!()
    }
}

// r4: impl Read?
pub trait VarIntReadExt: Read {
    /// Tries to read a [`VarInt`].
    fn read_var_i32(&mut self) -> io::Result<VarInt> {
        let mut result = VarInt {
            inner: [0; VarInt::MAX_LEN],
            len: 0,
        };

        for (index, byte) in result.inner.iter_mut().enumerate() {
            self.read_exact(slice::from_mut(byte))?;
            if *byte & MSB == 0 {
                result.len = index + 1;
                break;
            }
        }

        // for loop didn't break
        if result.len == 0 {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                VarIntError::BadContinueBit,
            ));
        }

        // normalize and ignore
        result.inner[result.len - 1] &= if result.len == 5 {
            VarInt::LAST_BYTE_MASK
        } else {
            !MSB
        };

        Ok(result)
    }

    /// Tries to read a [`VarInt`] strictly. Returns [`ErrorKind::InvalidData`] with either [`VarIntError::BadContinueBit`] or [`VarIntError::RedundantData`].
    fn read_var_i32_strict(&mut self) -> io::Result<StrictVarInt> {
        let mut result = VarInt {
            inner: [0; VarInt::MAX_LEN],

            // apparent length of varint (might contain redundant bytes or bits)
            len: 0,
        };

        for (index, byte) in result.inner.iter_mut().enumerate() {
            self.read_exact(slice::from_mut(byte))?;
            if *byte & MSB == 0 {
                result.len = index + 1;
                break;
            }
        }

        // for loop didn't break
        if result.len == 0 {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                VarIntError::BadContinueBit,
            ));
        }

        // check the last byte for redundant bytes or bits
        let mask = if result.len == 5 {
            VarInt::LAST_BYTE_MASK
        } else {
            !MSB
        };
        if result.inner[result.len] & mask == 0 {
            return Err(io::Error::new(ErrorKind::InvalidData, VarIntError::Loose));
        }

        Ok(StrictVarInt(result))
    }
}
impl<R: Read> VarIntReadExt for R {}

// r5: impl AsyncRead?
#[async_trait]
pub trait VarIntAsyncReadExt: AsyncRead {
    /// Tries to read a [`VarInt`] loosely (ignoring [`VarIntError::RedundantData`]). Returns [`ErrorKind::InvalidData`] with [`VarIntError::BadContinueBit`].
    async fn read_var_i32(&mut self) -> io::Result<VarInt>
    where
        Self: Unpin,
    {
        let mut result = VarInt {
            inner: [0; VarInt::MAX_LEN],
            len: 0,
        };

        for (index, byte) in result.inner.iter_mut().enumerate() {
            self.read_exact(slice::from_mut(byte)).await?;
            if *byte & MSB == 0 {
                result.len = index + 1;
                break;
            }
        }

        // for loop didn't break
        if result.len == 0 {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                VarIntError::BadContinueBit,
            ));
        }

        // normalize and ignore
        result.inner[result.len - 1] &= if result.len == 5 {
            VarInt::LAST_BYTE_MASK
        } else {
            !MSB
        };

        Ok(result)
    }

    /// Tries to read a [`VarInt`] strictly. Returns [`ErrorKind::InvalidData`] with either [`VarIntError::BadContinueBit`] or [`VarIntError::RedundantData`].
    async fn read_var_i32_strict(&mut self) -> io::Result<StrictVarInt>
    where
        Self: Unpin,
    {
        let mut result = VarInt {
            inner: [0; VarInt::MAX_LEN],

            // apparent length of varint (might contain redundant bytes or bits)
            len: 0,
        };

        for (index, byte) in result.inner.iter_mut().enumerate() {
            self.read_exact(slice::from_mut(byte)).await?;
            if *byte & MSB == 0 {
                result.len = index + 1;
                break;
            }
        }

        // if loop didn't break
        if result.len == 0 {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                VarIntError::BadContinueBit,
            ));
        }

        // check the last byte for redundant bytes or bits
        let mask = if result.len == 5 {
            VarInt::LAST_BYTE_MASK
        } else {
            !MSB
        };
        if result.inner[result.len] & mask != 0 {
            return Err(io::Error::new(ErrorKind::InvalidData, VarIntError::Loose));
        }

        Ok(StrictVarInt(result))
    }
}
impl<R: AsyncRead> VarIntAsyncReadExt for R {}

// w1: VarInt -> i32
impl From<VarInt> for i32 {
    #[inline]
    fn from(source: VarInt) -> i32 {
        let mut result = 0u32;

        for (index, byte) in source.inner.iter().enumerate() {
            result |= ((byte & !MSB) as u32) << (7 * index);
            if byte & MSB == 0 {
                break;
            }
        }

        result as i32
    }
}

// w2: VarInt -> [u8; 5]
impl From<VarInt> for [u8; VarInt::MAX_LEN] {
    fn from(source: VarInt) -> Self {
        source.inner
    }
}

// w3: VarInt ->&[u8; 5]
impl AsRef<[u8; VarInt::MAX_LEN]> for VarInt {
    fn as_ref(&self) -> &[u8; VarInt::MAX_LEN] {
        &self.inner
    }
}

// w4: VarInt ->&[u8]
impl AsRef<[u8]> for VarInt {
    fn as_ref(&self) -> &[u8] {
        self.inner.as_ref()
    }
}

// w5: impl Write
pub trait VarIntWriteExt: Write {
    fn write_var_i32(&mut self, source: &VarInt) -> io::Result<()> {
        self.write_all(&source.inner[..source.len])?;
        Ok(())
    }
}
impl<W: Write> VarIntWriteExt for W {}

// w6: impl AsyncWrite
#[async_trait]
pub trait VarIntAsyncWriteExt: AsyncWrite {
    async fn write_var_i32(&mut self, source: &VarInt) -> io::Result<()>
    where
        Self: Unpin,
    {
        self.write_all(&source.inner[..source.len]).await?;
        Ok(())
    }
}
impl<W: AsyncWrite> VarIntAsyncWriteExt for W {}

pub trait SharedBehavior123123 {
    fn is_loose() -> bool;
}

mod strict {
    use derive_more::*;

    use super::{SharedBehavior123123, VarInt};

    #[derive(Debug, Default, Clone, AsRef, Into)]
    pub struct StrictVarInt(
        #[as_ref]
        #[into]
        pub(super) VarInt,
    );

    impl SharedBehavior123123 for StrictVarInt {
        fn is_loose() -> bool {
            false
        }
    }

    impl StrictVarInt {
        pub const MAX_LEN: usize = 5;
        pub const LAST_BYTE_MASK: u8 = 0b0000_1111;

        pub const fn len(&self) -> usize {
            self.0.len
        }
    }

    impl PartialEq for StrictVarInt {
        fn eq(&self, other: &Self) -> bool {
            self.0.inner == other.0.inner
        }
    }
    impl Eq for StrictVarInt {}
}

pub use strict::StrictVarInt;
