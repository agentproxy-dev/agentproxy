use jsonwebtoken::jwk::Jwk;
use jsonwebtoken::{DecodingKey, Validation, decode, decode_header};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::map::Map;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub enum AuthError {
	InvalidToken(jsonwebtoken::errors::Error),
}

pub struct JwtAuthenticator {
	key: Arc<RwLock<MutableKey>>,
	issuer: Option<HashSet<String>>,
	audience: Option<HashSet<String>>,

	remote: Option<JwksRemoteSource>,
}

#[derive(Debug)]
pub enum JwkError {
	JwksFetchError(reqwest::Error),
	JwksFileError(std::io::Error),
	JwksParseError(serde_json::Error),
	InvalidJWK(jsonwebtoken::errors::Error),
	UnsupportedAlgorithm,
}

impl JwtAuthenticator {
	pub async fn new(value: &JwtConfig) -> Result<Self, JwkError> {
		let (jwk, remote): (Jwk, Option<JwksRemoteSource>) = match &value.jwks {
			JwksSource::Local(source) => match source {
				JwksLocalSource::Inline(jwk) => {
					let jwk: Jwk = serde_json::from_str(jwk).map_err(JwkError::JwksParseError)?;
					(jwk, None)
				},
				JwksLocalSource::File(path) => {
					let file = std::fs::File::open(path).map_err(JwkError::JwksFileError)?;
					let jwk: Jwk = serde_json::from_reader(file).map_err(JwkError::JwksParseError)?;
					(jwk, None)
				},
			},
			JwksSource::Remote(remote) => {
				let jwk = fetch_jwks(remote).await?;
				(jwk, Some(remote.clone()))
			},
		};
		if !jwk.is_supported() {
			tracing::error!("unsupported algorithm");
			return Err(JwkError::UnsupportedAlgorithm);
		}
		Ok(JwtAuthenticator {
			key: Arc::new(RwLock::new(
				DecodingKey::from_jwk(&jwk)
					.map_err(JwkError::InvalidJWK)?
					.into(),
			)),
			issuer: value.issuer.clone(),
			audience: value.audience.clone(),
			remote,
		})
	}

	pub async fn sync_jwks(&mut self) -> Result<(), JwkError> {
		match &self.remote {
			Some(remote) => {
				let jwk = fetch_jwks(remote).await?;
				self
					.key
					.write()
					.await
					.update(DecodingKey::from_jwk(&jwk).map_err(JwkError::InvalidJWK)?);
				Ok(())
			},
			None => Ok(()),
		}
	}
}

pub async fn sync_jwks_loop(
	authn: Arc<RwLock<Option<JwtAuthenticator>>>,
	ct: CancellationToken,
) -> Result<(), JwkError> {
	loop {
		tokio::select! {
			_ = ct.cancelled() => {
				tracing::info!("cancelled sync_jwks_loop");
				return Ok(());
			},
			_ = tokio::time::sleep(Duration::from_secs(10)) => {
				let mut authenticator = authn.write().await;
				match authenticator.as_mut() {
					Some(authenticator) => match authenticator.sync_jwks().await {
						Ok(_) => {
							tracing::trace!("synced jwks");
						},
						Err(e) => {
							tracing::error!("error syncing jwks: {:?}", e);
						},
					},
					None => {
						tracing::trace!("no authenticator, skipping sync");
					},
				}
				drop(authenticator);
			}
		}
	}
}

async fn fetch_jwks(remote: &JwksRemoteSource) -> Result<Jwk, JwkError> {
	let url = format!("{}:{}", remote.url, remote.port);
	let url = format!(
		"{}/{}",
		url,
		remote.path.clone().unwrap_or("jwks".to_string())
	);
	let client = reqwest::ClientBuilder::new()
		.timeout(remote.initial_timeout.unwrap_or(Duration::from_secs(10)))
		.build()
		.map_err(JwkError::JwksFetchError)?;
	let response = client
		.get(url)
		.send()
		.await
		.map_err(JwkError::JwksFetchError)?;
	let jwk: Jwk = serde_json::from_str(&response.text().await.map_err(JwkError::JwksFetchError)?)
		.map_err(JwkError::JwksParseError)?;
	Ok(jwk)
}

// MutableKey is a wrapper around DecodingKey that allows us to update the key atomically
pub struct MutableKey {
	key: DecodingKey,
}

impl MutableKey {
	pub fn new(key: DecodingKey) -> Self {
		Self { key }
	}

	pub fn update(&mut self, key: DecodingKey) {
		self.key = key;
	}
}

impl From<DecodingKey> for MutableKey {
	fn from(key: DecodingKey) -> Self {
		Self::new(key)
	}
}

impl JwtAuthenticator {
	pub async fn authenticate(&self, token: &str) -> Result<crate::rbac::Claims, AuthError> {
		let header = decode_header(token).map_err(AuthError::InvalidToken)?;

		let validation = {
			let mut validation = Validation::new(header.alg);
			validation.aud = self.audience.clone();
			validation.iss = self.issuer.clone();
			validation
		};

		let key = self.key.read().await;
		let token_data = decode::<Map<String, Value>>(token, &key.key, &validation)
			.map_err(AuthError::InvalidToken)?;
		tracing::info!("token data: {:?}", token_data);
		Ok(crate::rbac::Claims::new(token_data.claims))
	}
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum Authn {
	#[serde(rename = "jwt")]
	Jwt(JwtConfig),
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct JwtConfig {
	pub issuer: Option<HashSet<String>>,
	pub audience: Option<HashSet<String>>,
	pub jwks: JwksSource,
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum JwksSource {
	#[serde(rename = "local")]
	Local(JwksLocalSource),
	#[serde(rename = "remote")]
	Remote(JwksRemoteSource),
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct JwksRemoteSource {
	pub url: String,
	pub port: u16,
	pub path: Option<String>,
	pub headers: Option<HashMap<String, String>>,
	pub initial_timeout: Option<Duration>,
	pub refresh_interval: Option<Duration>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum JwksLocalSource {
	#[serde(rename = "file")]
	File(String),
	#[serde(rename = "inline")]
	Inline(String),
}
