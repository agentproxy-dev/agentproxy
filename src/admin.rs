use std::sync::Arc;

use crate::xds::XdsStore;
use axum::{extract::{Path, State}, http::StatusCode, routing::get, Json, Router};
use tracing::error;
use crate::proto::mcpproxy::dev::target::Target;
#[derive(Clone)]
pub struct App {
	state: Arc<std::sync::RwLock<XdsStore>>,
}

impl App {
	pub fn new(state: Arc<std::sync::RwLock<XdsStore>>) -> Self {
		Self { state }
	}
	pub fn router(&self) -> Router {
		Router::new()
			.route("/targets", get(targets_list_handler))
			.route("/rbac", get(rbac_handler))
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
	let targets = app.state.read().unwrap().targets.clone();
	match serde_json::to_string(&targets) {
		Ok(json_targets) => Ok(json_targets),
		Err(e) => {
			error!("error serializing targets: {:?}", e);
			Err(StatusCode::INTERNAL_SERVER_ERROR)
		},
	}
}

// async fn targets_get_handler(State(app): State<App>, Path(name): Path<String>) -> Result<Json<Target>, StatusCode> {
// 	let target = app.state.read().unwrap().targets.get(&name);
// 	match target {
// 		Some(target) => Ok(Json(target.clone())),
// 		None => Err(StatusCode::NOT_FOUND),
// 	}
// }

async fn rbac_handler(State(app): State<App>) -> Result<String, StatusCode> {
	let rbac = app.state.read().unwrap().policies.clone();
	match serde_json::to_string(&rbac) {
		Ok(json_rbac) => Ok(json_rbac),
		Err(e) => {
			error!("error serializing rbac: {:?}", e);
			Err(StatusCode::INTERNAL_SERVER_ERROR)
		},
	}
}

async fn listener_handler(State(app): State<App>) -> Result<String, StatusCode> {
	let listener = app.state.read().unwrap().listener.clone();
	match serde_json::to_string(&listener) {
		Ok(json_listener) => Ok(json_listener),
		Err(e) => {
			error!("error serializing listener: {:?}", e);
			Err(StatusCode::INTERNAL_SERVER_ERROR)
		},
	}
}
