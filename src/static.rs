use std::sync::Arc;
use tracing::{debug, info, trace};

use crate::inbound;
use crate::proto::aidp::dev::mcp::listener::Listener as XdsListener;
use crate::proto::aidp::dev::mcp::rbac::{Rule as XdsRule, RuleSet as XdsRuleSet};
use crate::proto::aidp::dev::mcp::target::Target as XdsTarget;
use crate::relay;
use crate::trcng;
use crate::xds::XdsStore as ProxyState;
#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StaticConfig {
	#[serde(default)]
	pub targets: Vec<XdsTarget>,
	#[serde(default)]
	pub policies: Vec<XdsRule>,
	#[serde(default)]
	pub listener: XdsListener,

	pub tracing: Option<trcng::Config>,
}

pub async fn run_local_client(
	cfg: &StaticConfig,
	state_ref: Arc<tokio::sync::RwLock<ProxyState>>,
	metrics: Arc<relay::metrics::Metrics>,
	ct: tokio_util::sync::CancellationToken,
) -> Result<(), crate::inbound::ServingError> {
	debug!(
		"load local config: {}",
		serde_yaml::to_string(&cfg).unwrap_or_default()
	);
	// Clear the state
	let state_clone = state_ref.clone();
	{
		let mut state = state_clone.write().await;
		let num_targets = cfg.targets.len();
		let num_policies = cfg.policies.len();
		for target in cfg.targets.clone() {
			trace!("inserting target {}", &target.name);
			state
				.targets
				.insert(target)
				.expect("failed to insert target into store");
		}
		if !cfg.policies.is_empty() {
			let rule_set = XdsRuleSet {
				name: "test".to_string(),
				namespace: "test".to_string(),
				rules: cfg.policies.clone(),
			};
			state
				.policies
				.insert(rule_set)
				.expect("failed to insert rule set into store");
		}
		info!(%num_targets, %num_policies, "local config initialized");
	}

	let listener = inbound::Listener::from_xds(cfg.listener.clone())
		.await
		.unwrap();

	listener.listen(state_ref, metrics, ct).await
}
