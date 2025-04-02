use std::sync::Arc;
use tracing::{debug, info, trace};

use crate::inbound::Listener;
use crate::outbound::Target;
use crate::rbac;
use crate::relay;
use crate::xds::XdsStore as ProxyState;

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StaticConfig {
	#[serde(default)]
	pub targets: Vec<Target>,
	#[serde(default)]
	pub policies: Vec<rbac::Rule>,
	#[serde(default)]
	pub listener: Listener,
}

pub async fn run_local_client(
	cfg: &StaticConfig,
	state_ref: Arc<std::sync::RwLock<ProxyState>>,
	metrics: Arc<relay::metrics::Metrics>,
) -> Result<(), crate::inbound::ServingError> {
	debug!(
		"load local config: {}",
		serde_yaml::to_string(&cfg).unwrap_or_default()
	);
	// Clear the state
	let state_clone = state_ref.clone();
	{
		let mut state = state_clone.write().unwrap();
		state.targets.clear();
		state.policies.clear();
		let num_targets = cfg.targets.len();
		let num_policies = cfg.policies.len();
		for target in cfg.targets.clone() {
			trace!("inserting target {}", &target.name);
			state.targets.insert(target);
		}
		let rule_set = rbac::RuleSet::new("test".to_string(), "test".to_string(), cfg.policies.clone());
		state.policies.insert(rule_set);
		info!(%num_targets, %num_policies, "local config initialized");
	}

	cfg.listener.listen(state_ref, metrics).await

	// serve_static_listener(cfg.listener.clone(), state_ref, metrics).await
}
