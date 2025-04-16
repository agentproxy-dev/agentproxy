use secrecy::ExposeSecret;
use secrecy::SecretString;
use serde::Serialize;

#[cfg(feature = "aws")]
pub(crate) mod aws;
#[cfg(feature = "gcp")]
pub(crate) mod gcp;

#[async_trait::async_trait]
pub trait BackendAuth: Send + Sync {
	async fn get_token(&self) -> Result<String, anyhow::Error>;
}

#[derive(Debug, Clone, Serialize)]
pub enum BackendAuthConfig {
	#[serde(rename = "passthrough", skip_serializing)]
	Passthrough,
	#[cfg(feature = "gcp")]
	#[serde(rename = "gcp")]
	GCP,
	#[cfg(feature = "aws")]
	#[serde(rename = "aws")]
	AWS,
}

#[derive(Debug, Clone)]
struct PassthroughBackend {
	token: SecretString,
}

#[async_trait::async_trait]
impl BackendAuth for PassthroughBackend {
	async fn get_token(&self) -> Result<String, anyhow::Error> {
		Ok(self.token.expose_secret().to_string())
	}
}

impl BackendAuthConfig {
	pub async fn build(
		&self,
		identity: &crate::rbac::Identity,
	) -> Result<Box<dyn BackendAuth>, anyhow::Error> {
		match self {
			BackendAuthConfig::Passthrough => match &identity.claims {
				Some(claims) => Ok(Box::new(PassthroughBackend {
					token: claims.jwt.clone(),
				})),
				None => Err(anyhow::anyhow!("Passthrough auth requires a JWT token")),
			},
			#[cfg(feature = "gcp")]
			BackendAuthConfig::GCP => Ok(Box::new(gcp::GCPBackend::new().await?)),
			#[cfg(feature = "aws")]
			BackendAuthConfig::AWS => {
				panic!("AWS backend not implemented")
			},
		}
	}
}
