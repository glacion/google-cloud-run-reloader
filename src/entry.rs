use anyhow::Result;
use axum::{
    extract::{FromRequest, Request},
    http::StatusCode,
};
use cloudevents::Event;
use serde::{Deserialize, Serialize};
use serde_json::from_value;
use tracing::debug;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationInfo {
    pub principal_email: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtoPayload {
    pub authentication_info: AuthenticationInfo,
    pub resource_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub proto_payload: ProtoPayload,
}

impl<T> FromRequest<T> for Entry
where
    Event: FromRequest<T>,
    T: Sync,
{
    type Rejection = StatusCode;

    async fn from_request(request: Request, state: &T) -> Result<Self, Self::Rejection> {
        debug!("received event: {:?}", request.body());
        let result = Event::from_request(request, state)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?
            .data()
            .ok_or(StatusCode::BAD_REQUEST)?
            .to_owned()
            .try_into()
            .map_err(|_| StatusCode::BAD_REQUEST)
            .and_then(|value| from_value(value).map_err(|_| StatusCode::BAD_REQUEST));
        result
    }
}
