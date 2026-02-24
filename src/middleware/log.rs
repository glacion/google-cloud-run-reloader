use axum::{extract::Request, middleware::Next, response::Response};
use std::time::Instant;
use tracing::info;

#[derive(Clone, Debug)]
pub struct Log;

impl Log {
    pub async fn request(request: Request, next: Next) -> Response {
        let method = request.method().clone().to_string();
        let path = request.uri().path().to_owned();
        let started = Instant::now();
        let response = next.run(request).await;
        let status = response.status().as_u16();
        let elapsed = started.elapsed().as_secs_f64() * 1_000.0;

        info!(elapsed, method, path, status, "request completed");

        response
    }
}
