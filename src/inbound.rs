use std::sync::Arc;

use rmcp::serve_server;
use tracing::info;

use crate::rbac;

use crate::sse::App as SseApp;
use crate::xds;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(tag = "type")]
#[allow(clippy::large_enum_variant)]
pub enum Listener {
	#[serde(rename = "sse")]
	Sse {
		host: String,
		port: u32,
		mode: Option<ListenerMode>,
		authn: Option<Authn>,
	},
	#[serde(rename = "stdio")]
	Stdio {},
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
		metrics: Arc<crate::relay::metrics::Metrics>,
	) -> Result<(), ServingError> {
		match self {
			Listener::Stdio {} => {
				let relay = serve_server(
					// TODO: This is a hack
					crate::relay::Relay::new(state.clone(), rbac::Identity::empty(), metrics),
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
			Listener::Sse {
				host,
				port,
				mode,
				authn,
			} => {
				let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port))
					.await
					.unwrap();
				let authenticator = match authn {
					Some(authn) => match authn {
						Authn::Jwt(jwt) => Arc::new(tokio::sync::RwLock::new(Some(
							crate::authn::JwtAuthenticator::new(jwt).await.unwrap(),
						))),
					},
					None => Arc::new(tokio::sync::RwLock::new(None)),
				};

				let mut run_set: tokio::task::JoinSet<Result<(), anyhow::Error>> =
					tokio::task::JoinSet::new();
				let clone = authenticator.clone();
				run_set.spawn(async move {
					crate::authn::sync_jwks_loop(clone)
						.await
						.map_err(|e| anyhow::anyhow!("error syncing jwks: {:?}", e))
				});

				let app = SseApp::new(state.clone(), metrics, authenticator);
				let router = app.router();

				let enable_proxy = Some(&ListenerMode::Proxy) == mode.as_ref();

				let listener = crate::proxyprotocol::Listener::new(listener, enable_proxy);
				let svc: axum::extract::connect_info::IntoMakeServiceWithConnectInfo<
					axum::Router,
					crate::proxyprotocol::Address,
				> = router.into_make_service_with_connect_info::<crate::proxyprotocol::Address>();
				info!("serving sse on {}:{}", host, port);
				run_set.spawn(async move {
					axum::serve(listener, svc)
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
#[serde(tag = "type")]
pub enum Authn {
	#[serde(rename = "jwt")]
	Jwt(crate::authn::JwtConfig),
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
