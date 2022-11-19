use arc_swap::ArcSwap;
use atty::Stream;
use clap::{
    builder::{PathBufValueParser, TypedValueParser},
    Parser,
};
use serde::Deserialize;
use std::{io, net::SocketAddr};
use std::{path::PathBuf, str::FromStr};

use clap_verbosity_flag::Verbosity;

use once_cell::sync::Lazy;
pub static ARGS: Lazy<Cli> = Lazy::new(Cli::parse);

#[derive(Debug, Parser)]
#[clap(version)]
pub struct Cli {
    #[command(flatten)]
    pub verbose: Verbosity,

    /// Path to the configuration file
    #[clap(
        short='c', long="config", value_name="FILE",
        // Err: clap doesn't allow empty string which leaves us with
        // only one way to Err in which std::env::current_dir() fails.
        value_parser = PathBufValueParser::new().try_map(std::path::absolute),
        default_value_os_t = ("./config.toml").into()
    )]
    pub config_path: PathBuf,

    /// Enable colored output (defaults to whether stdin is a tty)
    #[clap(
        short='f', long="fancy",
        value_parser,
        default_value_t = atty::is(Stream::Stdout)
    )]
    pub fancy: bool,
}

impl FromStr for Config {
    type Err = toml::de::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        toml::de::from_str(s)
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub rules: Vec<Rule>,
    pub fallback: Option<Rule>,
}

#[derive(Debug, Deserialize)]
pub struct Rule {
    pub host: String,
    pub backend: SocketAddr,
}

enum ConfigState<C: FromStr> {
    Loaded(C),
    IoError(io::Error),
    ParseError(C::Err),
}

struct ConfigLoader<C: FromStr> {
    inner: ArcSwap<ConfigState<C>>,
}
