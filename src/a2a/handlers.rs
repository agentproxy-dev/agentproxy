use crate::a2a::relay;
use crate::sse::AuthError;
use crate::{a2a, authn, proxyprotocol, rbac};
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
use http::StatusCode;
use http::request::Parts;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct App {
	state: Arc<tokio::sync::RwLock<crate::xds::XdsStore>>,
	metrics: Arc<crate::relay::metrics::Metrics>,
	authn: Arc<RwLock<Option<authn::JwtAuthenticator>>>,
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
		metrics: Arc<crate::relay::metrics::Metrics>,
		authn: Arc<RwLock<Option<authn::JwtAuthenticator>>>,
		ct: tokio_util::sync::CancellationToken,
	) -> Self {
		Self {
			state,
			metrics,
			authn,
			_ct: ct,
		}
	}
	pub fn router(&self) -> Router {
		Router::new()
			.route("/{target}/.well-known/agent.json", get(agent_card_handler))
			.route("/{target}", post(agent_call_handler))
			.with_state(self.clone())
	}
}

async fn agent_card_handler(
	State(app): State<App>,
	Path(target): Path<String>,
	Host(host): Host,
	ConnectInfo(connection): ConnectInfo<proxyprotocol::Address>,
	claims: Option<rbac::Claims>,
) -> anyhow::Result<Json<AgentCard>, StatusCode> {
	tracing::info!("new agent card request");
	let relay = relay::Relay::new(app.state.clone(), app.metrics.clone());
	let connection_id = connection.identity.clone().map(|i| i.to_string());
	let claims = rbac::Identity::new(claims.map(|c| c.0), connection_id);
	let card = relay
		.fetch_agent_card(host, claims, &target)
		.await
		.map_err(|_e| StatusCode::INTERNAL_SERVER_ERROR)?;

	Ok(Json(card))
}

async fn agent_call_handler(
	State(app): State<App>,
	ConnectInfo(connection): ConnectInfo<proxyprotocol::Address>,
	Path(target): Path<String>,
	claims: Option<rbac::Claims>,
	// TODO: needs to be generic task
	Json(request): Json<a2a_sdk::A2aRequest>,
) -> anyhow::Result<
	AxumEither<
		Sse<impl Stream<Item = anyhow::Result<Event, axum::Error>>>,
		Json<a2a_sdk::ClientJsonRpcMessage>,
	>,
	StatusCode,
> {
	tracing::info!("new agent call");
	let relay = a2a::relay::Relay::new(app.state.clone(), app.metrics.clone());
	let connection_id = connection.identity.clone().map(|i| i.to_string());
	let claims = rbac::Identity::new(claims.map(|c| c.0), connection_id);
	let rx = relay
		.proxy(request, claims, target)
		.await
		.map_err(|_e| StatusCode::INTERNAL_SERVER_ERROR)?;

	// TODO: use cancellation token
	match rx {
		a2a::relay::Response::Streaming(rx) => {
			let stream = rx.map(|message| Event::default().json_data(&message));
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
