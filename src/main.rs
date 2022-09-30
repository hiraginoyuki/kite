use kite::traits::convert::AsInner;

use clap::Parser;
use kite::{
    config::{Args, Config},
    mc::packet::{McPacketAsyncRead, McPacketRaw},
};
use std::{
    io::{self, Cursor},
    net::SocketAddr,
    sync::Arc,
};
use tokio::{
    fs,
    io::{copy_bidirectional, ErrorKind},
    net::{TcpListener, TcpStream},
    select,
};

#[tokio::main]
async fn main() {
    let args = Args::parse();
    eprintln!("[init] args: {:?}", args);
    eprintln!(""); // I think I should let the language decide what newline character to be printed

    // let config: Arc<Config> = Arc::new(
    //     r#"
    //     bind_addr = "0.0.0.0:25555"
    //
    //     [[rules]]
    //     host = "127.1"
    //     backend = "127.0.0.1:25565"
    //
    //     [[rules]]
    //     host = "127.0.0.1"
    //     backend = "127.0.0.1:25566"
    // "#
    //     .parse()
    //     .unwrap(),
    // );

    // eprintln!("[init] config: {:?}", config);
    // eprintln!("[init] bound: {} -> (kite)", config.bind_addr);
    // eprintln!("");

    let mut config: Arc<Config>;

    loop {
        config = Arc::new({
            let config = fs::read_to_string(&args.config_path).unwrap(); // print what to fix and wait for next io notification
            let config = config.parse().unwrap(); // also do that when there is a syntax error
            config
        })
    }

    // loop {
    //     let listener = match TcpListener::bind(config.bind_addr).await {
    //         Ok(listener) => listener,
    //     };
    //
    //     loop {
    //         select! {
    //             connection = listener.accept() => match connection {
    //                 Ok((client, peer)) => tokio::spawn(handle(config.clone(), client, peer)),
    //                 Err(e) => println!("[conn] error accepting - {:?}", e)
    //             }
    //         }
    //     }
    // }
}

async fn handle(config: Arc<Config>, mut client: TcpStream, peer: SocketAddr) {
    let rule = &config.rules[0];

    let mut server = TcpStream::connect(rule.backend).await.unwrap();

    eprintln!("[conn] {} -> (kite) -> {}", peer, rule.backend);

    let packet = client.read_mc_packet().await.unwrap();
    let mut cur = Cursor::new(packet.as_inner());

    if let Err(e) = copy_bidirectional(&mut client, &mut server).await {
        eprintln!("[warn] while proxying (tokio::io::copy_bidirectional) - {e}");
    };
}
