use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::fmt::{Display, Formatter};
use tracing::{error, warn};

pub enum Error {
    BadRequest(anyhow::Error),
    Internal(anyhow::Error),
}

impl Error {
    pub fn bad_request(error: anyhow::Error) -> Self {
        Self::BadRequest(error)
    }

    pub fn internal(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}

impl From<google_cloud_run_v2::Error> for Error {
    fn from(value: google_cloud_run_v2::Error) -> Self {
        Self::Internal(value.into())
    }
}

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Self::Internal(value)
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest(error) => write!(formatter, "bad request: {error}"),
            Self::Internal(error) => write!(formatter, "internal error: {error}"),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::BadRequest(error_details) => {
                warn!(error = %error_details, "request rejected");
                (StatusCode::BAD_REQUEST, "invalid event payload").into_response()
            }
            Self::Internal(error_details) => {
                error!(error = %error_details, "request failed");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
            }
        }
    }
}
