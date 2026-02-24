use crate::{
    middleware::log::Log,
    model::{Entry, Error, Secret},
    service::cloud_run::CloudRunService,
};
use axum::{Json, Router, extract::State, middleware::from_fn, routing::post};
use tower_http::trace::TraceLayer;
use tracing::{info, instrument};

#[derive(Clone, Debug)]
pub struct Reload {
    cloud_run: CloudRunService,
}

impl Reload {
    pub fn init(cloud_run: CloudRunService) -> Router {
        let route = Self { cloud_run };
        Router::new()
            .route("/", post(Self::handler))
            .with_state(route)
            .layer(from_fn(Log::request))
            .layer(TraceLayer::new_for_http())
    }

    #[instrument(skip(entry, state))]
    async fn handler(State(state): State<Self>, entry: Entry) -> Result<Json<Vec<String>>, Error> {
        let change = Secret::parse_change(entry)?;
        let services = state
            .cloud_run
            .reload(&change.project, &change.secret)
            .await?;
        info!(
            actor = change.actor,
            resource = change.secret,
            "secret changed"
        );
        Ok(Json(services))
    }
}
