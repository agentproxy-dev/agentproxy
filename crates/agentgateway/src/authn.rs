use crate::proto::agentgateway::dev::common;
use crate::proto::agentgateway::dev::listener::sse_listener::authn;
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{DecodingKey, Validation, decode, decode_header};
use secrecy::SecretString;
use serde::Serialize;
use serde_json::Value;
use serde_json::map::Map;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
#[derive(Debug)]
pub enum AuthError {
	InvalidToken(jsonwebtoken::errors::Error),
	NoValidKey(String),
}

#[derive(Clone, Serialize)]
pub struct JwtAuthenticator {
	#[serde(skip_serializing)]
	key: Arc<RwLock<KeySet>>,
	issuer: Option<HashSet<String>>,
	audience: Option<HashSet<String>>,

	remote: Option<JwksRemoteSource>,
}

impl std::fmt::Debug for JwtAuthenticator {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"JwtAuthenticator {{ issuer: {:?}, audience: {:?} }}",
			self.issuer, self.audience
		)
	}
}

#[derive(Debug)]
pub enum JwkError {
	JwksFetchError(reqwest::Error),
	JwksFileError(std::io::Error),
	JwksParseError(serde_json::Error),
	InvalidJWK(jsonwebtoken::errors::Error),
	UnsupportedAlgorithm,
	InvalidConfig(String),
}

#[derive(Clone, Serialize)]
struct JwksRemoteSource {
	#[serde(skip_serializing)]
	client: reqwest::Client,
	url: String,
	refresh_interval: Duration,
}

fn duration_from_pb(duration: Option<pbjson_types::Duration>, default: Duration) -> Duration {
	match duration {
		Some(duration) => {
			Duration::from_secs(duration.seconds as u64) + Duration::from_nanos(duration.nanos as u64)
		},
		None => default,
	}
}

impl JwksRemoteSource {
	fn from_xds(remote: &common::RemoteDataSource) -> Result<Self, JwkError> {
		let scheme = match remote.port {
			443 => "https",
			80 => "http",
			_ => return Err(JwkError::InvalidConfig("invalid port".to_string())),
		};
		let url = format!("{}://{}/{}", scheme, remote.host, remote.path);
		let client = reqwest::ClientBuilder::new()
			.timeout(duration_from_pb(
				remote.initial_timeout,
				Duration::from_secs(10),
			))
			.build()
			.map_err(JwkError::JwksFetchError)?;
		Ok(Self {
			client,
			url,
			refresh_interval: duration_from_pb(remote.refresh_interval, Duration::from_secs(10)),
		})
	}

	async fn fetch_jwks(&self) -> Result<JwkSet, JwkError> {
		let response = self
			.client
			.get(&self.url)
			.send()
			.await
			.map_err(JwkError::JwksFetchError)?;
		let jwks: JwkSet =
			serde_json::from_str(&response.text().await.map_err(JwkError::JwksFetchError)?)
				.map_err(JwkError::JwksParseError)?;
		Ok(jwks)
	}
}

impl JwtAuthenticator {
	pub async fn new(value: &authn::JwtConfig) -> Result<Self, JwkError> {
		let (jwks, remote): (JwkSet, Option<JwksRemoteSource>) = match &value.jwks {
			Some(authn::jwt_config::Jwks::LocalJwks(local)) => match &local.source {
				Some(common::local_data_source::Source::Inline(jwks)) => {
					let jwks: JwkSet = serde_json::from_slice(jwks).map_err(JwkError::JwksParseError)?;
					(jwks, None)
				},
				Some(common::local_data_source::Source::FilePath(path)) => {
					let file = std::fs::File::open(path).map_err(JwkError::JwksFileError)?;
					let jwks: JwkSet = serde_json::from_reader(file).map_err(JwkError::JwksParseError)?;
					(jwks, None)
				},
				_ => {
					return Err(JwkError::InvalidConfig(
						"invalid local JWKS source".to_string(),
					));
				},
			},
			Some(authn::jwt_config::Jwks::RemoteJwks(remote)) => {
				let remote = JwksRemoteSource::from_xds(remote)?;
				let jwks = remote.fetch_jwks().await?;
				(jwks, Some(remote.clone()))
			},
			_ => {
				return Err(JwkError::InvalidConfig("no JWKS provided".to_string()));
			},
		};
		for jwk in jwks.keys.iter() {
			if !jwk.is_supported() {
				tracing::error!("unsupported algorithm");
				return Err(JwkError::UnsupportedAlgorithm);
			}
		}
		let issuer = match value.issuer.len() {
			0 => None,
			_ => Some(HashSet::<String>::from_iter(
				value.issuer.iter().map(|s| s.to_string()),
			)),
		};
		let audience = match value.audience.len() {
			0 => None,
			_ => Some(HashSet::<String>::from_iter(
				value.audience.iter().map(|s| s.to_string()),
			)),
		};
		Ok(JwtAuthenticator {
			key: Arc::new(RwLock::new(KeySet::new(keys_from_jwks(&jwks)?))),
			issuer,
			audience,
			remote,
		})
	}

	pub async fn sync_jwks(&mut self) -> Result<(), JwkError> {
		match &self.remote {
			Some(remote) => {
				let jwk = remote.fetch_jwks().await?;
				let keys = keys_from_jwks(&jwk)?;
				self.key.write().await.update(keys);
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
	let interval: Duration = authn
		.read()
		.await
		.as_ref()
		.map_or(Duration::from_secs(10), |authn| {
			authn
				.remote
				.as_ref()
				.map_or(Duration::from_secs(10), |remote| remote.refresh_interval)
		});
	loop {
		tokio::select! {
			_ = ct.cancelled() => {
				tracing::info!("cancelled sync_jwks_loop");
				return Ok(());
			},
			_ = tokio::time::sleep(interval) => {
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

// keys_from_jwks converts a JwkSet to a Vec of (Option<String>, DecodingKey)
fn keys_from_jwks(jwks: &JwkSet) -> Result<Vec<(Option<String>, DecodingKey)>, JwkError> {
	jwks
		.keys
		.iter()
		.map(|jwk| -> Result<(Option<String>, DecodingKey), JwkError> {
			Ok((
				jwk.common.key_id.clone(),
				DecodingKey::from_jwk(jwk).map_err(JwkError::InvalidJWK)?,
			))
		})
		.collect()
}

// KeySet is a collection of keys that can be updated atomically
pub struct KeySet {
	keys: Vec<(Option<String>, DecodingKey)>,
}

// KeySet is a collection of keys that can be updated atomically
impl KeySet {
	pub fn new(keys: Vec<(Option<String>, DecodingKey)>) -> Self {
		Self { keys }
	}

	pub fn update(&mut self, key: Vec<(Option<String>, DecodingKey)>) {
		self.keys = key;
	}
}

impl From<Vec<(Option<String>, DecodingKey)>> for KeySet {
	fn from(keys: Vec<(Option<String>, DecodingKey)>) -> Self {
		Self::new(keys)
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

		let key_set = self.key.read().await;
		let key = match (header.kid, key_set.keys.len()) {
			// If there is a kid, find the key that matches the kid
			(Some(kid), _) => key_set
				.keys
				.iter()
				.find(|(id, _)| match (id, &kid) {
					(Some(id), kid) => id == kid,
					_ => false,
				})
				.ok_or(AuthError::NoValidKey("no key matching kid".to_string()))?,
			// If there is no kid, and there is only one key, use the first key
			(None, 1) => {
				// Use the first key if no kid is present
				key_set
					.keys
					.first()
					.ok_or(AuthError::NoValidKey("no key found".to_string()))?
			},
			// If there is no kid, and there is more than one key, return an error
			(None, _) => {
				return Err(AuthError::NoValidKey("no key found".to_string()));
			},
		};

		let token_data =
			decode::<Map<String, Value>>(token, &key.1, &validation).map_err(AuthError::InvalidToken)?;
		Ok(crate::rbac::Claims::new(
			token_data.claims,
			SecretString::new(token.into()),
		))
	}
}
