use clap::Parser;
use kite::config::{Args, Config};
use std::{io, net::SocketAddr, sync::Arc};
use tokio::{
    io::copy_bidirectional,
    net::{TcpListener, TcpStream},
};

#[tokio::main]
async fn main() {
    let args = Args::parse();
    // let config = Config::parse(&args.config_path).unwrap_or_else(|e| match e {
    //     Error::TomlError(e) => println!("{e:?}"),
    //     _ => unimplemented!()
    // });
    let config: Config = r#"
        bind_addr = "0.0.0.0:25555"

        [[rules]]
        host = "127.1"
        backend = "127.0.0.1:25565"

        [[rules]]
        host = "127.0.0.1"
        backend = "127.0.0.1:25566"
    "#
    .parse()
    .unwrap();

    println!("args: {args:?}");
    println!("config: {config:?}");
}

#[tokio::main]
async fn _main() -> io::Result<()> {
    let bind_addr: Arc<SocketAddr> = Arc::new("0.0.0.0:25555".parse().unwrap());
    let backend_addr: Arc<SocketAddr> = Arc::new("127.0.0.1:25565".parse().unwrap());

    eprintln!("[bind] {bind_addr} -> (kite)");
    let listener = TcpListener::bind(bind_addr.as_ref()).await?;

    loop {
        let (mut client, peer) = listener.accept().await?;
        let mut server = TcpStream::connect(backend_addr.as_ref()).await.unwrap();
        eprintln!("[conn] {peer} -> (kite) -> {backend_addr}");

        tokio::spawn(async move {
            if let Err(e) = copy_bidirectional(&mut client, &mut server).await {
                eprintln!("[info] while proxying :: {e}");
            };
        });
    }
}
