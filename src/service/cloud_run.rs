use anyhow::{Context, Result, anyhow};
use futures::future::join_all;
use google_cloud_run_v2::{client::Services as CloudRun, model::Service};
use tracing::info;

use crate::model::Error;

#[derive(Clone)]
pub struct CloudRunService {
    run: CloudRun,
}

impl CloudRunService {
    pub async fn connect() -> Result<Self> {
        let run = CloudRun::builder()
            .build()
            .await
            .context("failed to connect to cloud run v2 services")?;
        Ok(Self::new(run))
    }

    pub fn new(run: CloudRun) -> Self {
        Self { run }
    }

    pub async fn reload_services_for_secret(&self, project: &str, secret: &str) -> Result<(), Error> {
        reload_services_for_secret(&self.run, project, secret).await
    }
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

pub async fn reload_services_for_secret(
    run: &CloudRun,
    project: &str,
    secret: &str,
) -> Result<(), Error> {
    let location = "-";
    let operations = run
        .list_services()
        .set_parent(format!("projects/{}/locations/{}", project, location))
        .send()
        .await?
        .services
        .into_iter()
        .filter(|service| environment(service, secret) || volume(service, secret))
        .map(async |mut service: Service| -> Result<()> {
            let service_name = service.name.clone();
            info!(service = service_name, "updating service");

            if let Some(template) = service.template.as_mut() {
                template.revision = String::new();
            }

            run.update_service()
                .set_service(service)
                .send()
                .await
                .with_context(|| format!("failed to update service {service_name}"))?;

            Ok(())
        });

    let update_results = join_all(operations.collect::<Vec<_>>()).await;
    let failures = update_results
        .into_iter()
        .filter_map(Result::err)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();

    if !failures.is_empty() {
        return Err(Error::internal(anyhow!(
            "failed to update {} service(s): {}",
            failures.len(),
            failures.join("; ")
        )));
    }

    Ok(())
}
