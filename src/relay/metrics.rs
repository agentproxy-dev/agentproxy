use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::registry::Registry;

use crate::mtrcs::Recorder;

pub struct Metrics {
	tool_calls: Family<ToolCall, Counter>,
	tool_call_errors: Family<ToolCallError, Counter>,
	list_calls: Family<ListCall, Counter>,
	read_resource_calls: Family<GetResourceCall, Counter>,
	get_prompt_calls: Family<GetPromptCall, Counter>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct GetResourceCall {
	pub server: String,
	pub uri: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct GetPromptCall {
	pub server: String,
	pub name: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ListCall {
	pub resource_type: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ToolCall {
	pub server: String,
	pub name: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ToolCallError {
	pub server: String,
	pub name: String,
	pub error_type: String,
}

impl Metrics {
	pub fn new(registry: &mut Registry) -> Self {
		let tool_calls = Family::default();
		registry.register(
			"tool_calls",
			"The total number of tool calls",
			tool_calls.clone(),
		);

		let tool_call_errors = Family::default();
		registry.register(
			"tool_call_errors",
			"The total number of tool call errors",
			tool_call_errors.clone(),
		);

		let list_calls = Family::default();
		registry.register(
			"list_calls",
			"The total number of list calls",
			list_calls.clone(),
		);

		let read_resource_calls = Family::default();
		registry.register(
			"read_resource_calls",
			"The total number of read resource calls",
			read_resource_calls.clone(),
		);

		let get_prompt_calls = Family::default();
		registry.register(
			"get_prompt_calls",
			"The total number of get prompt calls",
			get_prompt_calls.clone(),
		);

		Self {
			tool_calls,
			tool_call_errors,
			list_calls,
			read_resource_calls,
			get_prompt_calls,
		}
	}
}

impl Recorder<ToolCall, ()> for Metrics {
	fn record(&self, tool_call: &ToolCall, _: ()) {
		self.tool_calls.get_or_create(tool_call).inc();
	}
}

impl Recorder<ToolCallError, ()> for Metrics {
	fn record(&self, tool_call_error: &ToolCallError, _: ()) {
		self.tool_call_errors.get_or_create(tool_call_error).inc();
	}
}

impl Recorder<ListCall, ()> for Metrics {
	fn record(&self, list_call: &ListCall, _: ()) {
		self.list_calls.get_or_create(list_call).inc();
	}
}

impl Recorder<GetResourceCall, ()> for Metrics {
	fn record(&self, get_resource_call: &GetResourceCall, _: ()) {
		self
			.read_resource_calls
			.get_or_create(get_resource_call)
			.inc();
	}
}

impl Recorder<GetPromptCall, ()> for Metrics {
	fn record(&self, get_prompt_call: &GetPromptCall, _: ()) {
		self.get_prompt_calls.get_or_create(get_prompt_call).inc();
	}
}
