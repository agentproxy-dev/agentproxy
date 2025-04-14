use std::sync::Arc;
use tracing::{debug, info, trace};

use crate::inbound;
use crate::proto::aidp::dev::listener::Listener as XdsListener;
use crate::proto::aidp::dev::mcp::target::Target as XdsTarget;
use crate::xds::XdsStore as ProxyState;
#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StaticConfig {
	#[serde(default)]
	pub targets: Vec<XdsTarget>,
	#[serde(default)]
	pub listener: XdsListener,
}

pub async fn run_local_client(
	cfg: &StaticConfig,
	state_ref: Arc<tokio::sync::RwLock<ProxyState>>,
	mut listener_manager: inbound::ListenerManager,
	ct: tokio_util::sync::CancellationToken,
) -> Result<(), anyhow::Error> {
	debug!(
		"load local config: {}",
		serde_yaml::to_string(&cfg).unwrap_or_default()
	);
	// Clear the state
	let state_clone = state_ref.clone();
	{
		let mut state = state_clone.write().await;
		let num_targets = cfg.targets.len();
		for target in cfg.targets.clone() {
			trace!("inserting target {}", &target.name);
			state
				.mcp_targets
				.insert(target)
				.expect("failed to insert target into store");
		}
		info!(%num_targets, "local config initialized");
	}

	listener_manager.run(ct).await
}
