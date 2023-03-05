use clap::Parser;
use miette::{Context, IntoDiagnostic, Result};
use once_cell::sync::Lazy;
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::TokioAsyncResolver;
use varivari::{VarIntAsyncReadExt, VarIntAsyncWriteExt};

use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tracing_subscriber::prelude::*;

use core::str;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

mod config;
use config::{Args, Config};

/// for pseudo code and discussion
#[allow(unused_macros)]
macro_rules! ignore {
    ($($tt:tt)*) => {};
}

// https://docs.rs/miette

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(console_subscriber::spawn()) // for tokio-console
        .with(
            tracing_subscriber::fmt::layer() // for stdout logging
                .compact()
                .with_filter(
                    EnvFilter::builder()
                        .with_default_directive(LevelFilter::DEBUG.into())
                        .parse_lossy("kite=debug"),
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

async fn listen(config: Config) -> anyhow::Result<()> {
    tracing::info!("started listening");

    let config = Arc::new(config);

    let listener = TcpListener::bind(SocketAddr::from(config.listen_addr.clone()))
        .await
        .unwrap();

    while let Ok((connection, _)) = listener.accept().await {
        let config = config.clone();
        tokio::spawn(handle_connection(connection, config));
    }

    todo!()
}

static DNS_RESOLVER: Lazy<TokioAsyncResolver> = Lazy::new(|| {
    TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default()).unwrap()
});

#[tracing::instrument(
    skip_all,
    fields(
        peer_addr = %client.peer_addr().unwrap(),
    ),
)]
async fn handle_connection(mut client: TcpStream, config: Arc<Config>) {
    let packet_len_raw = client.read_varint().await.unwrap();
    tracing::trace!(?packet_len_raw);

    let packet_raw = {
        let packet_len: usize = i32::from(packet_len_raw.clone()).try_into().unwrap();
        tracing::trace!(%packet_len);

        let mut packet_raw = vec![0; packet_len];
        client.read_exact(&mut packet_raw).await.unwrap();

        packet_raw
    };

    let mut reader: &[u8] = packet_raw.as_ref();

    let packet_id: i32 = reader.read_varint().await.unwrap().into();
    let protocol_version: i32 = reader.read_varint().await.unwrap().into();
    tracing::trace!(packet_id, protocol_version);

    let mut buf;
    let hostname: &str = {
        // variable length string
        let str_len: i32 = reader.read_varint().await.unwrap().into();
        tracing::trace!(hostname_len = str_len);
        let str_len = usize::try_from(str_len).unwrap();

        buf = [0; 255 * 4];
        let buf = &mut buf[..str_len];
        reader.read_exact(buf).await.unwrap();

        let eof_idx = buf
            .iter()
            .enumerate()
            .find(|(_, byte)| **byte == 0)
            .map(|(idx, _)| idx)
            .unwrap_or(buf.len());

        str::from_utf8(&buf[..eof_idx])
    }
    .unwrap();

    tracing::debug!(hostname);

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

    backend.write_varint(&packet_len_raw).await.unwrap();
    backend.write_all(&packet_raw).await.unwrap();

    tracing::info!("written handshake packet, start proxying");

    match tokio::io::copy_bidirectional(&mut client, &mut backend).await {
        Ok((a_to_b, b_to_a)) => tracing::info!(a_to_b, b_to_a, "connection closed"),
        Err(error) => tracing::info!(%error, "proxy ended with an error"),
    };
}
