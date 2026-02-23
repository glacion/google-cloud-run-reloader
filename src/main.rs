mod middleware;
mod model;
mod route;
mod service;

use anyhow::{Context, Result};
use axum::{middleware::from_fn, serve};
use service::cloud_run::CloudRunService;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt::Layer, prelude::*, registry};

#[tokio::main]
async fn main() -> Result<()> {
    registry()
        .with(EnvFilter::from_default_env())
        .with(
            Layer::new()
                .json()
                .flatten_event(true)
                .with_target(false)
                .with_ansi(false),
        )
        .init();

    info!("connecting to cloud run");
    let cloud_run = CloudRunService::connect().await?;

    let router = route::router(cloud_run).layer(from_fn(middleware::log::log_requests));

    let address = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = TcpListener::bind(address)
        .await
        .context("Failed to bind TCP listener")?;

    info!("starting server");
    serve(listener, router).await.context("server error")?;

    Ok(())
}
