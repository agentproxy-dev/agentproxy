use std::sync::Arc;

use crate::proto::agentproxy::dev::listener::Listener;
use crate::proto::agentproxy::dev::mcp::target::Target as McpTarget;
use crate::xds::XdsStore;
use crate::{inbound, proto::agentproxy::dev::a2a::target::Target as A2aTarget};
use axum::{
	Json, Router,
	extract::{Path, State},
	http::StatusCode,
	response::{IntoResponse, Response},
	routing::get,
};
use serde::{Deserialize, Serialize};
use tracing::error;

pub use ui::add_cors_layer;

#[derive(Clone)]
pub(crate) struct App {
	state: Arc<tokio::sync::RwLock<XdsStore>>,
}

impl App {
	fn new(state: Arc<tokio::sync::RwLock<XdsStore>>) -> Self {
		Self { state }
	}
	fn router(&self) -> Router {
		let router = Router::new()
			// Redirect to the UI
			.route(
				"/targets/mcp",
				get(targets_mcp_list_handler).post(targets_mcp_create_handler),
			)
			.route(
				"/targets/mcp/{name}",
				get(targets_mcp_get_handler).delete(targets_mcp_delete_handler),
			)
			.route(
				"/targets/a2a",
				get(targets_a2a_list_handler).post(targets_a2a_create_handler),
			)
			.route(
				"/targets/a2a/{name}",
				get(targets_a2a_get_handler).delete(targets_a2a_delete_handler),
			)
			.route(
				"/listeners/{name}/targets",
				get(listener_targets_list_handler),
			)
			.route(
				"/listeners",
				get(listener_list_handler).post(listener_create_handler),
			)
			.route(
				"/listeners/{name}",
				get(listener_get_handler).delete(listener_delete_handler),
			);

		let router = ui::add_ui_layer(router);

		let router = router.layer(ui::add_cors_layer());

		router.with_state(self.clone())
	}
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
	pub host: String,
	pub port: u16,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			host: "127.0.0.1".to_string(),
			port: 19000,
		}
	}
}

pub async fn start(
	state: Arc<tokio::sync::RwLock<XdsStore>>,
	ct: tokio_util::sync::CancellationToken,
	cfg: Option<Config>,
) -> Result<(), std::io::Error> {
	let cfg = cfg.unwrap_or_default();
	let listener = tokio::net::TcpListener::bind(format!("{}:{}", cfg.host, cfg.port)).await?;
	let app = App::new(state);
	let router = app.router();
	axum::serve(listener, router)
		.with_graceful_shutdown(async move {
			ct.cancelled().await;
		})
		.await
}

/// GET /targets/mcp  List all MCP targets
/// GET /targets/mcp/{name}  Get a MCP target by name
/// POST /targets/mcp  Create/update a MCP target
/// DELETE /targets/mcp/{name}  Delete a MCP target
///
/// GET /targets/a2a  List all A2A targets
/// GET /targets/a2a/{name}  Get an A2A target by name
/// POST /targets/a2a  Create/update an A2A target
/// DELETE /targets/a2a/{name}  Delete an A2A target
///
/// GET /listeners  List all listeners
/// GET /listeners/{name}  Get a listener by name
/// POST /listeners  Create/update a listener
/// DELETE /listeners/{name}  Delete a listener
///
/// GET /listeners/{name}/targets  List all targets for a listener

#[derive(Debug, Serialize, Deserialize)]
struct ErrorResponse {
	message: String,
}

impl IntoResponse for ErrorResponse {
	fn into_response(self) -> Response {
		(StatusCode::INTERNAL_SERVER_ERROR, Json(self)).into_response()
	}
}

async fn targets_a2a_list_handler(
	State(app): State<App>,
) -> Result<String, (StatusCode, impl IntoResponse)> {
	let targets = app.state.read().await.a2a_targets.clone();
	match serde_json::to_string(&targets) {
		Ok(json_targets) => Ok(json_targets),
		Err(e) => {
			error!("error serializing targets: {:?}", e);
			Err((
				StatusCode::INTERNAL_SERVER_ERROR,
				ErrorResponse {
					message: "error serializing targets".to_string(),
				},
			))
		},
	}
}

async fn targets_a2a_create_handler(
	State(app): State<App>,
	Json(target): Json<A2aTarget>,
) -> Result<(), (StatusCode, impl IntoResponse)> {
	let mut state = app.state.write().await;
	match state.a2a_targets.insert(target) {
		Ok(_) => Ok(()),
		Err(e) => {
			error!("error inserting target into store: {:?}", e);
			Err((
				StatusCode::INTERNAL_SERVER_ERROR,
				ErrorResponse {
					message: "error inserting target into store".to_string(),
				},
			))
		},
	}
}

async fn targets_a2a_get_handler(
	State(app): State<App>,
	Path(name): Path<String>,
) -> Result<Json<A2aTarget>, (StatusCode, impl IntoResponse)> {
	let state = app.state.read().await;
	let target = state.a2a_targets.get_proto(&name);
	match target {
		Some(target) => Ok(Json(target.clone())),
		None => Err((
			StatusCode::NOT_FOUND,
			ErrorResponse {
				message: "target not found".to_string(),
			},
		)),
	}
}

async fn targets_a2a_delete_handler(
	State(app): State<App>,
	Path(name): Path<String>,
) -> Result<(), (StatusCode, impl IntoResponse)> {
	let mut state = app.state.write().await;
	match state.a2a_targets.remove(&name) {
		Ok(_) => Ok(()),
		Err(e) => {
			error!("error removing target from store: {:?}", e);
			Err((
				StatusCode::INTERNAL_SERVER_ERROR,
				ErrorResponse {
					message: "error removing target from store".to_string(),
				},
			))
		},
	}
}

async fn targets_mcp_list_handler(
	State(app): State<App>,
) -> Result<String, (StatusCode, impl IntoResponse)> {
	let targets = app.state.read().await.mcp_targets.clone();
	match serde_json::to_string(&targets) {
		Ok(json_targets) => Ok(json_targets),
		Err(e) => {
			error!("error serializing targets: {:?}", e);
			Err((
				StatusCode::INTERNAL_SERVER_ERROR,
				ErrorResponse {
					message: "error serializing targets".to_string(),
				},
			))
		},
	}
}

async fn targets_mcp_get_handler(
	State(app): State<App>,
	Path(name): Path<String>,
) -> Result<Json<McpTarget>, (StatusCode, impl IntoResponse)> {
	let state = app.state.read().await;
	let target = state.mcp_targets.get_proto(&name);
	match target {
		Some(target) => Ok(Json(target.clone())),
		None => Err((
			StatusCode::NOT_FOUND,
			ErrorResponse {
				message: "target not found".to_string(),
			},
		)),
	}
}

async fn targets_mcp_delete_handler(
	State(app): State<App>,
	Path(name): Path<String>,
) -> Result<(), (StatusCode, impl IntoResponse)> {
	let mut state = app.state.write().await;
	match state.mcp_targets.remove(&name) {
		Ok(_) => Ok(()),
		Err(e) => {
			error!("error removing target from store: {:?}", e);
			Err((
				StatusCode::INTERNAL_SERVER_ERROR,
				ErrorResponse {
					message: "error removing target from store".to_string(),
				},
			))
		},
	}
}

async fn targets_mcp_create_handler(
	State(app): State<App>,
	Json(target): Json<McpTarget>,
) -> Result<(), (StatusCode, impl IntoResponse)> {
	let mut state = app.state.write().await;
	match state.mcp_targets.insert(target) {
		Ok(_) => Ok(()),
		Err(e) => {
			error!("error inserting target into store: {:?}", e);
			Err((
				StatusCode::INTERNAL_SERVER_ERROR,
				ErrorResponse {
					message: "error inserting target into store".to_string(),
				},
			))
		},
	}
}

async fn listener_targets_list_handler(
	State(app): State<App>,
	Path(name): Path<String>,
) -> Result<String, (StatusCode, impl IntoResponse)> {
	let state = app.state.read().await;
	let listener = state.listeners.get(&name);
	if listener.is_none() {
		return Err((
			StatusCode::NOT_FOUND,
			ErrorResponse {
				message: "listener not found".to_string(),
			},
		));
	}
	let listener = listener.unwrap();
	let marshalled = match listener.spec {
		inbound::ListenerType::A2a(_) => {
			let targets = state
				.a2a_targets
				.iter(&listener.name)
				.map(|(_, target)| target.0.clone())
				.collect::<Vec<_>>();
			serde_json::to_string(&targets)
		},
		_ => {
			let targets = state
				.mcp_targets
				.iter(&listener.name)
				.map(|(_, target)| target.0.clone())
				.collect::<Vec<_>>();
			serde_json::to_string(&targets)
		},
	};

	match marshalled {
		Ok(json_targets) => Ok(json_targets),
		Err(e) => {
			error!("error serializing targets: {:?}", e);
			Err((
				StatusCode::INTERNAL_SERVER_ERROR,
				ErrorResponse {
					message: "error serializing targets".to_string(),
				},
			))
		},
	}
}
async fn listener_list_handler(
	State(app): State<App>,
) -> Result<String, (StatusCode, impl IntoResponse)> {
	let listeners = app.state.read().await.listeners.clone();
	match serde_json::to_string(&listeners) {
		Ok(json_listeners) => Ok(json_listeners),
		Err(e) => {
			error!("error serializing listener: {:?}", e);
			Err((
				StatusCode::INTERNAL_SERVER_ERROR,
				ErrorResponse {
					message: "error serializing listener".to_string(),
				},
			))
		},
	}
}

async fn listener_create_handler(
	State(app): State<App>,
	Json(listener): Json<Listener>,
) -> Result<(), (StatusCode, impl IntoResponse)> {
	let mut state = app.state.write().await;
	match state.listeners.insert(listener).await {
		Ok(_) => Ok(()),
		Err(e) => {
			error!("error inserting listener into store: {:?}", e);
			Err((
				StatusCode::INTERNAL_SERVER_ERROR,
				ErrorResponse {
					message: "error inserting listener into store".to_string(),
				},
			))
		},
	}
}

async fn listener_get_handler(
	State(app): State<App>,
	Path(name): Path<String>,
) -> Result<Json<Listener>, (StatusCode, impl IntoResponse)> {
	let state = app.state.read().await;
	let listener = state.listeners.get_proto(&name);
	match listener {
		Some(listener) => Ok(Json(listener.clone())),
		None => Err((
			StatusCode::NOT_FOUND,
			ErrorResponse {
				message: "listener not found".to_string(),
			},
		)),
	}
}

async fn listener_delete_handler(
	State(app): State<App>,
	Path(name): Path<String>,
) -> Result<(), (StatusCode, impl IntoResponse)> {
	let mut state = app.state.write().await;
	match state.listeners.remove(&name).await {
		Ok(_) => Ok(()),
		Err(e) => {
			error!("error removing listener from store: {:?}", e);
			Err((
				StatusCode::INTERNAL_SERVER_ERROR,
				ErrorResponse {
					message: "error removing listener from store".to_string(),
				},
			))
		},
	}
}

mod ui {
	use super::*;
	use crate::admin::App;
	use http::{
		HeaderName, HeaderValue, Method,
		header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE},
	};
	use std::time::Duration;
	use tower_http::cors::CorsLayer;

	pub fn add_cors_layer() -> CorsLayer {
		CorsLayer::new()
			.allow_origin(
				[
					"http://0.0.0.0:3000",
					"http://localhost:3000",
					"http://127.0.0.1:3000",
					"http://0.0.0.0:19000",
					"http://127.0.0.1:19000",
					"http://localhost:19000",
				]
				.map(|origin| origin.parse::<HeaderValue>().unwrap()),
			)
			.allow_headers([
				CONTENT_TYPE,
				AUTHORIZATION,
				HeaderName::from_static("x-requested-with"),
			])
			.allow_methods([
				Method::GET,
				Method::POST,
				Method::PUT,
				Method::DELETE,
				Method::OPTIONS,
			])
			.allow_credentials(true)
			.expose_headers([CONTENT_TYPE, CONTENT_LENGTH])
			.max_age(Duration::from_secs(3600))
	}

	#[cfg(feature = "ui")]
	use {
		axum::{Router, response::Redirect},
		include_dir::{Dir, include_dir},
		tower_serve_static::ServeDir,
	};

	#[cfg(feature = "ui")]
	lazy_static::lazy_static! {
		static ref ASSETS_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/ui/out");
	}

	#[cfg(feature = "ui")]
	pub fn add_ui_layer(router: Router<App>) -> Router<App> {
		let service = ServeDir::new(&ASSETS_DIR);
		router
			.nest_service("/ui", service)
			.route("/", get(|| async { Redirect::permanent("/ui") }))
	}

	#[cfg(not(feature = "ui"))]
	pub fn add_ui_layer(router: Router<App>) -> Router<App> {
		router
	}
}
