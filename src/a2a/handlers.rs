use crate::a2a::metrics;
use crate::a2a::relay;
use crate::admin::add_cors_layer;
use crate::mtrcs::Recorder;
use crate::sse::AuthError;
use crate::{a2a, authn, proxyprotocol, rbac, trcng};
use a2a_sdk::AgentCard;
use axum::extract::{ConnectInfo, OptionalFromRequestParts, Path, State};
use axum::response::sse::Event;
use axum::response::{IntoResponse, Response, Sse};
use axum::routing::{get, post};
use axum::{Json, RequestPartsExt, Router};
use axum_extra::TypedHeader;
use axum_extra::extract::Host;
use futures::Stream;
use futures::StreamExt;
use headers::Authorization;
use headers::authorization::Bearer;
use http::request::Parts;
use http::{HeaderMap, StatusCode};
use opentelemetry::trace::{Span, SpanKind};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct App {
	state: Arc<tokio::sync::RwLock<crate::xds::XdsStore>>,
	metrics: Arc<crate::a2a::metrics::Metrics>,
	authn: Arc<RwLock<Option<authn::JwtAuthenticator>>>,
	listener_name: String,
	_ct: tokio_util::sync::CancellationToken,
}

impl OptionalFromRequestParts<App> for rbac::Claims {
	type Rejection = AuthError;

	async fn from_request_parts(
		parts: &mut Parts,
		state: &App,
	) -> anyhow::Result<Option<Self>, Self::Rejection> {
		let authn = state.authn.read().await;
		match authn.as_ref() {
			Some(authn) => {
				tracing::info!("jwt");
				let TypedHeader(Authorization(bearer)) = parts
					.extract::<TypedHeader<Authorization<Bearer>>>()
					.await
					.map_err(AuthError::NoAuthHeaderPresent)?;
				tracing::info!("bearer: {}", bearer.token());
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

impl App {
	pub fn new(
		state: Arc<tokio::sync::RwLock<crate::xds::XdsStore>>,
		metrics: Arc<crate::a2a::metrics::Metrics>,
		authn: Arc<RwLock<Option<authn::JwtAuthenticator>>>,
		ct: tokio_util::sync::CancellationToken,
		listener_name: String,
	) -> Self {
		Self {
			state,
			metrics,
			authn,
			listener_name,
			_ct: ct,
		}
	}
	pub fn router(&self) -> Router {
		Router::new()
			.route("/{target}/.well-known/agent.json", get(agent_card_handler))
			.route("/{target}", post(agent_call_handler))
			.layer(add_cors_layer())
			.with_state(self.clone())
	}
}

async fn agent_card_handler(
	State(app): State<App>,
	Path(target): Path<String>,
	Host(host): Host,
	headers: HeaderMap,
	ConnectInfo(connection): ConnectInfo<proxyprotocol::Address>,
	claims: Option<rbac::Claims>,
) -> anyhow::Result<Json<AgentCard>, StatusCode> {
	tracing::info!("new agent card request");

	let relay = relay::Relay::new(
		app.state.clone(),
		app.metrics.clone(),
		app.listener_name.clone(),
	);
	let connection_id = connection.identity.clone().map(|i| i.to_string());
	let claims = rbac::Identity::new(claims, connection_id);
	let context = trcng::extract_context_from_request(&headers);
	let rq_ctx = relay::RqCtx::new(claims, context);

	let tracer = trcng::get_tracer();
	let _span = trcng::start_span_with_attributes(
		"agent_card",
		&rq_ctx.identity,
		vec![opentelemetry::KeyValue::new("agent", target.clone())],
	)
	.with_kind(SpanKind::Server)
	.start_with_context(tracer, &rq_ctx.context);
	let card = relay
		.fetch_agent_card(host, &rq_ctx, &target)
		.await
		.map_err(|_e| StatusCode::INTERNAL_SERVER_ERROR)?;

	app.metrics.clone().record(
		&metrics::AgentCall {
			agent: target.to_string(),
			method: "agent_card".to_string(),
		},
		(),
	);
	Ok(Json(card))
}

async fn agent_call_handler(
	State(app): State<App>,
	ConnectInfo(connection): ConnectInfo<proxyprotocol::Address>,
	Path(target): Path<String>,
	claims: Option<rbac::Claims>,
	headers: HeaderMap,
	// TODO: needs to be generic task
	Json(request): Json<a2a_sdk::A2aRequest>,
) -> anyhow::Result<
	AxumEither<
		Sse<impl Stream<Item = anyhow::Result<Event, axum::Error>>>,
		Json<a2a_sdk::JsonRpcMessage>,
	>,
	StatusCode,
> {
	tracing::info!("new agent call");
	let relay = a2a::relay::Relay::new(
		app.state.clone(),
		app.metrics.clone(),
		app.listener_name.clone(),
	);
	let connection_id = connection.identity.clone().map(|i| i.to_string());
	let claims = rbac::Identity::new(claims, connection_id);
	let context = trcng::extract_context_from_request(&headers);
	let rq_ctx = relay::RqCtx::new(claims, context);

	let attrs = vec![
		opentelemetry::KeyValue::new("agent", target.clone()),
		opentelemetry::KeyValue::new("method", request.method()),
		opentelemetry::KeyValue::new("request.id", request.id()),
		opentelemetry::KeyValue::new(
			"session.id",
			request.session_id().unwrap_or("none".to_string()),
		),
	];
	let tracer = trcng::get_tracer();
	let mut span = trcng::start_span_with_attributes(request.method(), &rq_ctx.identity, attrs)
		.with_kind(SpanKind::Server)
		.start_with_context(tracer, &rq_ctx.context);

	app.metrics.clone().record(
		&metrics::AgentCall {
			agent: target.to_string(),
			method: request.method().to_string(),
		},
		(),
	);
	let rx = relay
		.proxy_request(request, &rq_ctx, target)
		.await
		.map_err(|_e| StatusCode::INTERNAL_SERVER_ERROR)?;

	// TODO: use cancellation token
	match rx {
		a2a::relay::Response::Streaming(rx) => {
			let stream = rx
				.map(move |i| {
					if let Some(resp) = i.response() {
						span.add_event(
							"received response",
							vec![opentelemetry::KeyValue::new(
								"request.id",
								resp.id().unwrap_or("none".to_string()),
							)],
						)
					}
					i
				})
				.map(|message| Event::default().json_data(&message));
			Ok(AxumEither::Left(Sse::new(stream)))
		},
		a2a::relay::Response::Single(item) => Ok(AxumEither::Right(Json(item))),
	}
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
pub enum AxumEither<L, R> {
	Left(L),
	Right(R),
}

impl<L, R> IntoResponse for AxumEither<L, R>
where
	L: IntoResponse,
	R: IntoResponse,
{
	fn into_response(self) -> Response {
		match self {
			Self::Left(l) => l.into_response(),
			Self::Right(r) => r.into_response(),
		}
	}
}
