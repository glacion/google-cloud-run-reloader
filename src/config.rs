use anyhow::Result;
use clap::{Args, Parser};
use std::net::SocketAddr;

#[derive(Debug, Parser)]
#[command(name = env!("CARGO_PKG_NAME"), version = env!("CARGO_PKG_VERSION"))]
pub struct Config {
    #[command(flatten)]
    pub server: ServerConfig,
}

#[derive(Debug, Args)]
pub struct ServerConfig {
    #[arg(long = "host", default_value = "0.0.0.0", env = "HOST")]
    pub host: String,

    #[arg(long = "port", default_value_t = 8080, env = "PORT")]
    pub port: u16,
}

impl ServerConfig {
    pub fn address(&self) -> Result<SocketAddr> {
        Ok(format!("{}:{}", self.host, self.port).parse::<SocketAddr>()?)
    }
}
