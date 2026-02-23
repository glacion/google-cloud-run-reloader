use axum::{Extension, Router, http::StatusCode, routing::post};
use tracing::info;

use crate::{
    model::{Entry, Error, Secret},
    service::cloud_run::CloudRunService,
};

pub fn router(cloud_run: CloudRunService) -> Router {
    Router::new()
        .route("/", post(handler))
        .layer(Extension(cloud_run))
}

pub async fn handler(
    Extension(cloud_run): Extension<CloudRunService>,
    entry: Entry,
) -> Result<StatusCode, Error> {
    let change = Secret::parse_change(entry)?;
    info!(actor = change.actor, resource = change.resource, "secret changed");
    cloud_run
        .reload_services_for_secret(&change.project, &change.secret)
        .await?;

    Ok(StatusCode::OK)
}
