use crate::admin::add_cors_layer;
use crate::authn;
use crate::inbound;
use crate::relay;
use crate::relay::Relay;
use crate::trcng;
use crate::xds::XdsStore as AppState;
use crate::{proxyprotocol, rbac};
use anyhow::Result;
use axum::extract::{ConnectInfo, OptionalFromRequestParts};
use axum::{
	Json, RequestPartsExt, Router,
	extract::{Query, State},
	http::header::HeaderMap,
	http::{StatusCode, request::Parts},
	response::sse::{Event, KeepAlive, Sse},
	response::{IntoResponse, Response},
	routing::get,
};
use axum_extra::typed_header::TypedHeaderRejection;
use axum_extra::{
	TypedHeader,
	headers::{Authorization, authorization::Bearer},
};
use futures::{SinkExt, StreamExt, stream::Stream};
use rmcp::{
	model::{ClientJsonRpcMessage, GetExtensions, JsonRpcBatchRequestItem},
	service::serve_server_with_ct,
	transport::common::axum::session_id as generate_streamable_session_id,
	transport::streamable_http_server::session::{
		self, EventId, HEADER_LAST_EVENT_ID, HEADER_SESSION_ID, Session, SessionId,
		StreamableHttpMessageReceiver,
	},
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{self};
use tokio::sync::RwLock;
use tokio_stream::wrappers::ReceiverStream;

type SessionManager = Arc<tokio::sync::RwLock<HashMap<SessionId, Session>>>;

#[derive(Clone)]
pub struct App {
	state: Arc<tokio::sync::RwLock<AppState>>,
	connection_id: Arc<tokio::sync::RwLock<Option<String>>>,
	txs:
		Arc<tokio::sync::RwLock<HashMap<SessionId, tokio::sync::mpsc::Sender<ClientJsonRpcMessage>>>>,
	metrics: Arc<relay::metrics::Metrics>,
	authn: Arc<RwLock<Option<authn::JwtAuthenticator>>>,
	ct: tokio_util::sync::CancellationToken,
	listener_name: String,
	mcp_session_manager: SessionManager,
	sse_ping_interval: Duration,
}

impl App {
	pub fn new(
		state: Arc<tokio::sync::RwLock<AppState>>,
		metrics: Arc<relay::metrics::Metrics>,
		authn: Arc<RwLock<Option<authn::JwtAuthenticator>>>,
		ct: tokio_util::sync::CancellationToken,
		listener_name: String,
	) -> Self {
		Self {
			state,
			txs: Default::default(),
			metrics,
			authn,
			connection_id: Arc::new(tokio::sync::RwLock::new(None)),
			ct,
			listener_name,
			mcp_session_manager: Default::default(),
			sse_ping_interval: Duration::from_secs(15),
		}
	}

	pub fn router(&self) -> Router {
		Router::new()
			.route("/sse", get(sse_handler).post(post_event_handler))
			.route(
				"/mcp",
				get(mcp_get_handler)
					.post(mcp_post_handler)
					.delete(mcp_delete_handler),
			)
			.layer(add_cors_layer())
			.with_state(self.clone())
	}
}

impl OptionalFromRequestParts<App> for rbac::Claims {
	type Rejection = AuthError;

	async fn from_request_parts(
		parts: &mut Parts,
		state: &App,
	) -> Result<Option<Self>, Self::Rejection> {
		let authn = state.authn.read().await;
		match authn.as_ref() {
			Some(authn) => {
				let TypedHeader(Authorization(bearer)) = parts
					.extract::<TypedHeader<Authorization<Bearer>>>()
					.await
					.map_err(AuthError::NoAuthHeaderPresent)?;
				let claims = authn.authenticate(bearer.token()).await;
				match claims {
					Ok(claims) => Ok(Some(claims)),
					Err(e) => Err(AuthError::JwtError(e)),
				}
			},
			None => Ok(None),
		}
	}
}

impl IntoResponse for AuthError {
	fn into_response(self) -> Response {
		let (status, error_message) = match self {
			AuthError::NoAuthHeaderPresent(e) => (
				StatusCode::UNAUTHORIZED,
				format!("No auth header present, error: {}", e),
			),
			AuthError::JwtError(e) => (
				StatusCode::UNAUTHORIZED,
				match e {
					authn::AuthError::InvalidToken(e) => format!("Invalid token, error: {}", e),
				},
			),
		};
		let body = Json(json!({
				"error": error_message,
		}));
		(status, body).into_response()
	}
}

#[derive(Debug)]
pub enum AuthError {
	NoAuthHeaderPresent(TypedHeaderRejection),
	JwtError(authn::AuthError),
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostEventQuery {
	pub session_id: String,
}

async fn post_event_handler(
	State(app): State<App>,
	ConnectInfo(_connection): ConnectInfo<proxyprotocol::Address>,
	claims: Option<rbac::Claims>,
	headers: HeaderMap,
	Query(PostEventQuery { session_id }): Query<PostEventQuery>,
	Json(message): Json<ClientJsonRpcMessage>,
) -> Result<StatusCode, StatusCode> {
	tracing::info!(session_id, ?message, "new client message for /sse");
	let tx = {
		let rg = app.txs.read().await;
		rg.get(session_id.as_str())
			.ok_or(StatusCode::NOT_FOUND)?
			.clone()
	};

	let context = trcng::extract_context_from_request(&headers);

	// Add claims to the message for RBAC
	// TODO: maybe do it here so we don't need to do this.
	let mut message = message;
	if let ClientJsonRpcMessage::Request(req) = &mut message {
		let claims = rbac::Identity::new(claims, app.connection_id.read().await.clone());
		let rq_ctx = relay::RqCtx::new(claims, context);
		req.request.extensions_mut().insert(rq_ctx);
	}

	if tx.send(message).await.is_err() {
		tracing::error!("send message error");
		return Err(StatusCode::GONE);
	}
	Ok(StatusCode::ACCEPTED)
}

async fn sse_handler(
	State(app): State<App>,
	ConnectInfo(connection): ConnectInfo<proxyprotocol::Address>,
	_claims: Option<rbac::Claims>, // We want to validate, but no RBAC
) -> Result<Sse<impl Stream<Item = Result<Event, io::Error>>>, StatusCode> {
	// it's 4KB

	let session = generate_streamable_session_id();
	tracing::info!(%session, ?connection, "sse connection");
	let connection_id = connection.identity.clone().map(|i| i.to_string());
	{
		let mut writable = app.connection_id.write().await;
		*writable = connection_id;
	}
	use tokio_stream::wrappers::ReceiverStream;
	use tokio_util::sync::PollSender;
	let (from_client_tx, from_client_rx) = tokio::sync::mpsc::channel(64);
	let (to_client_tx, to_client_rx) = tokio::sync::mpsc::channel(64);
	app
		.txs
		.write()
		.await
		.insert(session.clone(), from_client_tx);
	{
		let session = session.clone();
		let policies = {
			let state = app.state.read().await;
			let listener = state
				.listeners
				.get(&app.listener_name)
				.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
			match &listener.spec {
				inbound::ListenerType::Sse(s) => s.policies().clone(),
				_ => rbac::RuleSets::default(),
			}
		};
		tokio::spawn(async move {
			let relay = Relay::new(
				app.state.clone(),
				app.metrics.clone(),
				policies,
				app.listener_name.clone(),
			);
			let stream = ReceiverStream::new(from_client_rx);
			let sink = PollSender::new(to_client_tx.clone()).sink_map_err(std::io::Error::other);
			let result = serve_server_with_ct(relay.clone(), (sink, stream), app.ct.child_token())
				.await
				.inspect_err(|e| {
					tracing::error!("serving error: {:?}", e);
				});

			if let Err(e) = result {
				tracing::error!(error = ?e, "initialize error");
				app.txs.write().await.remove(&session);
				return;
			}
			let state = app.state.read().await;
			let mut rx: tokio::sync::broadcast::Receiver<String> = state.mcp_targets.subscribe();
			drop(state);
			loop {
				// Add a listener drain channel here.
				tokio::select! {
					removed = rx.recv() => {
						tracing::info!("removed: {}", removed.clone().unwrap());
						if let Ok(name) = removed {
							relay.remove_target(&name).await.unwrap();
						}
					}
					_ = app.ct.cancelled() => {
						tracing::info!("cancelled");
						result.unwrap().cancel().await.unwrap();
						break;
					}
					_ = to_client_tx.closed() =>{
						tracing::info!("client disconnected");
						break;
					}
				};
			}
			app.txs.write().await.remove(&session);
		});
	}

	let stream = futures::stream::once(futures::future::ok(
		Event::default()
			.event("endpoint")
			.data(format!("?sessionId={session}")),
	))
	.chain(ReceiverStream::new(to_client_rx).map(|message| {
		match serde_json::to_string(&message) {
			Ok(bytes) => Ok(Event::default().event("message").data(&bytes)),
			Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
		}
	}));
	Ok(Sse::new(stream))
}

fn mcp_receiver_as_stream(
	receiver: StreamableHttpMessageReceiver,
) -> impl Stream<Item = Result<Event, io::Error>> {
	ReceiverStream::new(receiver.inner).map(|message| match serde_json::to_string(&message.message) {
		Ok(bytes) => Ok(
			Event::default()
				.event("message")
				.data(&bytes)
				.id(message.event_id.to_string()),
		),
		Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
	})
}

#[axum::debug_handler]
async fn mcp_post_handler(
	State(app): State<App>,
	ConnectInfo(connection): ConnectInfo<proxyprotocol::Address>,
	claims: Option<rbac::Claims>,
	headers: HeaderMap,
	Json(mut message): Json<ClientJsonRpcMessage>,
) -> Result<Response, Response> {
	let session_id_from_header = headers
		.get(HEADER_SESSION_ID)
		.and_then(|v| v.to_str().ok().map(String::from));

	{
		let mut writable_conn_id = app.connection_id.write().await;
		*writable_conn_id = connection.identity.clone().map(|i| i.to_string());
	}

	let tracing_context = trcng::extract_context_from_request(&headers);
	let rbac_identity = rbac::Identity::new(claims, app.connection_id.read().await.clone());
	let rq_ctx = Arc::new(relay::RqCtx::new(rbac_identity, tracing_context));

	match message {
		ClientJsonRpcMessage::Request(ref mut req) => {
			req.request.extensions_mut().insert(rq_ctx.clone());
		},
		ClientJsonRpcMessage::BatchRequest(ref mut batch) => {
			for item in batch.iter_mut() {
				if let JsonRpcBatchRequestItem::Request(req) = item {
					req.request.extensions_mut().insert(rq_ctx.clone());
				}
			}
		},
		_ => {},
	}

	if let Some(session_id) = session_id_from_header {
		tracing::debug!(%session_id, ?message, "new client message for /mcp existing session");
		let handle = {
			let sm = app.mcp_session_manager.read().await;
			let session = sm
				.get(session_id.as_str())
				.ok_or_else(|| (StatusCode::NOT_FOUND, "Session not found".to_string()).into_response())?;
			session.handle().clone()
		};

		match &message {
			ClientJsonRpcMessage::Request(_) | ClientJsonRpcMessage::BatchRequest(_) => {
				let receiver = handle.establish_request_wise_channel().await.map_err(|e| {
					(
						StatusCode::INTERNAL_SERVER_ERROR,
						format!("Failed to establish request channel: {e}"),
					)
						.into_response()
				})?;
				let http_request_id = receiver.http_request_id;
				if let Err(push_err) = handle.push_message(message, http_request_id).await {
					tracing::error!(%session_id, ?push_err, "Push message error for /mcp");
					return Err(
						(
							StatusCode::INTERNAL_SERVER_ERROR,
							format!("Failed to push message: {push_err}"),
						)
							.into_response(),
					);
				}
				let stream = mcp_receiver_as_stream(receiver);
				Ok(
					Sse::new(stream)
						.keep_alive(KeepAlive::new().interval(app.sse_ping_interval))
						.into_response(),
				)
			},
			_ => {
				let result = handle.push_message(message, None).await;
				if result.is_err() {
					Err((StatusCode::GONE, "Session terminated".to_string()).into_response())
				} else {
					Ok(StatusCode::ACCEPTED.into_response())
				}
			},
		}
	} else {
		let session_id = generate_streamable_session_id();
		tracing::debug!(%session_id, ?message, "New client message for /mcp, creating session");

		let (session, transport) = session::create_session(session_id.clone(), Default::default());

		let policies = {
			let state = app.state.read().await;
			let listener = state.listeners.get(&app.listener_name).ok_or(
				(
					StatusCode::INTERNAL_SERVER_ERROR,
					"Listener not found".to_string(),
				)
					.into_response(),
			)?;
			match &listener.spec {
				inbound::ListenerType::Sse(s) => s.policies().clone(),
				_ => rbac::RuleSets::default(),
			}
		};

		tokio::spawn(async move {
			let relay = Relay::new(
				app.state.clone(),
				app.metrics.clone(),
				policies,
				app.listener_name.clone(),
			);
			let result = serve_server_with_ct(relay.clone(), transport, app.ct.child_token())
				.await
				.inspect_err(|e| {
					tracing::error!("serving error: {:?}", e);
				});

			let state = app.state.read().await;
			let mut rx: tokio::sync::broadcast::Receiver<String> = state.mcp_targets.subscribe();
			drop(state);
			loop {
				// Add a listener drain channel here.
				tokio::select! {
					removed = rx.recv() => {
						tracing::info!("removed: {}", removed.clone().unwrap());
						if let Ok(name) = removed {
							relay.remove_target(&name).await.unwrap();
						}
					}
					_ = app.ct.cancelled() => {
						tracing::info!("cancelled");
						result.unwrap().cancel().await.unwrap();
						break;
					}
				};
			}
		});

		let response_message = session.handle().initialize(message).await.map_err(|e| {
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				format!("Failed to initialize session: {e}"),
			)
				.into_response()
		})?;

		let mut response = Json(response_message).into_response();
		response.headers_mut().insert(
			HEADER_SESSION_ID,
			http::HeaderValue::from_str(&session_id).map_err(|_| {
				(
					StatusCode::INTERNAL_SERVER_ERROR,
					"Invalid session ID generated".to_string(),
				)
					.into_response()
			})?,
		);

		app
			.mcp_session_manager
			.write()
			.await
			.insert(session_id, session);
		Ok(response)
	}
}

#[axum::debug_handler]
async fn mcp_get_handler(
	State(app): State<App>,
	claims: Option<rbac::Claims>,
	ConnectInfo(connection): ConnectInfo<proxyprotocol::Address>,
	headers: HeaderMap,
) -> Result<Sse<impl Stream<Item = Result<Event, io::Error>>>, Response> {
	if app.authn.read().await.is_some() && claims.is_none() {
		return Err(
			(
				StatusCode::UNAUTHORIZED,
				"Authentication required".to_string(),
			)
				.into_response(),
		);
	}
	{
		let mut writable_conn_id = app.connection_id.write().await;
		*writable_conn_id = connection.identity.map(|i| i.to_string());
	}

	let session_id_from_header = headers.get(HEADER_SESSION_ID).and_then(|v| v.to_str().ok());

	if let Some(session_id_str) = session_id_from_header {
		let session_id = session_id_str.to_string();
		let last_event_id_str = headers
			.get(HEADER_LAST_EVENT_ID)
			.and_then(|v| v.to_str().ok());

		match last_event_id_str {
			Some(last_event_id_val) => {
				let last_event_id = last_event_id_val.parse::<EventId>().map_err(|e| {
					(
						StatusCode::BAD_REQUEST,
						format!("Invalid {HEADER_LAST_EVENT_ID}: {e}"),
					)
						.into_response()
				})?;
				tracing::debug!(%session_id, ?last_event_id, "Resuming /mcp session");
				let sm = app.mcp_session_manager.read().await;
				let session = sm.get(session_id_str).ok_or_else(|| {
					(
						StatusCode::NOT_FOUND,
						format!("Session {session_id} not found"),
					)
						.into_response()
				})?;
				let handle = session.handle();
				let receiver = handle.resume(last_event_id).await.map_err(|e| {
					(
						StatusCode::INTERNAL_SERVER_ERROR,
						format!("Resume error: {e}"),
					)
						.into_response()
				})?;
				let stream = mcp_receiver_as_stream(receiver);
				Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(app.sse_ping_interval)))
			},
			None => {
				tracing::debug!(%session_id, "Establishing common channel for /mcp session");
				let sm = app.mcp_session_manager.read().await;
				let session = sm.get(session_id_str).ok_or_else(|| {
					(
						StatusCode::NOT_FOUND,
						format!("Session {session_id} not found"),
					)
						.into_response()
				})?;
				let handle = session.handle();
				let receiver = handle.establish_common_channel().await.map_err(|e| {
					(
						StatusCode::INTERNAL_SERVER_ERROR,
						format!("Establish common channel error: {e}"),
					)
						.into_response()
				})?;
				let stream = mcp_receiver_as_stream(receiver);
				Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(app.sse_ping_interval)))
			},
		}
	} else {
		Err(
			(
				StatusCode::BAD_REQUEST,
				format!("Missing {HEADER_SESSION_ID} header"),
			)
				.into_response(),
		)
	}
}

#[axum::debug_handler]
async fn mcp_delete_handler(
	State(app): State<App>,
	claims: Option<rbac::Claims>,
	headers: HeaderMap,
) -> Result<StatusCode, Response> {
	if app.authn.read().await.is_some() && claims.is_none() {
		return Err(
			(
				StatusCode::UNAUTHORIZED,
				"Authentication required".to_string(),
			)
				.into_response(),
		);
	}

	if let Some(session_id_val) = headers.get(HEADER_SESSION_ID) {
		let session_id_str = session_id_val.to_str().map_err(|e| {
			(
				StatusCode::BAD_REQUEST,
				format!("Invalid {HEADER_SESSION_ID}: {e}"),
			)
				.into_response()
		})?;

		let session_id = session_id_str.to_string();
		tracing::info!(%session_id, "Attempting to delete /mcp session");

		let mut sm = app.mcp_session_manager.write().await;
		let session = sm
			.remove(session_id_str)
			.ok_or_else(|| (StatusCode::NOT_FOUND, "Session not found".to_string()).into_response())?;

		match session.cancel().await {
			Ok(quit_reason) => {
				tracing::info!(%session_id, ?quit_reason, "/mcp session deleted successfully");
				Ok(StatusCode::ACCEPTED)
			},
			Err(e) => {
				tracing::error!(%session_id, error = ?e, "Error cancelling /mcp session (JoinError)");
				Err(
					(
						StatusCode::INTERNAL_SERVER_ERROR,
						format!("Failed to cancel session {session_id}: {e}"),
					)
						.into_response(),
				)
			},
		}
	} else {
		Err(
			(
				StatusCode::BAD_REQUEST,
				format!("Missing {HEADER_SESSION_ID} header"),
			)
				.into_response(),
		)
	}
}
