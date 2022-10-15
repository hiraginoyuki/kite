use anyhow::Context;
use async_channel::bounded as mpmc_bounded;
use clap::Parser;
use kite::config::{Args, Config};
use notify::RecommendedWatcher;
use once_cell::sync::OnceCell;
use std::{net::SocketAddr, path::Path, process, sync::Arc};
use tokio::{
    fs,
    io::ErrorKind::*,
    net::{TcpListener, TcpStream},
    select,
};

mod watch;
use watch::WatchManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    static ARGS: OnceCell<Args> = OnceCell::new();
    let args: &'static Args = ARGS.get_or_init(|| Args::parse());
    let mut watch_manager = WatchManager::<RecommendedWatcher>::new()
        .context("Failed to create watcher (possibly due to platform-specific behavior)")?;

    watch_manager.watch(&args.config_path);

    loop {
        macro_rules! reload {
            () => {{
                eprintln!("[conf] change detected; reloading");
                continue;
            }};
        }

        let config: Arc<Config> = match fs::read_to_string(&args.config_path).await {
            Ok(str) => match str.parse() {
                Ok(config) => Arc::new(config),
                Err(e) => {
                    eprintln!("{e}");
                    reload!();
                }
            },
            Err(e) if matches!(e.kind(), NotFound,) => {
                reload!();
            }
            Err(_) => todo!("to future me: please handle unexpected io errors :sob::sob::sob:"),
        };

        // let watcher = args.config_path.watch(|_| true);
        let listener = TcpListener::bind(&config.bind_addr).await.unwrap();
        let (tx, rx) = mpmc_bounded(1);

        loop {
            select! {
                // _ = eye.recv() => {
                //     println!("[conf] change detected; reloading");
                //     tokio::spawn(async move { tx.send(HandlerMsg::RequestStop).await; });
                //     break;
                // }
                maybe_conn = listener.accept() => match maybe_conn {
                    Ok((conn, peer)) => {
                        tokio::spawn(handle_connection(args, Arc::clone(&config), conn, peer, rx.clone()));
                    },
                    Err(e) => {
                        eprintln!("[warn] fafafa - {e}");
                        continue;
                    }
                }
            }
        }
    }
}

enum HandlerMsg {
    RequestStop,
}

#[allow(unused_variables, unused_mut)] // TODO: remove later
async fn handle_connection(
    args: &'static Args,
    config: Arc<Config>,
    mut client: TcpStream,
    peer: SocketAddr,
    msg_channel: async_channel::Receiver<HandlerMsg>,
) {
    // let packet = client.read_mc_packet()?;
    // let server = config.get(&packet);
    // let server = TcpStream::connect(server)?;
    // let proxy = tokio::copy_bidirectional(&mut client, &mut server);

    // server.write(packet).await?;

    // select! {
    //         RequestStop => return
    //     }
    //     _ = proxy => return
    // }
}
