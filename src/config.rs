use clap::Parser;
use serde::Deserialize;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

/* e.g.
    listen_addr = "160.16.63.79:25565"

    [[rule]]
    match = "shiina.family"
    addr = "127.0.0.1:25566"

    [[rule]]
    match = "mc.chihuyu.love"
    addr = "127.0.0.1:25567"
*/
#[derive(Deserialize, Clone)]
pub(crate) struct Config {
    pub listen_addr: SocketAddr, // Address to listen; e.g. 160.16.63.79:25565
    #[serde(rename = "rule")]
    pub rules: Vec<Rule>,
}

#[derive(Deserialize, Clone)]
pub(crate) struct Rule {
    #[serde(rename = "match")]
    pub matcher: String,
    pub addr: SocketAddr,
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
