use crate::a2a::metrics;
use crate::inbound;
use crate::outbound;
use crate::outbound::A2aTargetSpec;
use crate::xds::XdsStore;
use crate::{a2a, backend, rbac};
use a2a_sdk::AgentCard;
use anyhow::Context;
use bytes::Bytes;
use eventsource_stream::Eventsource;
use http::HeaderName;
use http::{HeaderMap, HeaderValue, header::AUTHORIZATION};
use serde::Serialize;
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::instrument;

/// Relay is responsible for abstracting different A2A backends identified by their target name.
pub struct Relay {
	state: Arc<tokio::sync::RwLock<XdsStore>>,
	pool: pool::ConnectionPool,
	listener_name: String,
	_metrics: Arc<metrics::Metrics>,
}

impl Relay {
	pub fn new(
		state: Arc<tokio::sync::RwLock<XdsStore>>,
		metrics: Arc<metrics::Metrics>,
		listener_name: String,
	) -> Self {
		Self {
			state: state.clone(),
			pool: pool::ConnectionPool::new(state.clone(), listener_name.clone()),
			listener_name,
			_metrics: metrics,
		}
	}
}

pub enum Response {
	Streaming(ReceiverStream<a2a_sdk::JsonRpcMessage>),
	Single(a2a_sdk::JsonRpcMessage),
}

impl Relay {
	pub async fn fetch_agent_card(
		&self,
		host: String,
		ctx: &RqCtx,
		service_name: &str,
	) -> anyhow::Result<AgentCard> {
		let mut card = self
			.pool
			.connect(ctx, service_name)
			.await?
			.fetch_agent_card()
			.await?;
		let state = self.state.read().await;
		let listener = state
			.listeners
			.get(&self.listener_name)
			.expect("listener not found");
		let (url, pols) = match &listener.spec {
			inbound::ListenerType::A2a(a) => (a.url(host), a.policies()),
			inbound::ListenerType::Sse(s) => (s.url(host), s.policies()),
			inbound::ListenerType::Stdio => panic!("stdio listener not supported"),
		};
		card.url = format!("{}/{}", url, service_name);

		card.skills = card
			.skills
			.iter()
			.filter(|s| {
				// TODO for now we treat it as a 'tool'
				pols.validate(
					&rbac::ResourceType::Tool(rbac::ResourceId::new(
						service_name.to_string(),
						s.name.to_string(),
					)),
					&ctx.identity,
				)
			})
			.cloned()
			.collect();
		Ok(card)
	}
	pub async fn proxy_request(
		self,
		request: a2a_sdk::A2aRequest,
		rq_ctx: &RqCtx,
		service_name: String,
	) -> anyhow::Result<Response> {
		use futures::StreamExt;

		{
			// TODO: how can we apply RBAC? The request doesn't say what 'skill' to use
			// let state = self.state.read().await;
			// let pols = &state.policies;
			// pols.validate(&rbac::ResourceType::Tool { id: tool_name }, &ctx.identity)
		}
		let svc = self
			.pool
			.connect(rq_ctx, &service_name)
			.await
			.context(format!("Service {} not found", &service_name))?;
		let resp = svc.send_request(&request).await?;

		// TODO: implement RBAC
		let content = resp
			.headers()
			.get(reqwest::header::CONTENT_TYPE)
			.and_then(|value| value.to_str().ok())
			.and_then(|value| value.parse::<mime::Mime>().ok())
			.map(|mime| mime.type_().as_str().to_string() + "/" + mime.subtype().as_str());

		// This may be a streaming response or singleton.
		match content.as_deref() {
			Some("application/json") => {
				let j = resp.json::<a2a_sdk::JsonRpcMessage>().await?;
				Ok(Response::Single(j))
			},
			Some("text/event-stream") => {
				let (tx, rx) = tokio::sync::mpsc::channel::<a2a_sdk::JsonRpcMessage>(64);
				tokio::spawn(async move {
					let mut events = resp.bytes_stream().eventsource();

					while let Some(thing) = events.next().await {
						let event = thing.expect("TODO");
						if event.event == "message" {
							let j: a2a_sdk::JsonRpcMessage =
								serde_json::from_str(&event.data).expect("TODO handle error");
							tx.send(j).await.unwrap();
						}
					}
				});

				Ok(Response::Streaming(ReceiverStream::new(rx)))
			},
			_ => anyhow::bail!("expected JSON or event stream"),
		}
	}
}

mod pool {
	use super::*;

	pub(crate) struct ConnectionPool {
		listener_name: String,
		state: Arc<tokio::sync::RwLock<XdsStore>>,
	}

	impl ConnectionPool {
		pub(crate) fn new(state: Arc<tokio::sync::RwLock<XdsStore>>, listener_name: String) -> Self {
			Self {
				state,
				listener_name,
			}
		}

		#[instrument(level = "debug", skip_all, fields(name))]
		pub(crate) async fn connect(
			&self,
			rq_ctx: &RqCtx,
			name: &str,
		) -> Result<a2a::Client, anyhow::Error> {
			let target_info: Option<(
				outbound::Target<outbound::A2aTargetSpec>,
				tokio_util::sync::CancellationToken,
			)> = {
				let state = self.state.read().await;
				state
					.a2a_targets
					.get(name, &self.listener_name)
					.map(|(target, ct)| (target.clone(), ct.clone()))
			};

			// TODO use ct
			let Some((target, _ct)) = target_info else {
				// Handle target not found in state configuration
				return Err(anyhow::anyhow!(
					"Target configuration not found for {}",
					name
				));
			};
			tracing::trace!("connecting to target: {}", target.name);
			let transport = match &target.spec {
				A2aTargetSpec::Sse(sse) => {
					tracing::info!("starting A2a transport for target: {}", target.name);

					let scheme = match sse.port {
						443 => "https",
						_ => "http",
					};
					let url = format!("{}://{}:{}{}", scheme, sse.host, sse.port, sse.path);
					let mut upstream_headers = get_default_headers(sse.backend_auth.as_ref(), rq_ctx).await?;
					for (key, value) in sse.headers.iter() {
						upstream_headers.insert(
							HeaderName::from_bytes(key.as_bytes())?,
							HeaderValue::from_str(value)?,
						);
					}
					let client = reqwest::Client::builder()
						.default_headers(upstream_headers)
						.build()
						.unwrap();
					a2a::Client {
						url: reqwest::Url::parse(&url).expect("failed to parse url"),
						client,
					}
				},
			};
			Ok(transport)
		}
	}
}

struct SerializeStream<T>(T);

impl<T: Serialize> From<SerializeStream<T>> for bytes::Bytes {
	fn from(val: SerializeStream<T>) -> Self {
		Bytes::from(serde_json::to_vec(&val.0).unwrap())
	}
}

async fn get_default_headers(
	auth_config: Option<&backend::BackendAuthConfig>,
	rq_ctx: &RqCtx,
) -> Result<HeaderMap, anyhow::Error> {
	match auth_config {
		Some(auth_config) => {
			let backend_auth = auth_config.build(&rq_ctx.identity).await?;
			let token = backend_auth.get_token().await?;
			let mut upstream_headers = HeaderMap::new();
			let auth_value = HeaderValue::from_str(token.as_str())?;
			upstream_headers.insert(AUTHORIZATION, auth_value);
			Ok(upstream_headers)
		},
		None => Ok(HeaderMap::new()),
	}
}
#[derive(Clone)]
pub struct RqCtx {
	pub identity: rbac::Identity,
	pub context: opentelemetry::Context,
}

impl Default for RqCtx {
	fn default() -> Self {
		Self {
			identity: rbac::Identity::default(),
			context: opentelemetry::Context::new(),
		}
	}
}

impl RqCtx {
	pub fn new(identity: rbac::Identity, context: opentelemetry::Context) -> Self {
		Self { identity, context }
	}
}
