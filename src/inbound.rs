use crate::authn;
use crate::authn::JwtAuthenticator;
use crate::proto;
use crate::proto::mcpproxy::dev::listener::{
	Listener as XdsListener,
	listener::{
		Listener as XdsListenerSpec, SseListener as XdsSseListener,
		sse_listener::TlsConfig as XdsTlsConfig,
	},
};
use crate::proxyprotocol;
use crate::relay;
use crate::signal;
use crate::sse::App as SseApp;
use crate::xds;
use rmcp::serve_server;
use serde::{Deserialize, Serialize};
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tls_listener::TlsListener;
use tokio_rustls::{
	TlsAcceptor,
	rustls::ServerConfig,
	rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
};
use tokio_util::sync::CancellationToken;
use tracing::info;

#[derive(Clone, Serialize, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Listener {
	#[serde(rename = "sse")]
	Sse(SseListener),
	#[serde(rename = "stdio")]
	Stdio,
}

impl Listener {
	pub async fn from_xds(value: XdsListener) -> Result<Self, anyhow::Error> {
		Ok(match value.listener {
			Some(XdsListenerSpec::Sse(sse)) => Listener::Sse(SseListener::from_xds(sse).await?),
			Some(XdsListenerSpec::Stdio(_)) => Listener::Stdio,
			_ => Listener::Stdio,
		})
	}
}

#[derive(Clone, Serialize, Debug)]

pub struct SseListener {
	host: String,
	port: u32,
	mode: Option<ListenerMode>,
	authn: Option<JwtAuthenticator>,
	tls: Option<TlsConfig>,
}

impl SseListener {
	async fn from_xds(value: XdsSseListener) -> Result<Self, anyhow::Error> {
		let tls = match value.tls {
			Some(tls) => Some(from_xds_tls_config(tls)?),
			None => None,
		};
		let authn = match value.authn {
			Some(authn) => match authn.jwt {
				Some(jwt) => Some(
					JwtAuthenticator::new(&jwt)
						.await
						.map_err(|e| anyhow::anyhow!("error creating jwt authenticator: {:?}", e))?,
				),
				None => None,
			},
			None => None,
		};
		Ok(SseListener {
			host: value.address,
			port: value.port,
			mode: None,
			authn,
			tls,
		})
	}
}
#[derive(Clone, Debug)]
pub struct TlsConfig {
	pub(crate) inner: Arc<ServerConfig>,
}

// TODO: Implement Serialize for TlsConfig
impl Serialize for TlsConfig {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		todo!()
	}
}

fn from_xds_tls_config(value: XdsTlsConfig) -> Result<TlsConfig, anyhow::Error> {
	let cert_bytes = value
		.cert_pem
		.ok_or(anyhow::anyhow!("cert_pem is required"))?
		.source
		.ok_or(anyhow::anyhow!("cert_pem source is required"))?;
	let key_bytes = value
		.key_pem
		.ok_or(anyhow::anyhow!("key_pem is required"))?
		.source
		.ok_or(anyhow::anyhow!("key_pem source is required"))?;
	let cert = proto::resolve_local_data_source(&cert_bytes)?;
	let key = proto::resolve_local_data_source(&key_bytes)?;
	Ok(TlsConfig {
		inner: rustls_server_config(key, cert)?,
	})
}

fn rustls_server_config(
	key: impl AsRef<Vec<u8>>,
	cert: impl AsRef<Vec<u8>>,
) -> Result<Arc<ServerConfig>, anyhow::Error> {
	let key = PrivateKeyDer::from_pem_slice(key.as_ref())?;

	let certs = CertificateDer::pem_slice_iter(cert.as_ref())
		.map(|cert| cert.unwrap())
		.collect();

	let mut config = ServerConfig::builder()
		.with_no_client_auth()
		.with_single_cert(certs, key)?;

	config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

	Ok(Arc::new(config))
}

#[derive(Clone)]
struct AxumTlsAcceptor(TlsAcceptor);

impl tls_listener::AsyncTls<tokio::net::TcpStream> for AxumTlsAcceptor {
	type Stream = tokio_rustls::server::TlsStream<tokio::net::TcpStream>;
	type Error = std::io::Error;
	type AcceptFuture = tokio_rustls::Accept<tokio::net::TcpStream>;

	fn accept(&self, stream: tokio::net::TcpStream) -> Self::AcceptFuture {
		self.0.accept(stream)
	}
}

// We use a wrapper type to bridge axum's `Listener` trait to our `TlsListener` type.
struct AxumTlsListener {
	inner: TlsListener<tokio::net::TcpListener, AxumTlsAcceptor>,
	local_addr: SocketAddr,
}

impl axum::serve::Listener for AxumTlsListener {
	type Io = tokio_rustls::server::TlsStream<tokio::net::TcpStream>;
	type Addr = SocketAddr;

	async fn accept(&mut self) -> (Self::Io, Self::Addr) {
		loop {
			// To change the TLS certificate dynamically, you could `select!` on this call with a
			// channel receiver, and call `self.inner.replace_acceptor` in the other branch.
			match self.inner.accept().await {
				Ok(tuple) => break tuple,
				Err(tls_listener::Error::ListenerError(e)) if !is_connection_error(&e) => {
					// See https://github.com/tokio-rs/axum/blob/da3539cb0e5eed381361b2e688a776da77c52cd6/axum/src/serve/listener.rs#L145-L157
					// for the rationale.
					tokio::time::sleep(Duration::from_secs(1)).await
				},
				Err(_) => continue,
			}
		}
	}

	fn local_addr(&self) -> io::Result<Self::Addr> {
		Ok(self.local_addr)
	}
}

// Taken from https://github.com/tokio-rs/axum/blob/da3539cb0e5eed381361b2e688a776da77c52cd6/axum/src/serve/listener.rs#L160-L167
fn is_connection_error(e: &io::Error) -> bool {
	matches!(
		e.kind(),
		io::ErrorKind::ConnectionRefused
			| io::ErrorKind::ConnectionAborted
			| io::ErrorKind::ConnectionReset
	)
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
			Listener::Stdio => {
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
				let authenticator = match &sse_listener.authn {
					Some(authn) => Arc::new(tokio::sync::RwLock::new(Some(authn.clone()))),
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

				let socket_addr: SocketAddr = format!("{}:{}", sse_listener.host, sse_listener.port)
					.as_str()
					.parse()
					.unwrap();
				let listener = tokio::net::TcpListener::bind(socket_addr).await.unwrap();
				let app = SseApp::new(state.clone(), metrics, authenticator);
				let router = app.router();

				info!("serving sse on {}:{}", sse_listener.host, sse_listener.port);
				let child_token = ct.child_token();
				match &sse_listener.tls {
					Some(tls) => {
						let tls_acceptor = TlsAcceptor::from(tls.inner.clone());
						let axum_tls_acceptor = AxumTlsAcceptor(tls_acceptor);
						let tls_listener = AxumTlsListener {
							inner: tls_listener::TlsListener::new(axum_tls_acceptor, listener),
							local_addr: socket_addr,
						};
						run_set.spawn(async move {
							axum::serve(tls_listener, router.into_make_service())
								.with_graceful_shutdown(async move {
									child_token.cancelled().await;
								})
								.await
								.map_err(ServingError::Sse)
								.inspect_err(|e| {
									tracing::error!("serving error: {:?}", e);
								})
								.map_err(|e| anyhow::anyhow!("serving error: {:?}", e))
						});
					},
					None => {
						let enable_proxy = Some(&ListenerMode::Proxy) == sse_listener.mode.as_ref();

						let listener = proxyprotocol::Listener::new(listener, enable_proxy);
						let svc: axum::extract::connect_info::IntoMakeServiceWithConnectInfo<
							axum::Router,
							proxyprotocol::Address,
						> = router.into_make_service_with_connect_info::<proxyprotocol::Address>();
						run_set.spawn(async move {
							axum::serve(listener, svc)
								.with_graceful_shutdown(async move {
									child_token.cancelled().await;
								})
								.await
								.map_err(ServingError::Sse)
								.inspect_err(|e| {
									tracing::error!("serving error: {:?}", e);
								})
								.map_err(|e| anyhow::anyhow!("serving error: {:?}", e))
						});
					},
				}

				run_set.spawn(async move {
					let sig = signal::Shutdown::new();
					sig.wait().await;
					ct.cancel();
					Ok(())
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
pub enum ListenerMode {
	#[serde(rename = "proxy")]
	Proxy,
}

impl Default for Listener {
	fn default() -> Self {
		Self::Stdio {}
	}
}
