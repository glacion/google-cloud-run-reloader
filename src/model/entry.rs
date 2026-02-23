use anyhow::{Context, Result, anyhow};
use axum::extract::{FromRequest, Request};
use cloudevents::{Data, Event};
use serde::{Deserialize, Serialize};
use serde_json::{from_slice, from_str};
use tracing::debug;

use super::Error;

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
    type Rejection = Error;

    async fn from_request(request: Request, state: &T) -> Result<Self, Self::Rejection> {
        debug!("received event: {:?}", request.body());
        Event::from_request(request, state)
            .await
            .map_err(|_| Error::bad_request(anyhow!("failed to parse cloud event")))?
            .data()
            .context("missing cloud event data")
            .and_then(parse_event_data)
            .map_err(Error::bad_request)
    }
}

fn parse_event_data(data: &Data) -> Result<Entry> {
    match data {
        Data::Json(value) => Entry::deserialize(value).context("failed to deserialize event payload"),
        Data::String(value) => from_str::<serde_json::Value>(value)
            .context("cloud event data is not valid JSON")
            .and_then(|json| Entry::deserialize(&json).context("failed to deserialize event payload")),
        Data::Binary(value) => from_slice::<serde_json::Value>(value)
            .context("cloud event data is not valid JSON")
            .and_then(|json| Entry::deserialize(&json).context("failed to deserialize event payload")),
    }
}
