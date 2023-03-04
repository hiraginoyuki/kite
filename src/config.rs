use clap::Parser;
use serde::Deserialize;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;

/* e.g.
    [listen]
    host = "0.0.0.0" # default
    port = 25565 # default

    [[rule]]
    match = "vanilla.shiina.family"
    host = "127.0.0.1"
    port = 25566

    [[rule]]
    match = "forge.shiina.family"
    host = "160.16.63.79"
    port = 25567

    [[rule]]
    match = "hypixel.shiina.family"
    host = "mc.hypixel.net"

    [[rule]]
    match = "test.shiina.family"
    host = "backend1"
*/
#[derive(Deserialize, Debug)]
pub(crate) struct Config {
    #[serde(rename = "listen")]
    pub listen_addr: ListenAddr, // Address to listen; e.g. 160.16.63.79:25565
    #[serde(rename = "rule")]
    pub rules: Vec<Rule>,
}

const fn default_host() -> IpAddr {
    IpAddr::V4(Ipv4Addr::UNSPECIFIED)
}
const fn default_port() -> u16 {
    25565
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct ListenAddr {
    #[serde(default = "default_host")]
    pub host: IpAddr,
    #[serde(default = "default_port")]
    pub port: u16,
}

impl From<ListenAddr> for SocketAddr {
    fn from(ListenAddr { host, port }: ListenAddr) -> Self {
        SocketAddr::new(host, port)
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct Rule {
    #[serde(rename = "match")]
    pub matcher: String,
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

impl FromStr for Config {
    type Err = toml::de::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        toml::from_str(s)
    }
}

// $ kite
// $ kite --config ./kite.toml
// $ kite -c ./kite.toml
#[derive(Parser)]
pub(crate) struct Args {
    #[clap(short, long, default_value_os_t = PathBuf::from("./kite.toml"))] // TODO
    pub config: PathBuf,
}
