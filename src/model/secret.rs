use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use super::{Entry, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Secret {
    pub project: String,
    pub secret: String,
    pub version: String,
}

pub struct SecretChange {
    pub actor: String,
    pub resource: String,
    pub project: String,
    pub secret: String,
}

impl Secret {
    pub fn parse(resource: &str) -> Result<Self> {
        resource.to_owned().try_into()
    }

    pub fn parse_change(entry: Entry) -> Result<SecretChange, Error> {
        let actor = entry.proto_payload.authentication_info.principal_email;
        let resource = entry.proto_payload.resource_name;
        let secret_resource = Self::parse(&resource).map_err(Error::bad_request)?;

        Ok(SecretChange {
            actor,
            resource,
            project: secret_resource.project,
            secret: secret_resource.secret,
        })
    }
}

impl TryFrom<String> for Secret {
    type Error = anyhow::Error;

    fn try_from(resource: String) -> Result<Self, Self::Error> {
        let parts: Vec<&str> = resource.split('/').collect();
        if parts.len() != 6 {
            return Err(anyhow!("invalid resource name format: {}", resource));
        }

        if parts[0] != "projects" || parts[2] != "secrets" || parts[4] != "versions" {
            return Err(anyhow!("unexpected resource segments: {}", resource));
        }

        Ok(Self {
            project: parts[1].to_owned(),
            secret: parts[3].to_owned(),
            version: parts[5].to_owned(),
        })
    }
}

impl From<Secret> for String {
    fn from(value: Secret) -> Self {
        format!(
            "projects/{}/secrets/{}/versions/{}",
            value.project, value.secret, value.version
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Secret;

    #[test]
    fn parses_valid_secret_resource() {
        let secret = Secret::parse("projects/123/secrets/my-secret/versions/latest")
            .expect("secret resource should parse");

        assert_eq!(secret.project, "123");
        assert_eq!(secret.secret, "my-secret");
        assert_eq!(secret.version, "latest");
    }

    #[test]
    fn serializes_to_resource_name() {
        let resource = String::from(Secret {
            project: "123".to_owned(),
            secret: "my-secret".to_owned(),
            version: "latest".to_owned(),
        });

        assert_eq!(resource, "projects/123/secrets/my-secret/versions/latest");
    }

    #[test]
    fn rejects_invalid_resource_format() {
        let error = Secret::parse("projects/123/secrets/my-secret")
            .expect_err("invalid resource format should fail parsing");

        assert!(error
            .to_string()
            .contains("invalid resource name format: projects/123/secrets/my-secret"));
    }

    #[test]
    fn rejects_unexpected_resource_segments() {
        let error = Secret::parse("project/123/secret/my-secret/version/latest")
            .expect_err("unexpected segments should fail parsing");

        assert!(error
            .to_string()
            .contains("unexpected resource segments: project/123/secret/my-secret/version/latest"));
    }
}
