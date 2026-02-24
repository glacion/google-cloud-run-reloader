use crate::config::OpenTelemetryConfig;
use crate::telemetry::detector::google::GoogleCloudDetector;
use anyhow::{Context, Result};
use opentelemetry::trace::TracerProvider;
use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::{MetricExporter, SpanExporter};
use opentelemetry_resource_detectors::{
    HostResourceDetector, OsResourceDetector, ProcessResourceDetector,
};
use opentelemetry_sdk::{
    Resource,
    metrics::{PeriodicReader, SdkMeterProvider},
    trace::SdkTracerProvider,
};
use tracing_subscriber::{EnvFilter, fmt::Layer, prelude::*, registry};

mod detector;

pub struct Telemetry {
    tracer: SdkTracerProvider,
    meter: SdkMeterProvider,
}

impl Telemetry {
    pub async fn init(config: &OpenTelemetryConfig) -> Result<Self> {
        let resource = Self::resource(config).await?;

        let tracer = Self::tracer(resource.clone())?;
        let meter = Self::meter(resource.clone())?;
        global::set_tracer_provider(tracer.clone());
        global::set_meter_provider(meter.clone());

        Self::subscriber(tracer.clone(), &config.name);
        Ok(Self { tracer, meter })
    }

    pub fn shutdown(&self) -> Result<()> {
        self.meter.shutdown().context("failed to shutdown meter")?;
        self.tracer
            .shutdown()
            .context("failed to shutdown tracer")?;
        Ok(())
    }

    async fn resource(config: &OpenTelemetryConfig) -> Result<Resource> {
        Ok(Resource::builder()
            .with_attribute(KeyValue::new("service.version", config.version.to_owned()))
            .with_service_name(config.name.to_owned())
            .with_detectors(&[
                Box::new(GoogleCloudDetector::default().await?),
                Box::new(HostResourceDetector::default()),
                Box::new(OsResourceDetector),
                Box::new(ProcessResourceDetector),
            ])
            .build())
    }

    fn tracer(resource: Resource) -> Result<SdkTracerProvider> {
        let exporter = SpanExporter::builder()
            .with_tonic()
            .build()
            .context("failed to build OTLP trace exporter")?;

        Ok(SdkTracerProvider::builder()
            .with_resource(resource)
            .with_batch_exporter(exporter)
            .build())
    }

    fn meter(resource: Resource) -> Result<SdkMeterProvider> {
        let exporter = MetricExporter::builder()
            .with_tonic()
            .build()
            .context("failed to build OTLP metric exporter")?;
        let reader = PeriodicReader::builder(exporter).build();
        Ok(SdkMeterProvider::builder()
            .with_resource(resource)
            .with_reader(reader)
            .build())
    }

    fn subscriber(provider: SdkTracerProvider, service_name: &str) {
        let tracer = provider.tracer(service_name.to_owned());

        let layer = Layer::new()
            .json()
            .flatten_event(true)
            .with_ansi(false)
            .with_target(false);

        registry()
            .with(EnvFilter::from_default_env())
            .with(tracing_opentelemetry::layer().with_tracer(tracer))
            .with(layer)
            .init();
    }
}
