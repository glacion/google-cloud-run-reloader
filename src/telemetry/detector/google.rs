use anyhow::Result;
use opentelemetry::KeyValue;
use opentelemetry_sdk::{Resource, resource::ResourceDetector};
use opentelemetry_semantic_conventions::attribute::{
    CLOUD_ACCOUNT_ID, CLOUD_AVAILABILITY_ZONE, CLOUD_PLATFORM, CLOUD_PROVIDER, CLOUD_REGION,
    FAAS_INSTANCE, SERVICE_INSTANCE_ID,
};
use reqwest::Client;

#[derive(Clone, Debug)]
pub struct GoogleCloudDetector {
    resource: Resource,
}

impl ResourceDetector for GoogleCloudDetector {
    fn detect(&self) -> Resource {
        self.resource.clone()
    }
}

impl GoogleCloudDetector {
    pub async fn default() -> Result<Self> {
        let resource = match Self::resource().await {
            Ok(resource) => resource,
            Err(_) => Resource::builder().build(),
        };

        Ok(Self { resource })
    }

    async fn resource() -> Result<Resource> {
        let client = Client::builder().build()?;
        let instance_id = Self::metadata(&client, "instance/id").await?;
        let project_id = Self::metadata(&client, "project/project-id").await?;
        let region = Self::metadata(&client, "instance/region").await?;
        let zone = Self::metadata(&client, "instance/zone").await?;

        Ok(Resource::builder()
            .with_attributes([
                KeyValue::new(CLOUD_ACCOUNT_ID, project_id),
                KeyValue::new(CLOUD_AVAILABILITY_ZONE, zone),
                KeyValue::new(CLOUD_PLATFORM, "gcp_cloud_run"),
                KeyValue::new(CLOUD_PROVIDER, "gcp"),
                KeyValue::new(CLOUD_REGION, region),
                KeyValue::new(FAAS_INSTANCE, instance_id.clone()),
                KeyValue::new(SERVICE_INSTANCE_ID, instance_id),
            ])
            .build())
    }

    async fn metadata(client: &Client, path: &str) -> Result<String> {
        let url = format!("http://metadata.google.internal/computeMetadata/v1/{path}");
        Ok(client
            .get(url)
            .header("Metadata-Flavor", "Google")
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?)
    }
}
