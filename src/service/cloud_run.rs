use crate::model::Error;
use anyhow::{Context, Result};
use futures::future::join_all;
use google_cloud_lro::Poller;
use google_cloud_run_v2::{client::Services as CloudRun, model::Service};
use google_cloud_wkt::FieldMask;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::error;

#[derive(Clone, Debug)]
pub struct CloudRunService {
    run: CloudRun,
}

impl CloudRunService {
    pub async fn init() -> Result<Self> {
        let run = CloudRun::builder()
            .build()
            .await
            .context("failed to connect to cloud run v2 services")?;

        Ok(Self { run })
    }

    pub async fn reload(&self, project: &str, secret: &str) -> Result<Vec<String>, Error> {
        let location = "-";

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos().to_string())
            .context("failed to retrieve system time")?;

        let targets = self
            .run
            .list_services()
            .set_parent(format!("projects/{}/locations/{}", project, location))
            .send()
            .await?
            .services
            .into_iter()
            .filter(|service| Self::environment(service, secret) || Self::volume(service, secret));

        let names = targets
            .clone()
            .map(|service| service.name)
            .collect::<Vec<_>>();

        let operations = targets
            .map(|service| self.update(service, &timestamp))
            .collect::<Vec<_>>();

        join_all(operations)
            .await
            .into_iter()
            .filter_map(Result::err)
            .inspect(|error| error!(error = %error, "failed to update service"))
            .next()
            .map_or(Ok(names), Err)
    }

    fn environment(service: &Service, secret_id: &str) -> bool {
        service.template.as_ref().is_some_and(|template| {
            template
                .containers
                .iter()
                .flat_map(|container| container.env.iter())
                .filter_map(|environment| environment.value_source())
                .filter_map(|source| source.secret_key_ref.as_ref())
                .filter(|selector| selector.secret == secret_id)
                .any(|selector| selector.version == "latest")
        })
    }

    async fn update(&self, service: Service, timestamp: &str) -> Result<Service, Error> {
        let template = service
            .template
            .clone()
            .context("service does not have a template")?
            .set_annotations([("reloader.glacion.com/timestamp", timestamp)]);

        self.run
            .update_service()
            .set_update_mask(FieldMask::default().set_paths(["template.annotations"]))
            .set_service(service.set_template(template))
            .poller()
            .until_done()
            .await
            .map_err(Error::from)
    }

    fn volume(service: &Service, secret_id: &str) -> bool {
        service.template.as_ref().is_some_and(|template| {
            template
                .volumes
                .iter()
                .filter_map(|volume| volume.secret())
                .filter(|source| source.secret.split('/').next_back() == Some(secret_id))
                .map(|source| &source.items)
                .any(|paths| paths.is_empty() || paths.iter().any(|path| path.version == "latest"))
        })
    }
}
