use std::{net::SocketAddr, path::PathBuf, str::FromStr};

use atty::Stream;
use clap::Parser;
use serde::Deserialize;

#[derive(Debug, Parser)]
#[clap(version, about)]
pub struct Args {
    #[clap(
        short='c', long="config",
        value_parser, value_name = "FILE",
        default_value_os_t = ("./config.toml").into()
    )]
    pub config_path: PathBuf,

    /// Enable colored outputs even if stdout is not a tty
    #[clap(
        short='f', long="fancy",
        value_parser,
        default_value_t = atty::is(Stream::Stdout)
    )]
    pub fancy: bool,
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

impl FromStr for Config {
    type Err = toml::de::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        toml::de::from_str(s)
    }
}
