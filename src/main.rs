use anyhow::Context;
use clap::Parser;
use once_cell::sync::Lazy;
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::TokioAsyncResolver;
use varivari::{VarIntAsyncReadExt, VarIntAsyncWriteExt};

use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

mod config;
use config::{Args, Config};

/// for pseudo code and discussion
macro_rules! ignore {
    ($($tt:tt)*) => {};
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let config: Config = fs::read_to_string(args.config)
        .await
        .unwrap()
        .parse()
        .unwrap();

    if let Err(err) = listen(config).await {
        eprintln!("error: {}", err);
        std::process::exit(1);
    };
}

async fn listen(config: Config) -> anyhow::Result<()> {
    let config = Arc::new(config);
    let listener = TcpListener::bind(SocketAddr::from(config.listen_addr.clone()))
        .await
        .context("failed to bind to listen address")?;

    while let Ok((connection, _)) = listener.accept().await {
        let config = config.clone();
        tokio::spawn(handle_connection(connection, config));
    }

    todo!()
}

ignore![
    0_000_1001, // packet_len = 9 ( VarInt::len() = 1 ) 確実にもっと長い; 後続する他のフィールドを無視してるから
    0_000_0000, // packet_id = 0 ( VarInt::len() = 1 )
    1_110_1111, 0_000_0101, // protocol_version = 751 = 0b1011101111 ( VarInt::len() = 2 )
    0_000_0005, 'h', 'e', 'l', 'l', 'o', // server address
    0110_0011, 1101_1101, // port (normal u16), 25565
];

static RESOLVER: Lazy<TokioAsyncResolver> = Lazy::new(|| {
    TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default()).unwrap()
});

async fn handle_connection(mut client: TcpStream, config: Arc<Config>) {
    let packet_len = client.read_varint().await.unwrap();

    let packet_raw = {
        let packet_len: usize = i32::from(packet_len.clone()).try_into().unwrap();
        dbg!(packet_len);
        let mut packet_raw = vec![0; packet_len];
        client.read_exact(&mut packet_raw).await.unwrap();

        dbg!(packet_raw)
    };

    let mut reader = Cursor::new(&packet_raw);

    let packet_id: i32 = reader.read_varint().await.unwrap().into();
    let protocol_version: i32 = reader.read_varint().await.unwrap().into();
    dbg!(packet_id);
    dbg!(protocol_version);

    let hostname: String = {
        // variable length string
        let str_len: i32 = reader.read_varint().await.unwrap().into();
        dbg!(str_len);
        let str_len = dbg!(usize::try_from(str_len).unwrap());

        let mut str_body = vec![0; str_len];
        reader.read_exact(&mut str_body).await.unwrap();

        String::from_utf8_lossy(&str_body).into()
    };

    dbg!(&hostname);

    let backend_rule = config
        .rules
        .iter()
        .find(|rule| rule.matcher == hostname)
        .unwrap();

    let address = RESOLVER
        .lookup_ip(&backend_rule.host)
        .await
        .unwrap()
        .iter()
        .next()
        .unwrap();

    let mut backend = TcpStream::connect((address, backend_rule.port))
        .await
        .unwrap();

    backend.write_varint(&packet_len).await.unwrap();
    backend.write_all(&packet_raw).await.unwrap();

    tokio::io::copy_bidirectional(&mut client, &mut backend)
        .await
        .unwrap();

    ignore! {
        client.set_nonblocking(true).unwrap();
        backend.set_nonblocking(true).unwrap();

        let mut buf = [0u8; 1048576];
        loop {
            let len = match client.read(&mut buf) {
                Ok(len) => len,
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => 0,
                err => err.unwrap(),
            };
            backend.write_all(&buf[..len]).unwrap();
            if len != 0 {
                println!("{len} bytes sent to backend");
            }

            let len = match backend.read(&mut buf) {
                Ok(len) => len,
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => 0,
                err => err.unwrap(),
            };
            client.write_all(&buf[..len]).unwrap();
            if len != 0 {
                println!("{len} bytes sent to client");
            }
        }
    }
}

// https://docs.rs/nom
// https://github.com/Geal/nom/tree/main/doc

ignore! {
    {
        fn read_varint() {}
        let a = read_varint(&mut stream);
    }

    struct RawPacket {
        len: usize,
        id: i32,
        body: Vec<u8>,
    }

    impl RawPacket {
        fn read_packet(stream: &mut impl io::Read) -> io::Result<Packet> { // TODO: other result?
            let len = stream.read_varint() as usize?;
            let id = stream.read_varint() as i32;

            let mut body = vec![0; len];
            stream::read_exact(&mut packet_body)?; // TODO: what io errors happen?

            Packet { len, id, body }
        }
    }

    "https://docs.rs/ozelot"
    struct HandshakePacket {
    }

    // no
    fn read_hello(stream: &mut impl io::Read) {
        let packet_len = stream.read_varint();
        let packet_id = stream.read_varint();
        let prtocol_version = stream.read_varint();
        let hostname_with_garbage = stream.read_str();

        let hostname = hostname.(take until 0x00 which is where hostname ends);
        let hostname = String::from_utf8(&hostname):
    }

    "https://docs.rs/mc-varint/latest/src/mc_varint/lib.rs.html#103-122"
    fn on_connection(&mut conn) {
        let packet = (read first packet);
    }

    "https://github.com/hiraginoyuki/varivari"
    "https://wiki.vg/Protocol#Handshake"
    ^ handshake packet has hostname field. Forge appends something like b"\0FML1.1" which is annoying but you can just take_until b'\0'

    async fn handle_connection(mut client: TcpStream) {
        let handshake_packet = client.read_packet().await.unwrap();
        let hostname = handshake_packet.(get hostname in the packet);

        let backend = backends.(get matching backend for #hostname);

        let backend: TcpStream = TcpStream::connect(backend).unwrap();
        // (kite <-> backend)

        backend.write(handshake_packet).await.unwrap();

        tokio::copy_bidirectional(&mut client, &mut server);

        "https://docs.rs/tokio/latest/tokio/io/fn.copy_bidirectional.html";
        async fn tokio::copy_bidirectional(
            &mut stream1: impl io::Read + io::Write,
            &mut stream2: impl io::Read + io::Write,
        ) i.e. {
            let mut buf;
            loop {
                stream1.read(&mut buf).await.unwrap();
                stream2.write(&buf).await.unwrap();

                stream2.read(&mut buf).await.unwrap();
                stream1.write(&buf).await.unwrap();
            }
        }
    }
}
