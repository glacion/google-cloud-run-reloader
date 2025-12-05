mod entry;
mod error;

use anyhow::{Context, Result};
use axum::{Extension, Router, http::StatusCode, routing::post, serve};
use entry::Entry;
use error::Error;
use futures::future::join_all;
use google_cloud_run_v2::{client::Services as CloudRun, model::Service};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::error;
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
    let run = CloudRun::builder()
        .build()
        .await
        .context("failed to connect to cloud run v2 services")?;

    let router = Router::new()
        .route("/", post(handler))
        .layer(Extension(run));

    let address = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = TcpListener::bind(address)
        .await
        .context("Failed to bind TCP listener")?;

    info!("starting server");
    serve(listener, router).await.context("server error")?;

    Ok(())
}

fn environment(service: &Service, secret_id: &str) -> bool {
    service.template.as_ref().map_or(false, |template| {
        template
            .containers
            .iter()
            .flat_map(|container| container.env.iter())
            .filter_map(|environment| environment.value_source())
            .filter_map(|source| source.secret_key_ref.as_ref())
            .filter(|selector| selector.secret == secret_id)
            .any(|selector| selector.version == "latest")
    })
}

fn volume(service: &Service, secret_id: &str) -> bool {
    service.template.as_ref().map_or(false, |template| {
        template
            .volumes
            .iter()
            .filter_map(|volume| volume.secret())
            .filter(|source| source.secret.split("/").last() == Some(secret_id))
            .map(|source| &source.items)
            .any(|paths| paths.is_empty() || paths.iter().any(|path| path.version == "latest"))
    })
}

async fn handler(Extension(run): Extension<CloudRun>, entry: Entry) -> Result<StatusCode, Error> {
    // projects/PROJECT_NUMBER/secrets/SECRET_NAME/versions/VERSION_NUMBER
    let actor = entry.proto_payload.authentication_info.principal_email;
    let resource = entry.proto_payload.resource_name;
    let parts: Vec<_> = resource.split('/').collect();
    let project = parts[1];
    let secret = parts[3];
    let location = "-";
    info!(actor = actor, resource = resource, "secret changed");

    let operations = run
        .list_services()
        .set_parent(format!("projects/{}/locations/{}", project, location))
        .send()
        .await?
        .services
        .into_iter()
        .filter(|service| environment(service, secret) || volume(service, secret))
        .map(async |mut service: Service| -> Result<()> {
            info!(service = service.name, "updating service");

            service.template.as_mut().map(|template| {
                template.revision = String::new();
            });

            run.update_service()
                .set_service(service)
                .send()
                .await
                .map_err(|error| {
                    error!(
                        error = error.status().unwrap().message,
                        "failed to update service"
                    );
                    error
                })?;

            Ok(())
        });

    let _ = join_all(operations.collect::<Vec<_>>())
        .await
        .iter()
        .filter_map(|result: &Result<(), anyhow::Error>| result.as_ref().err());

    Ok(StatusCode::OK)
}
