use crate::traits::convert::{AsInner, IntoInner, TryFromInner};
use async_trait::async_trait;
use core::num::TryFromIntError;
use std::io::{Read, Write};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

macro_rules! var_integer_impl {{
    $(#[$outer:meta])*
    struct $VarInteger: ident: From<$primitive: ty> + TryFrom<$primitive_unsigned: ty> {
        inner: [u8; $MAX_SIZE: literal],
        size: usize,

        @mask = $illegal_bits_in_last_byte:literal;
        fn $ReadExt:ident :: $fn_read:ident();
        fn $WriteExt:ident :: $fn_write:ident();
        fn $AsyncReadExt:ident :: $fn_async_read:ident();
        fn $AsyncWriteExt:ident :: $fn_async_write:ident();
        fn from($test_primitive:literal) = $test_bytes:expr;
    }
    $($impl: item)*
} => {
    $(#[$outer])*
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub struct $VarInteger {
        inner: [u8; Self::MAX_SIZE],
        size: usize,
    }

    /// ```
    #[doc = concat!(
        "use ", module_path!(), "::", stringify!($VarInteger), ";\n",
        "assert_eq!(", stringify!($VarInteger), "::MAX_SIZE, ", stringify!($MAX_SIZE), ");"
    )]
    /// ```
    impl $VarInteger {
        pub const MAX_SIZE: usize = $MAX_SIZE;
        /// Returns the number of bytes that make sense.
        pub fn size(&self) -> usize {
            self.size
        }
    }

    impl Default for $VarInteger {
        fn default() -> Self {
            Self {
                inner: [0; Self::MAX_SIZE],
                size: 1,
            }
        }
    }

    impl<'a> From<&'a $VarInteger> for &'a [u8; $VarInteger::MAX_SIZE] {
        fn from(value: &'a $VarInteger) -> Self {
            &value.inner
        }
    }
    impl AsInner for $VarInteger {
        type Inner = [u8; Self::MAX_SIZE];
        fn as_inner(&self) -> &Self::Inner {
            &self.inner
        }
    }

    impl From<$VarInteger> for [u8; $VarInteger::MAX_SIZE] {
        fn from(value: $VarInteger) -> Self {
            value.inner
        }
    }
    impl IntoInner for $VarInteger {
        type Inner = [u8; Self::MAX_SIZE];
        fn into_inner(self) -> Self::Inner {
            self.inner
        }
    }

    impl TryFrom<&[u8]> for $VarInteger {
        type Error = &'static str;
        fn try_from(value: &[u8]) -> Result<Self, &'static str> {
            let mut result = Self::default();
            result.size = match value.iter().enumerate().find(|(_, byte)| *byte & 0b1000_0000 == 0) {
                Some((idx, _)) => if idx < Self::MAX_SIZE {
                    idx + 1
                } else {
                    return Err("Too large");
                },
                _ => return Err("Invalid data"),
            };
            #[cfg(debug_assertions)]
            if result.size < value.len() {
                panic!("Received {} redundant bytes: {:?}", value.len() - result.size, &value[result.size..value.len()]);
            }
            result.inner[..result.size].copy_from_slice(&value[..result.size]);
            Ok(result)
        }
    }

    impl TryFromInner<[u8; $VarInteger::MAX_SIZE]> for $VarInteger {
        type Error = &'static str;
        fn try_from_inner(inner: [u8; Self::MAX_SIZE]) -> Result<Self, &'static str> {
            let mut result = Self {
                inner,
                size: match inner.iter().enumerate().find(|(_, byte)| *byte & 0b1000_0000 == 0) {
                    Some((idx, _)) => idx + 1,
                    _ => return Err("Invalid data"),
                }
            };
            result.inner[result.size..].fill(0);
            Ok(result)
        }
    }

    impl From<$primitive> for $VarInteger {
        fn from(mut val: $primitive) -> Self {
            let mut result = Self { inner: [0; Self::MAX_SIZE], size: 1 };
            if val == 0 { return result }
            for i in 0..$VarInteger::MAX_SIZE {
                result.inner[i] = (val & 0b0111_1111) as u8;
                val = val >> 7;
                if val == 0 {
                    break
                }
                result.inner[i] |= 0b1000_0000;
            }
            result
        }
    }
    impl TryFrom<$primitive_unsigned> for $VarInteger {
        type Error = TryFromIntError;
        fn try_from(value: $primitive_unsigned) -> Result<Self, Self::Error> {
            Ok(<$primitive>::try_from(value)?.into())
        }
    }

    impl From<&$VarInteger> for $primitive {
        fn from(val: &$VarInteger) -> Self {
            val.as_inner()[..val.size]
                .iter()
                .enumerate()
                .map(|(idx, &val)| ((val & 0b0_111_1111) << (7 * idx)) as Self)
                .sum()
        }
    }
    impl TryFrom<&$VarInteger> for $primitive_unsigned {
        type Error = TryFromIntError;
        fn try_from(value: &$VarInteger) -> Result<Self, Self::Error> {
            <$primitive>::from(value).try_into()
        }
    }

    impl<R: Read + ?Sized> $ReadExt for R {}
    trait $ReadExt: Read {
        fn $fn_read(&mut self) -> std::io::Result<$VarInteger> {
            let mut buf = [0; $VarInteger::MAX_SIZE];

            let mut idx = 0;
            let len = loop {
                self.read_exact(&mut buf[idx..=idx])?;
                if buf[idx] & 0b1_000_0000 == 0 {
                    break idx + 1;
                }
                idx += 1;
                if idx >= $VarInteger::MAX_SIZE {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Read data is too large"))
                }
            };

            Ok($VarInteger { inner: buf, size: len })
        }
    }

    impl<W: Write + ?Sized> $WriteExt for W {}
    pub trait $WriteExt: Write {
        fn $fn_write(&mut self, src: &$VarInteger) -> std::io::Result<()> {
            self.write_all(&src.inner[0..src.size])
        }
    }

    #[async_trait]
    impl<W: AsyncWrite + ?Sized> $AsyncWriteExt for W {}
    #[async_trait]
    pub trait $AsyncWriteExt: AsyncWrite {
        async fn $fn_async_write(&mut self, src: &$VarInteger) -> tokio::io::Result<()>
        where
            Self: Unpin
        {
            self.write_all(&src.inner[0..src.size]).await
        }
    }

    // impl<R: Read + ?Sized> $ReadExt for R {}
    // pub trait $ReadExt: Read {
    //     fn $fn_read(&mut self) -> std::io::Result<$VarInteger> {
    //         let mut result = $VarInteger::default();
    //         let mut buf = [0];
    //         for i in 0..=$VarInteger::MAX_SIZE-1 {
    //             self.read_exact(&mut buf)?;
    //             result.inner[i] = buf[0];
    //             if buf[0] & 0b1000_0000 == 0 {
    //                 result.size = i + 1;
    //                 break;
    //             }
    //         }
    //         if result.inner[$VarInteger::MAX_SIZE-1] & $illegal_bits_in_last_byte != 0 {
    //             return Err(tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, concat!("Read data is larger than ", stringify!($primitive::MAX))))
    //         }
    //         Ok(result)
    //     }
    // }

    // #[async_trait]
    // impl<R: AsyncRead + ?Sized> $AsyncReadExt for R {}
    // #[async_trait]
    // pub trait $AsyncReadExt: AsyncRead {
    //     async fn $fn_async_read(&mut self) -> tokio::io::Result<$VarInteger>
    //     where
    //         Self: Unpin
    //     {
    //         let mut result = $VarInteger::default();
    //         let mut buf = [0];
    //         for i in 0..=$VarInteger::MAX_SIZE-1 {
    //             self.read_exact(&mut buf).await?;
    //             result.inner[i] = buf[0];
    //             if buf[0] & 0b1000_0000 == 0 {
    //                 result.size = i + 1;
    //                 break;
    //             }
    //         }
    //         if result.inner[$VarInteger::MAX_SIZE-1] & $illegal_bits_in_last_byte != 0 {
    //             return Err(tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, concat!("Read data is larger than ", stringify!($primitive::MAX))))
    //         }
    //         Ok(result)
    //     }
    // }

    $($impl)*
}}

var_integer_impl! {
    /// ```
    /// use kite::mc::var_integer::VarInt;
    /// use kite::traits::convert::*;
    ///
    /// let int = 25565;
    /// let var_int_bytes: [u8; 5] = [0b1_101_1101, 0b1_100_0111, 0b0_000_0001, 0, 0];
    ///
    /// let var_int_converted = VarInt::from(int);
    /// let var_int = VarInt::try_from_inner(var_int_bytes).unwrap();
    ///
    /// assert_eq!(&var_int_bytes, var_int_converted.as_inner());
    /// assert_eq!(&var_int_bytes, var_int.as_inner());
    /// ```
    struct VarInt: From<i32> + TryFrom<u32> {
        inner: [u8; 5],
        size: usize,

        @mask = 0b11110000;
        fn ReadExtVarInt::read_var_int();
        fn WriteExtVarInt::write_var_int();
        fn AsyncReadExtVarInt::read_var_int();
        fn AsyncWriteExtVarInt::write_var_int();
        fn from(25565) = [0b1_101_1101, 0b1_100_0111, 0b0_000_0001, 0, 0];
    }
}

var_integer_impl! {
    struct VarLong: From<i64> + TryFrom<u64> {
        inner: [u8; 10],
        size: usize,

        @mask = 0b11111110;
        fn ReadExtVarLong::read_var_long();
        fn WriteExtVarLong::write_var_long();
        fn AsyncReadExtVarLong::read_var_long();
        fn AsyncWriteExtVarLong::write_var_long();
        fn from(25565) = [0b1_101_1101, 0b1_100_0111, 0b0_000_0001, 0, 0];
    }
}
