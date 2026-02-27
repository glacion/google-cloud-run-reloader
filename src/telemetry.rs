use anyhow::Result;
use tracing_subscriber::{EnvFilter, fmt::Layer, prelude::*, registry};

pub struct Telemetry;

impl Telemetry {
    pub async fn init() -> Result<Self> {
        let layer = Layer::new()
            .json()
            .flatten_event(true)
            .with_ansi(false)
            .with_target(false);

        registry()
            .with(EnvFilter::from_default_env())
            .with(layer)
            .init();
        Ok(Self)
    }
}
