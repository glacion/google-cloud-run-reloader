use axum::{extract::Request, middleware::Next, response::Response};
use std::time::Instant;
use tracing::info;

pub async fn log_requests(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let started_at = Instant::now();

    let response = next.run(request).await;
    let status = response.status().as_u16();
    let elapsed_ms = started_at.elapsed().as_millis();

    info!(
        method = method.as_str(),
        path,
        status,
        elapsed_ms,
        "request completed"
    );

    response
}
