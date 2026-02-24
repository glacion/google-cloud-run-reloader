pub mod health;
pub mod reload;

use axum::Router as AxumRouter;
use health::Health;
use reload::Reload;

use crate::service::cloud_run::CloudRunService;

#[derive(Clone, Debug)]
pub struct Router {
    pub axum: AxumRouter,
}

impl Router {
    pub fn init(cloud_run: CloudRunService) -> Router {
        let axum = AxumRouter::new()
            .merge(Health::init())
            .merge(Reload::init(cloud_run));
        Self { axum }
    }
}
