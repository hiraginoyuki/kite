use {
    std::ops::RangeInclusive,
    tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWriteExt},
};

use ignore::ignore;

use tokio_util::compat::TokioAsyncReadCompatExt;
use unsigned_varint::{aio as varint, io::ReadError, encode};

ignore! {
    varivari::VarInt,
    varivari::io::{VarIntAsyncReadExt, VarIntAsyncWriteExt},
}

fn to_io_error(error: ReadError) -> io::Error {
    match error {
        ReadError::Io(err) => err,
        ReadError::Decode(err) => io::Error::new(io::ErrorKind::InvalidData, err),
        _ => unimplemented!(),
    }
}

// parse variable length string (https://wiki.vg/Protocol#Data_types)
pub async fn read_string<R>(reader: &mut R) -> io::Result<String>
where
    R: AsyncRead + TokioAsyncReadCompatExt + Send + Unpin,
{
    let str_len = varint::read_u32(reader.compat())
        .await
        .map_err(to_io_error)? as usize;

    tracing::trace!(str_len);

    let mut buf = vec![0; 255 * 4];
    reader.read_exact(&mut buf[..str_len]).await?; //

    let eof_idx = buf
        .iter()
        .enumerate()
        .find(|(_, byte)| **byte == 0)
        .map(|(idx, _)| idx)
        .unwrap_or(buf.len());

    buf.truncate(eof_idx);

    Ok(String::from_utf8(buf).map_err(|_| io::ErrorKind::InvalidData)?)
}

#[derive(Debug)]
pub struct RawPacket {
    inner: Vec<u8>,
}

impl RawPacket {
    pub const MAX_LEN: usize = 2_usize.pow(28) - 1;

    pub fn inner(&self) -> &Vec<u8> {
        &self.inner
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub async fn read_from<R>(reader: &mut R) -> io::Result<Self>
    where
        R: io::AsyncRead + TokioAsyncReadCompatExt + Unpin + Send,
    {
        const LEN_RANGE: RangeInclusive<i32> = 1..=RawPacket::MAX_LEN as i32;
        let len = varint::read_u32(reader.compat())
            .await
            .map_err(to_io_error)? as i32;
        if !LEN_RANGE.contains(&len) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let len = len as usize;

        let mut body = vec![0; len];
        reader.read_exact(&mut body).await?;

        Ok(Self { inner: body })
    }

    pub async fn write_to<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::AsyncWrite + Unpin + Send,
    {
        let mut buf = encode::u32_buffer();
        let len = encode::u32(self.len() as u32, &mut buf);

        writer.write_all(len).await?;
        writer.write_all(self.inner()).await?;

        Ok(())
    }
}

impl AsRef<[u8]> for RawPacket {
    fn as_ref(&self) -> &[u8] {
        self.inner.as_ref()
    }
}
