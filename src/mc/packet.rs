use std::io::ErrorKind;

use super::var_integer::{VarInt, VarIntAsyncReadExt};
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::utils::ranging::{RangeCmp, Ranging::*};
use async_trait::async_trait;

pub struct McPacketRaw {
    inner: Vec<u8>,
    idx_id: usize,
    idx_data: usize,
}
impl McPacketRaw {
    pub const MAX_LEN: usize = !(!0 << 28); // 28 bits
}

#[async_trait]
pub trait McPacketAsyncReadExt: AsyncRead {
    async fn read_mc_packet(&mut self) -> io::Result<McPacketRaw>
    where
        Self: Unpin,
    {
        let packet_len = match self.read_var_i32().await {
            Ok(len) => match (0..=McPacketRaw::MAX_LEN as i32).ranging(&(&len).into()) {
                Contained => Ok(len),
                LessThanStart | GreaterThanEnd => {
                    return Err(io::Error::new(ErrorKind::InvalidData, "noo"))
                }
            },
            Err(e) if e.kind() == ErrorKind::InvalidData => Err(io::Error::new(
                ErrorKind::InvalidData,
                "packet length not detected",
            )),
            Err(e) => Err(e),
        }?;

        let mut buf = &mut vec![0u8; i32::from(&packet_len) as usize];
        self.read_exact(&mut buf[packet_len..]).await?;

        let mut cur = Cursor::new();

        Ok(McPacketRaw {
            inner: buf,
            idx_id: packet_len,
        })
    }
}
impl<R: AsyncRead> McPacketAsyncReadExt for R {}

#[async_trait]
pub trait McPacketAsyncWriteExt: AsyncWrite {
    async fn write_mc_packet(&mut self, packet: &McPacketRaw) -> io::Result<()>
    where
        Self: Unpin,
    {
        self.write(&packet.inner).await?;
        Ok(())
    }
}
impl<W: AsyncWrite> McPacketAsyncWriteExt for W {}

// #[derive(Debug)]
// pub enum DataError {
//     InvalidLength,
// }
// #[derive(Debug)]
// pub enum Error {
//     IoError(tokio::io::Error),
//     InvalidData(&'static str),
// }
// use Error::*;

// pub struct McPacketRaw {
//     id: i32,
//     inner: Vec<u8>,
// }
// impl McPacketRaw {
//     pub const MAX_LEN: i32 = 0b111_1111__111_1111__111_1111;
//     fn len(&self) -> usize {
//         self.inner.len()
//     }
// }

// impl TryFromInner for McPacketRaw {
//     type Inner = Vec<u8>;
//     type Error = Error;

//     fn try_from_inner(inner: Self::Inner) -> Result<Self, Self::Error> {
//         let length = match parse_var_i32(&inner[..5]) {
//             Ok(len @ 1..=McPacketRaw::MAX_LEN) => len,
//             Ok(_) => return Err(InvalidData("packet length is out of range")), // out of range
//             Err(_) => return Err(InvalidData("")),                             // invalid varint
//         } as usize;

//         Ok(Self { inner, id })
//     }
// }

// impl AsInner for McPacketRaw {
//     type Inner = Vec<u8>;
//     fn as_inner(&self) -> &Self::Inner {
//         &self.inner
//     }
// }
// impl IntoInner for McPacketRaw {
//     type Inner = Vec<u8>;
//     fn into_inner(self) -> Self::Inner {
//         self.inner
//     }
// }
