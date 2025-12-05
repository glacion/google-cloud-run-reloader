use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub struct Error(anyhow::Error);

impl From<google_cloud_run_v2::Error> for Error {
    fn from(value: google_cloud_run_v2::Error) -> Self {
        value.into()
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("something went wrong: {}", self.0),
        )
            .into_response()
    }
}
