use axum::{Router, http::StatusCode, routing::get};

#[derive(Clone, Debug)]
pub struct Health;

impl Health {
    pub fn init() -> Router {
        Router::new().route("/health", get(Self::handler))
    }

    async fn handler() -> StatusCode {
        StatusCode::OK
    }
}
