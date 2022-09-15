use std::convert::TryInto;

use async_trait::async_trait;
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::{
    mc::var_integer::VarIntAsyncWriteExt,
    traits::convert::{AsInner, IntoInner, TryFromInner},
};

use super::var_integer::VarIntAsyncReadExt;

pub struct McPacketRaw {
    inner: Vec<u8>,
}
impl McPacketRaw {
    pub const MAX_LEN: usize = 0b111_1111_111_1111_111_1111;
    fn len(&self) -> usize {
        self.inner.len()
    }
}
impl AsInner for McPacketRaw {
    type Inner = Vec<u8>;

    fn as_inner(&self) -> &Self::Inner {
        &self.inner
    }
}
impl IntoInner for McPacketRaw {
    type Inner = Vec<u8>;

    fn into_inner(self) -> Self::Inner {
        self.inner
    }
}

impl TryFromInner for McPacketRaw {
    type Inner = Vec<u8>;
    type Error = &'static str;

    fn try_from_inner(inner: Self::Inner) -> Result<Self, Self::Error> {
        if inner.len() < i32::MAX as u32 as usize {
            Ok(Self { inner })
        } else {
            Err("ok i hate you")
        }
    }
}

#[async_trait]
impl<R: AsyncRead> McPacketAsyncRead for R {}
#[async_trait]
pub trait McPacketAsyncRead: AsyncRead {
    async fn read_mc_packet(&mut self) -> io::Result<McPacketRaw>
    where
        Self: Unpin,
    {
        let size: usize = self.read_var_i32().await?.try_into().unwrap();
        let mut buf = vec![0u8; size];
        self.read_exact(&mut buf).await?;
        Ok(McPacketRaw { inner: buf })
    }
}

#[async_trait]
impl<W> McPacketAsyncWrite for W where W: AsyncWrite {}
#[async_trait]
pub trait McPacketAsyncWrite: AsyncWrite {
    async fn write_mc_packet(&mut self, packet: McPacketRaw) -> io::Result<()>
    where
        Self: Unpin,
    {
        self.write_var_i32(packet.len().try_into().unwrap()).await?;
        self.write(packet.as_inner()).await?;
        Ok(())
    }
}
