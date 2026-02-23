pub mod reload;

use axum::Router;

use crate::service::cloud_run::CloudRunService;

pub fn router(cloud_run: CloudRunService) -> Router {
    Router::new().merge(reload::router(cloud_run))
}
