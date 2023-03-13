use ignore::ignore;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

use once_cell::sync::Lazy;
use tokio::net::{TcpListener, TcpStream};
use tokio::fs;

use miette::{Context, IntoDiagnostic, Result};
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tracing_subscriber::prelude::*;

use crate::config::{Args, Config};
use crate::mc::RawPacket;
use clap::Parser;

use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::TokioAsyncResolver;
use unsigned_varint::aio as varint;

mod config;
mod mc;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(console_subscriber::spawn()) // for tokio-console
        .with(
            tracing_subscriber::fmt::layer() // for stdout logging
                .compact()
                .with_filter(
                    EnvFilter::builder()
                        .with_default_directive(
                            match cfg!(debug_assertions) {
                                true => LevelFilter::DEBUG,
                                false => LevelFilter::INFO,
                            }
                            .into(),
                        )
                        .from_env_lossy(),
                ),
        )
        .init();

    let args = Args::parse();

    let raw_config = fs::read_to_string(&args.config)
        .await
        .into_diagnostic()
        .wrap_err_with(|| {
            format!(
                "Failed to read the config file from `{}`",
                &args.config.display()
            )
        })?;

    let config: Config = raw_config.parse().into_diagnostic()?;

    listen(config).await.map_err(|err| {
        tracing::error!("error: {}", err);
        std::process::exit(1)
    })
}

ignore! {
    enum KiteError {
        IoError(std::io::Error),
        ParseError(idk),
    }
}

async fn listen(config: Config) -> Result<(), Box<dyn Error>> {
    tracing::info!("started listening");

    let config = Arc::new(config);

    let listener = TcpListener::bind(SocketAddr::from(config.listen_addr.clone()))
        .await
        .unwrap();

    loop {
        match listener.accept().await {
            Ok((connection, _)) => {
                tokio::spawn(handle_connection(connection, Arc::clone(&config)));
            }
            Err(error) => {
                tracing::error!(%error, "failed to accept a connection");
                break;
            }
        }
    }

    Ok(())
}

static DNS_RESOLVER: Lazy<TokioAsyncResolver> = Lazy::new(|| {
    TokioAsyncResolver::tokio_from_system_conf()
        .or_else(|_| {
            TokioAsyncResolver::tokio(ResolverConfig::cloudflare(), ResolverOpts::default())
        })
        .unwrap()
});

// https://github.com/hiraginoyuki/yamp-draft

#[tracing::instrument(
    skip_all,
    fields(peer_addr = %client.peer_addr().unwrap()),
)]
async fn handle_connection(mut client: TcpStream, config: Arc<Config>) {
    ignore! {
        let mut header = 0u8;
        client.peek(slice::from_mut(&mut header));

        // https://wiki.vg/Protocol#Legacy_Server_List_Ping
        let packet = match header {
            0xfe => parse_legacy_packet(),
            _ => parse_normal_packet(),
        }
    }

    let packet = RawPacket::read_from(&mut client).await.unwrap();
    tracing::trace!(?packet);
    ignore! {
        io::Error(io::ErrorKind::InvalidData) => "packet parsing failure" {
            message: "invalid data received",
        }
        io::Error(_) => "unexpected eof, timeout, etc",
    }

    let mut cursor: &[u8] = packet.as_ref();

    let packet_id = varint::read_u32(&mut cursor).await.unwrap() as i32;
    let protocol_version = varint::read_u32(&mut cursor).await.unwrap() as i32;
    ignore!("https://github.com/dmonad/lib0/blob/main/decoding.js#L118");

    tracing::trace!(?packet_id, ?protocol_version);

    // TODO: into function?
    let hostname = mc::read_string(&mut cursor).await.unwrap(); //

    tracing::info!(hostname);

    let backend_rule = match config.rules.iter().find(|rule| rule.matcher == hostname) {
        Some(rule) => rule,
        None => {
            tracing::info!("no matching rule found; disconnecting");
            return;
        }
    };

    let query_result = DNS_RESOLVER
        .lookup_ip(&backend_rule.host)
        .await
        .unwrap_or_else(|_| todo!("handle when query failed"));

    let backend_addr = query_result
        .into_iter()
        .next()
        .unwrap_or_else(|| todo!("handle when no dns record is found"));

    let mut backend = TcpStream::connect((backend_addr, backend_rule.port))
    .await
    .unwrap_or_else(|_| todo!("failed to connect to the backend; log and quit"));

    tracing::info!("connected to backend");

    packet.write_to(&mut backend).await.unwrap();

    tracing::info!("written handshake packet, start proxying");

    match tokio::io::copy_bidirectional(&mut client, &mut backend).await {
        Ok((a_to_b, b_to_a)) => tracing::info!(a_to_b, b_to_a, "connection closed"),
        Err(error) => tracing::info!(%error, "proxy ended with an error"),
    };
}
