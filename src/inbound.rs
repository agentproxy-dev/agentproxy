use std::sync::Arc;

use crate::authn;
use crate::proxyprotocol;
use crate::relay;
use crate::signal;
use crate::sse::App as SseApp;
use crate::xds;
use rmcp::serve_server;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use tracing::info;

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Listener {
	#[serde(rename = "sse")]
	Sse(SseListener),
	#[serde(rename = "stdio")]
	Stdio,
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]

pub struct SseListener {
	host: String,
	port: u32,
	mode: Option<ListenerMode>,
	authn: Option<Authn>,
	tls: Option<TlsConfig>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct TlsConfig {
	key_pem: Option<LocalDataSource>,
	cert_pem: Option<LocalDataSource>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum LocalDataSource {
	#[serde(rename = "file")]
	File(String),
	#[serde(rename = "inline")]
	Inline(String),
}

#[derive(Debug)]
pub enum ServingError {
	Sse(std::io::Error),
	StdIo(tokio::task::JoinError),
}

impl Listener {
	pub async fn listen(
		&self,
		state: Arc<std::sync::RwLock<xds::XdsStore>>,
		metrics: Arc<relay::metrics::Metrics>,
	) -> Result<(), ServingError> {
		match self {
			Listener::Stdio {} => {
				let relay = serve_server(
					// TODO: This is a hack
					relay::Relay::new(state.clone(), metrics),
					(tokio::io::stdin(), tokio::io::stdout()),
				)
				.await
				.inspect_err(|e| {
					tracing::error!("serving error: {:?}", e);
				})
				.unwrap();
				tracing::info!("serving stdio");
				relay
					.waiting()
					.await
					.map_err(ServingError::StdIo)
					.map(|_| ())
					.inspect_err(|e| {
						tracing::error!("serving error: {:?}", e);
					})
			},
			Listener::Sse(sse_listener) => {
				let listener =
					tokio::net::TcpListener::bind(format!("{}:{}", sse_listener.host, sse_listener.port))
						.await
						.unwrap();
				let authenticator = match &sse_listener.authn {
					Some(authn) => match authn {
						Authn::Jwt(jwt) => Arc::new(tokio::sync::RwLock::new(Some(
							authn::JwtAuthenticator::new(jwt).await.unwrap(),
						))),
					},
					None => Arc::new(tokio::sync::RwLock::new(None)),
				};

				let mut run_set: tokio::task::JoinSet<Result<(), anyhow::Error>> =
					tokio::task::JoinSet::new();
				let clone = authenticator.clone();
				let ct = CancellationToken::new();
				let child_token = ct.child_token();
				run_set.spawn(async move {
					authn::sync_jwks_loop(clone, child_token)
						.await
						.map_err(|e| anyhow::anyhow!("error syncing jwks: {:?}", e))
				});

				run_set.spawn(async move {
					let sig = signal::Shutdown::new();
					sig.wait().await;
					ct.cancel();
					Ok(())
				});

				let app = SseApp::new(state.clone(), metrics, authenticator);
				let router = app.router();

				let enable_proxy = Some(&ListenerMode::Proxy) == sse_listener.mode.as_ref();

				let listener = proxyprotocol::Listener::new(listener, enable_proxy);
				let svc: axum::extract::connect_info::IntoMakeServiceWithConnectInfo<
					axum::Router,
					proxyprotocol::Address,
				> = router.into_make_service_with_connect_info::<proxyprotocol::Address>();
				info!("serving sse on {}:{}", sse_listener.host, sse_listener.port);
				run_set.spawn(async move {
					axum::serve(listener, svc)
						.with_graceful_shutdown(async {
							let sig = signal::Shutdown::new();
							sig.wait().await;
						})
						.await
						.map_err(ServingError::Sse)
						.inspect_err(|e| {
							tracing::error!("serving error: {:?}", e);
						})
						.map_err(|e| anyhow::anyhow!("serving error: {:?}", e))
				});

				while let Some(res) = run_set.join_next().await {
					match res {
						Ok(_) => {},
						Err(e) => {
							tracing::error!("serving error: {:?}", e);
						},
					}
				}
				Ok(())
			},
		}
	}
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum Authn {
	#[serde(rename = "jwt")]
	Jwt(authn::JwtConfig),
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum ListenerMode {
	#[serde(rename = "proxy")]
	Proxy,
}

impl Default for Listener {
	fn default() -> Self {
		Self::Stdio {}
	}
}
