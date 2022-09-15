use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, Error, ErrorKind, Result};

#[async_trait]
impl<R: AsyncRead> VarIntAsyncReadExt for R {}
#[async_trait]
pub trait VarIntAsyncReadExt: AsyncRead {
    async fn read_var_i32(&mut self) -> Result<i32>
    where
        Self: Unpin,
    {
        let mut result = 0;
        let mut buf = [0];
        let mut len = 0;
        for i in 0..5 {
            self.read_exact(&mut buf).await?;
            result |= (buf[i] as i32 & 0b0_111_1111) << (7 * i);
            if buf[i] & 0b1_000_0000 == 0 {
                len = i + 1;
                break;
            }
        }
        // vvvvvvvv len was never changed from 0, meaning the varint is longer than it can be
        if len == 0 {
            return Err(Error::new(ErrorKind::InvalidData, "varint too long"));
        }
        Ok(result)
    }
}

#[async_trait]
impl<W: AsyncWrite> VarIntAsyncWriteExt for W {}
#[async_trait]
pub trait VarIntAsyncWriteExt: AsyncWrite {
    async fn write_var_i32(&mut self, source: i32) -> Result<()>
    where
        Self: Unpin,
    {
        let mut source = source as u32;
        loop {
            let byte = source as u8 & 0b0_111_1111;
            source <<= 7;
            if source != 0 {
                self.write_u8(byte | 0b1_000_0000).await?;
                break;
            } else {
                self.write_u8(byte).await?;
            }
        }
        Ok(())
    }
}
