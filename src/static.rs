use std::sync::Arc;
use tracing::{debug, info, trace};

use crate::inbound;
use crate::proto::agentproxy::dev::a2a::target::Target as XdsA2aTarget;
use crate::proto::agentproxy::dev::listener::Listener as XdsListener;
use crate::proto::agentproxy::dev::mcp::target::Target as XdsMcpTarget;
use crate::xds::XdsStore as ProxyState;
#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticConfig {
	#[serde(default)]
	targets: Targets,
	#[serde(default)]
	pub listeners: Vec<XdsListener>,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Targets {
	#[serde(default)]
	pub mcp: Vec<XdsMcpTarget>,
	#[serde(default)]
	pub a2a: Vec<XdsA2aTarget>,
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
		let num_mcp_targets = cfg.targets.mcp.len();
		for target in cfg.targets.mcp.clone() {
			trace!("inserting target {}", &target.name);
			state
				.mcp_targets
				.insert(target)
				.expect("failed to insert target into store");
		}
		let num_a2a_targets = cfg.targets.a2a.len();
		for target in cfg.targets.a2a.clone() {
			trace!("inserting target {}", &target.name);
			state
				.a2a_targets
				.insert(target)
				.expect("failed to insert target into store");
		}
		info!(%num_mcp_targets, %num_a2a_targets, "local config initialized");
	}
	listener_manager.run(ct).await
}
