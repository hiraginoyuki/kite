use async_trait::async_trait;
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub struct VarInt {
    len: usize,
    inner: [u8; 5],
}
impl Default for VarInt {
    fn default() -> Self {
        VarInt {
            len: 1,
            inner: [0; 5],
        }
    }
}

impl From<&VarInt> for i32 {
    fn from(var_int: &VarInt) -> i32 {
        let mut result = 0u32;

        for i in 0..5 {
            result |= ((var_int.inner[i] & 0b0_111_1111) as u32) << (7 * i);
            if var_int.inner[i] & 0b1_000_0000 != 0 {
                break;
            }
        }

        result as i32
    }
}
impl From<i32> for VarInt {
    fn from(source: i32) -> Self {
        let mut source = source as u32;
        let mut buf = [0u8; 5];
        let mut len = 0;

        for i in 0..buf.len() {
            buf[i] = source as u8 & 0b0_111_1111;
            source >>= 7;
            if source != 0 {
                buf[i] |= 0b1_000_0000;
            } else {
                len = i;
                break;
            }
        }

        VarInt { len, inner: buf }
    }
}

impl<R: AsyncRead> VarIntAsyncReadExt for R {}
#[async_trait]
pub trait VarIntAsyncReadExt: AsyncRead {
    async fn read_var_i32(&mut self) -> io::Result<VarInt>
    where
        Self: Unpin,
    {
        let mut buf = [0; 5];
        let mut len = 0;

        for i in 0..buf.len() {
            self.read_exact(&mut buf[i..=i]).await?;
            if buf[i] & 0b1_000_0000 == 0 {
                len = i + 1;
                break;
            }
        }

        if len == 0 {
            // len was intact, meaning CONTINUE_BIT never became 0
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "varint too long",
            ));
        }

        let actual_len = buf[..len]
            .iter()
            .enumerate()
            .rev()
            .find(|(_, &byte)| byte & 0b0_111_111 != 0)
            .map(|(idx, _)| idx + 1)
            //     ^^^ index of last non-zero component
            .unwrap_or(1);

        if len != actual_len {
            buf[actual_len - 1] &= 0b0_111_1111;
            buf[actual_len..].fill(0);
        }

        Ok(VarInt {
            len: actual_len,
            inner: buf,
        })
    }
}

impl<W: AsyncWrite> VarIntAsyncWriteExt for W {}
#[async_trait]
pub trait VarIntAsyncWriteExt: AsyncWrite {
    async fn write_var_i32(&mut self, source: i32) -> io::Result<()>
    where
        Self: Unpin,
    {
        let mut source = source as u32;
        let mut buf = [0u8; 5];

        for i in 0..buf.len() {
            buf[i] = source as u8 & 0b0_111_1111;
            source >>= 7;
            if source != 0 {
                buf[i] |= 0b1_000_0000;
            } else {
                self.write(&buf[..=i]).await?;
            }
        }

        Ok(())
    }
}
