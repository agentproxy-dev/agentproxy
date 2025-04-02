use serde::{Deserialize, Serialize};
use std::default::Default;

#[cfg(feature = "aws")]
pub(crate) mod aws;
#[cfg(feature = "gcp")]
pub(crate) mod gcp;

#[async_trait::async_trait]
pub trait BackendAuth: Send + Sync {
	async fn get_token(&self) -> Result<String, anyhow::Error>;
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(tag = "type")]
pub enum BackendAuthConfig {
	#[default]
	#[cfg(feature = "gcp")]
	#[serde(rename = "gcp")]
	GCP,
	#[cfg(feature = "aws")]
	#[serde(rename = "aws")]
	AWS,
}

impl BackendAuthConfig {
	pub async fn build(&self) -> impl BackendAuth {
		match self {
			BackendAuthConfig::GCP => gcp::GCPBackend::new().await.unwrap(),
			#[cfg(feature = "aws")]
			BackendAuthConfig::AWS => {
				panic!("AWS backend not implemented")
			},
		}
	}
}
