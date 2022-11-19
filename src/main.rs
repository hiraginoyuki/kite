#![feature(absolute_path, io_error_more)]

use clap::{error::ErrorKind, CommandFactory};
use log::{debug, error, info, trace, warn};

pub mod config;
use config::ARGS;

use crate::config::{Cli, Config};
use async_channel::bounded as mpmc_bounded;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode::NonRecursive};
use std::{net::SocketAddr, process, sync::Arc, time::Duration};
use tokio::{
    fs,
    io::ErrorKind::*,
    net::{TcpListener, TcpStream},
    select,
};

const DEBOUNCE_TIMEOUT: Duration = Duration::from_millis(50);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    better_panic::install();
    env_logger::builder()
        .filter_level(ARGS.verbose.log_level_filter())
        .init();

    let (_debouncer, rx) = {
        let (tx, rx) = async_channel::unbounded();

        let mut debouncer = new_debouncer(DEBOUNCE_TIMEOUT, None, move |maybe_events| {
            match maybe_events {
                Ok(events) => {
                    // Err: As defined in main(), rx will never be closed unless .close() is manually called.
                    tx.send_blocking(events).expect("rx closed unexpectedly");
                }
                Err(errors) => warn!("{:?}", errors),
            };
        })
        .unwrap();

        let parent = ARGS
            .config_path
            .parent()
            // None: Occurs when user specifies a path that doesn't have parent
            // TODO: Print a user-friendly text and exit
            .unwrap_or_else(|| {
                Cli::command()
                    .error(ErrorKind::ValueValidation, "CONFIG doesn't have a parent")
                    .exit()
            });

        trace!("config_path.parent() = {parent:?}");

        debouncer
            .watcher()
            .watch(parent, NonRecursive)
            // Err: Occurs when the directory is somehow not accessible
            // See also: https://man7.org/linux/man-pages/man2/inotify_add_watch.2.html#ERRORS
            .unwrap_or_else(|_| {
                Cli::command()
                    .error(
                        ErrorKind::ValueValidation,
                        "could not watch parent directory",
                    )
                    .exit()
            });

        (debouncer, rx)
    };

    loop {
        macro_rules! config_modification {
            () => {
                async {
                    while let Ok(events) = rx.recv().await {
                        trace!("{:?}, {:?}", events, ARGS.config_path);
                        if events.iter().any(|event| *event.path == *ARGS.config_path) {
                            break;
                        }
                    }
                }
            };
        }

        macro_rules! reload {
            () => {{
                config_modification!().await;
                info!("change detected; reloading");
                continue;
            }};
        }

        let config: Arc<Config> = match fs::read_to_string(&ARGS.config_path).await {
            Ok(str) => match str.parse() {
                Ok(config) => Arc::new(config),
                Err(e) => {
                    error!("parse: {e}");
                    reload!();
                }
            },
            Err(e) => match e.kind() {
                NotFound => {
                    error!("not found; waiting");
                    reload!();
                }
                IsADirectory => {
                    error!("specified path is a directory");
                    reload!();
                }
                _ => {
                    error!("unexpected IO error while reading config file: {e:?}");
                    process::exit(1);
                }
            },
        };

        // let watcher = args.config_path.watch(|_| true);
        let listener = TcpListener::bind(&config.bind_addr).await.unwrap();
        let (tx, rx) = mpmc_bounded(1);

        debug!("{config:?}");

        loop {
            select! {
                () = config_modification!() => {
                    info!("change detected; reloading");
                    tokio::spawn(async move {
                        let _ = tx.send(HandlerMsg::RequestStop).await;
                    });
                    break;
                }
                maybe_conn = listener.accept() => match maybe_conn {
                    Ok((conn, peer)) => {
                        info!("conn from {peer:?}");
                        tokio::spawn(handle_connection(Arc::clone(&config), conn, peer, rx.clone()));
                    },
                    Err(e) => {
                        error!("accept: {e}");
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
    config: Arc<Config>,
    mut client: TcpStream,
    peer: SocketAddr,
    msg_channel: async_channel::Receiver<HandlerMsg>,
) {
    // use ozelot::{ClientState, Server};
    // let mut o = Server::from_tcpstream(client.into_std().unwrap()).unwrap();
    // o.set_clientstate(ClientState::Status);

    // let ps = o.read().unwrap();
    // trace!("{:?}", ps);

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
