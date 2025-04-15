use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::registry::Registry;

use crate::mtrcs::Recorder;

pub struct Metrics {
	agent_calls: Family<AgentCall, Counter>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct AgentCall {
	pub agent: String,
	pub method: String,
}

impl Metrics {
	pub fn new(registry: &mut Registry) -> Self {
		let agent_calls = Family::default();
		registry.register(
			"agent_calls",
			"The total number of agent calls",
			agent_calls.clone(),
		);

		Self { agent_calls }
	}
}

impl Recorder<&AgentCall, ()> for Metrics {
	fn record(&self, call: &AgentCall, _: ()) {
		self.agent_calls.get_or_create(call).inc();
	}
}
