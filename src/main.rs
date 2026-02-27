mod config;
mod middleware;
mod model;
mod route;
mod service;
mod telemetry;

use anyhow::Result;
use axum::serve;
use clap::Parser;
use config::Config;
use service::cloud_run::CloudRunService;
use telemetry::Telemetry;
use tokio::net::TcpListener;

use route::Router;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::parse();
    let telemetry = Telemetry::init().await?;
    let cloud_run = CloudRunService::init().await?;

    let router = Router::init(cloud_run);

    let listener = TcpListener::bind(config.server.address()?).await?;
    serve(listener, router.axum).await?;

    Ok(())
}
