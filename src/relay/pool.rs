use super::*;
use futures::{FutureExt, StreamExt, future::BoxFuture, stream::BoxStream};
use reqwest::{Client as HttpClient, IntoUrl, Url, header::ACCEPT};
use rmcp::model::ClientJsonRpcMessage;
use rmcp::service::serve_client_with_ct;
use rmcp::transport::sse::{SseClient, SseTransportError};
use sse_stream::{Error as SseError, Sse, SseStream};

pub(crate) struct ConnectionPool {
	state: Arc<tokio::sync::RwLock<XdsStore>>,
	by_name: HashMap<String, Arc<upstream::UpstreamTarget>>,
}

impl ConnectionPool {
	pub(crate) fn new(state: Arc<tokio::sync::RwLock<XdsStore>>) -> Self {
		Self {
			state,
			by_name: HashMap::new(),
		}
	}

	pub(crate) async fn get_or_create(
		&mut self,
		rq_ctx: &RqCtx,
		name: &str,
	) -> anyhow::Result<Arc<upstream::UpstreamTarget>> {
		// Connect if it doesn't exist
		if !self.by_name.contains_key(name) {
			// Read target info and drop lock before calling connect
			let target_info: Option<(Target, tokio_util::sync::CancellationToken)> = {
				let state = self.state.read().await;
				state
					.targets
					.get(name)
					.map(|(target, ct)| (target.clone(), ct.clone()))
			};

			if let Some((target, ct)) = target_info {
				// Now self is not immutably borrowed by state lock
				self.connect(rq_ctx, &ct, &target).await?;
			} else {
				// Handle target not found in state configuration
				return Err(anyhow::anyhow!(
					"Target configuration not found for {}",
					name
				));
			}
		}
		let target = self.by_name.get(name).cloned();
		Ok(target.ok_or(McpError::invalid_request(
			format!("Service {} not found", name),
			None,
		))?)
	}

	pub(crate) async fn remove(&mut self, name: &str) -> Option<Arc<upstream::UpstreamTarget>> {
		self.by_name.remove(name)
	}

	pub(crate) async fn list(
		&mut self,
		rq_ctx: &RqCtx,
	) -> anyhow::Result<Vec<(String, Arc<upstream::UpstreamTarget>)>> {
		// Iterate through all state targets, and get the connection from the pool
		// If the connection is not in the pool, connect to it and add it to the pool
		// 1. Get target configurations (name, Target, CancellationToken) from the state's TargetStore
		let targets_config: Vec<(String, (Target, tokio_util::sync::CancellationToken))> = {
			let state = self.state.read().await;
			// Iterate the underlying HashMap directly to get the full tuple
			state
				.targets
				.iter()
				.map(|(name, target)| (name.clone(), target.clone()))
				.collect()
		};

		// 2. Identify targets needing connection without holding lock or borrowing self mutably yet
		let mut connections_to_make = Vec::new();
		for (name, (target, ct)) in &targets_config {
			if !self.by_name.contains_key(name) {
				connections_to_make.push((name.clone(), target.clone(), ct.clone()));
			}
		}

		// 3. Connect the missing ones (self is borrowed mutably here)
		for (name, target, ct) in connections_to_make {
			tracing::debug!("Connecting missing target: {}", name);
			self.connect(rq_ctx, &ct, &target).await.map_err(|e| {
				tracing::error!("Failed to connect target {}: {}", name, e);
				e // Propagate error
			})?;
		}
		tracing::debug!("Finished connecting missing targets.");

		// 4. Collect all required connections from the pool
		let results = targets_config
			.into_iter()
			.filter_map(|(name, _)| self.by_name.get(&name).map(|arc| (name, arc.clone())))
			.collect();

		Ok(results)
	}

	#[instrument(
    level = "debug",
    skip_all,
    fields(
        name=%target.name,
    ),
  )]
	pub(crate) async fn connect(
		&mut self,
		rq_ctx: &RqCtx,
		ct: &tokio_util::sync::CancellationToken,
		target: &Target,
	) -> Result<(), anyhow::Error> {
		// Already connected
		if let Some(_transport) = self.by_name.get(&target.name) {
			return Ok(());
		}
		tracing::trace!("connecting to target: {}", target.name);
		let transport: upstream::UpstreamTarget = match &target.spec {
			TargetSpec::Sse {
				host,
				port,
				path,
				backend_auth,
				headers,
			} => {
				tracing::trace!("starting sse transport for target: {}", target.name);
				let path = match path.as_str() {
					"" => "/sse",
					_ => path,
				};
				let scheme = match port {
					443 => "https",
					_ => "http",
				};

				let url = format!("{}://{}:{}{}", scheme, host, port, path);
				let mut upstream_headers = get_default_headers(backend_auth, rq_ctx).await?;
				for (key, value) in headers {
					upstream_headers.insert(
						HeaderName::from_bytes(key.as_bytes())?,
						HeaderValue::from_str(value)?,
					);
				}
				let client = reqwest::Client::builder()
					.default_headers(upstream_headers)
					.build()
					.unwrap();
				let client = ReqwestSseClient::new_with_client(url.as_str(), client).await?;
				let transport = SseTransport::start_with_client(client).await?;

				upstream::UpstreamTarget::Mcp(serve_client_with_ct((), transport, ct.child_token()).await?)
			},
			TargetSpec::Stdio { cmd, args, env: _ } => {
				tracing::trace!("starting stdio transport for target: {}", target.name);
				upstream::UpstreamTarget::Mcp(
					serve_client_with_ct(
						(),
						TokioChildProcess::new(Command::new(cmd).args(args)).unwrap(),
						ct.child_token(),
					)
					.await?,
				)
			},
			TargetSpec::OpenAPI(open_api) => {
				tracing::info!("starting OpenAPI transport for target: {}", target.name);
				let client = reqwest::Client::new();

				let scheme = match open_api.port {
					443 => "https",
					_ => "http",
				};
				upstream::UpstreamTarget::OpenAPI(openapi::Handler {
					host: open_api.host.clone(),
					client,
					tools: open_api.tools.clone(),
					scheme: scheme.to_string(),
					prefix: open_api.prefix.clone(),
					port: open_api.port,
					headers: get_default_headers(&open_api.backend_auth, rq_ctx).await?,
				})
			},
			TargetSpec::A2a { .. } => {
				// TODO: we probably want to silently ignore these instead of log an error,
				// or make it so the API doesn't allow expressing this at all.
				anyhow::bail!("A2a target is not supported for target {}", target.name);
			},
		};
		self
			.by_name
			.insert(target.name.clone(), Arc::new(transport));
		Ok(())
	}
}

async fn get_default_headers(
	auth_config: &Option<backend::BackendAuthConfig>,
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

// This is mostly a copy/paste of the upstream ReqwestSseClient, but with the
// ability to dynamically add headers per-request. For example tracing headers,
// or custom headers for the backend.

#[derive(Clone)]
pub struct ReqwestSseClient {
	http_client: HttpClient,
	sse_url: Url,
}

impl ReqwestSseClient {
	pub async fn new_with_client<U>(
		url: U,
		client: HttpClient,
	) -> Result<Self, SseTransportError<reqwest::Error>>
	where
		U: IntoUrl,
	{
		let url = url.into_url()?;
		Ok(Self {
			http_client: client,
			sse_url: url,
		})
	}
}

const MIME_TYPE: &str = "text/event-stream";
const HEADER_LAST_EVENT_ID: &str = "Last-Event-ID";

impl SseClient<reqwest::Error> for ReqwestSseClient {
	fn connect(
		&self,
		last_event_id: Option<String>,
	) -> BoxFuture<
		'static,
		Result<BoxStream<'static, Result<Sse, SseError>>, SseTransportError<reqwest::Error>>,
	> {
		let client = self.http_client.clone();
		let sse_url = self.sse_url.as_ref().to_string();
		let last_event_id = last_event_id.clone();
		let fut = async move {
			let mut request_builder = client.get(&sse_url).header(ACCEPT, MIME_TYPE);
			if let Some(last_event_id) = last_event_id {
				request_builder = request_builder.header(HEADER_LAST_EVENT_ID, last_event_id);
			}
			let response = request_builder.send().await?;
			let response = response.error_for_status()?;
			match response.headers().get(reqwest::header::CONTENT_TYPE) {
				Some(ct) => {
					if !ct.as_bytes().starts_with(MIME_TYPE.as_bytes()) {
						return Err(SseTransportError::UnexpectedContentType(Some(ct.clone())));
					}
				},
				None => {
					return Err(SseTransportError::UnexpectedContentType(None));
				},
			}
			let event_stream = SseStream::from_byte_stream(response.bytes_stream()).boxed();
			Ok(event_stream)
		};
		fut.boxed()
	}

	fn post(
		&self,
		session_id: &str,
		message: ClientJsonRpcMessage,
	) -> BoxFuture<'static, Result<(), SseTransportError<reqwest::Error>>> {
		let client = self.http_client.clone();
		let sse_url = self.sse_url.clone();
		let session_id = session_id.to_string();
		Box::pin(async move {
			let mut headers = HeaderMap::new();
			if let ClientJsonRpcMessage::Request(req) = &message {
				match req.request.extensions().get::<RqCtx>() {
					Some(rq_ctx) => {
						let tracer = trcng::get_tracer();
						let _span = tracer
							.span_builder("sse_post")
							.with_kind(SpanKind::Client)
							.start_with_context(tracer, &rq_ctx.context);
						trcng::add_context_to_request(&mut headers, &rq_ctx.context);
					},
					None => {
						tracing::trace!("No RqCtx found in extensions");
					},
				}
			}
			let uri = sse_url.join(&session_id).map_err(SseTransportError::from)?;
			let request_builder = client.post(uri.as_ref()).json(&message).headers(headers);
			request_builder
				.send()
				.await
				.and_then(|resp| resp.error_for_status())
				.map_err(SseTransportError::from)
				.map(drop)
		})
	}
}
