//! Utilities related to Minecraft.

pub mod var_integer;

use async_trait::async_trait;
use tokio::io::{self, AsyncRead, AsyncWrite};

use self::var_integer::{VarInt, AsyncReadVarIntExt};

pub struct McPacketRaw {
    size: usize,
    data: Vec<u8>,
}

#[async_trait]
pub trait McPacketAsyncRead: AsyncRead {
    async fn read_mc_packet(&mut self) -> io::Result<McPacketRaw>
    where
        Self: Unpin
    {
        let size = match self.read_var_int().await {
            Ok(size) => size,
            _ => todo!(),
        };
        let size: usize = u32::try_from(&size).unwrap().try_into().unwrap();
        let mut packet = McPacketRaw { size, data: Vec::with_capacity(size) };
        todo!()
    }
}
#[async_trait]
impl<R> McPacketAsyncRead for R where R: AsyncRead { }

// 
// #[async_trait]
// pub trait McPacketAsyncWrite: AsyncWrite {
//     async fn write_mc_packet(&mut self, packet: McPacketRaw) -> io::Result<()>
//     where
//         Self: Unpin
//     {
//         todo!()
//     }
// }
// #[async_trait]
// impl<W> McPacketAsyncWrite for W where W: AsyncWrite {}
