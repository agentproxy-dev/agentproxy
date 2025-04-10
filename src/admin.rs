use std::sync::Arc;

use crate::proto::mcpproxy::dev::rbac::Config as Rbac;
use crate::proto::mcpproxy::dev::target::Target;
use crate::xds::XdsStore;
use axum::{
	Json, Router,
	extract::{Path, State},
	http::StatusCode,
	routing::get,
};
use tracing::error;
#[derive(Clone)]
pub struct App {
	state: Arc<tokio::sync::RwLock<XdsStore>>,
}

impl App {
	pub fn new(state: Arc<tokio::sync::RwLock<XdsStore>>) -> Self {
		Self { state }
	}
	pub fn router(&self) -> Router {
		Router::new()
			.route(
				"/targets",
				get(targets_list_handler).post(targets_create_handler),
			)
			.route(
				"/targets/{name}",
				get(targets_get_handler).delete(targets_delete_handler),
			)
			.route("/rbac", get(rbac_handler).post(rbac_create_handler))
			.route(
				"/rbac/{name}",
				get(rbac_get_handler).delete(rbac_delete_handler),
			)
			.route("/listeners", get(listener_handler))
			.with_state(self.clone())
	}
}

/// GET /targets  List all targets
/// GET /targets/:name  Get a target by name
/// POST /targets  Create/update a target
/// DELETE /targets/:name  Delete a target
///
/// GET /rbac  List all rbac policies
/// GET /rbac/:name  Get a rbac policy by name
/// POST /rbac  Create/update a rbac policy
/// DELETE /rbac/:name  Delete a rbac policy
///
/// GET /listeners  List all listeners
/// GET /listener/:name  Get a listener by name
/// POST /listeners  Create/update a listener
/// DELETE /listeners/:name  Delete a listener

async fn targets_list_handler(State(app): State<App>) -> Result<String, StatusCode> {
	let targets = app.state.read().await.targets.clone();
	match serde_json::to_string(&targets) {
		Ok(json_targets) => Ok(json_targets),
		Err(e) => {
			error!("error serializing targets: {:?}", e);
			Err(StatusCode::INTERNAL_SERVER_ERROR)
		},
	}
}

async fn targets_get_handler(
	State(app): State<App>,
	Path(name): Path<String>,
) -> Result<Json<Target>, StatusCode> {
	let state = app.state.read().await;
	let target = state.targets.get_proto(&name);
	match target {
		Some(target) => Ok(Json(target.clone())),
		None => Err(StatusCode::NOT_FOUND),
	}
}

async fn targets_delete_handler(
	State(app): State<App>,
	Path(name): Path<String>,
) -> Result<(), StatusCode> {
	let mut state = app.state.write().await;
	state.targets.remove(&name);
	Ok(())
}

async fn targets_create_handler(
	State(app): State<App>,
	Json(target): Json<Target>,
) -> Result<(), StatusCode> {
	let mut state = app.state.write().await;
	match state.targets.insert(target) {
		Ok(_) => Ok(()),
		Err(e) => {
			error!("error inserting target into store: {:?}", e);
			Err(StatusCode::INTERNAL_SERVER_ERROR)
		},
	}
}

async fn rbac_handler(State(app): State<App>) -> Result<String, StatusCode> {
	let rbac = app.state.read().await.policies.clone();
	match serde_json::to_string(&rbac) {
		Ok(json_rbac) => Ok(json_rbac),
		Err(e) => {
			error!("error serializing rbac: {:?}", e);
			Err(StatusCode::INTERNAL_SERVER_ERROR)
		},
	}
}

async fn rbac_get_handler(
	State(app): State<App>,
	Path(name): Path<String>,
) -> Result<Json<Rbac>, StatusCode> {
	let state = app.state.read().await;
	let rbac = state.policies.get_proto(&name);
	match rbac {
		Some(rbac) => Ok(Json(rbac.clone())),
		None => Err(StatusCode::NOT_FOUND),
	}
}

async fn rbac_create_handler(
	State(app): State<App>,
	Json(rbac): Json<Rbac>,
) -> Result<(), StatusCode> {
	let mut state = app.state.write().await;
	match state.policies.insert(rbac) {
		Ok(_) => Ok(()),
		Err(e) => {
			error!("error inserting rbac into store: {:?}", e);
			Err(StatusCode::INTERNAL_SERVER_ERROR)
		},
	}
}

async fn rbac_delete_handler(
	State(app): State<App>,
	Path(name): Path<String>,
) -> Result<(), StatusCode> {
	let mut state = app.state.write().await;
	state.policies.remove(&name);
	Ok(())
}

async fn listener_handler(State(app): State<App>) -> Result<String, StatusCode> {
	let listener = app.state.read().await.listener.clone();
	match serde_json::to_string(&listener) {
		Ok(json_listener) => Ok(json_listener),
		Err(e) => {
			error!("error serializing listener: {:?}", e);
			Err(StatusCode::INTERNAL_SERVER_ERROR)
		},
	}
}
