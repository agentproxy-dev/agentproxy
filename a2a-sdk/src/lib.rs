#![allow(clippy::redundant_closure_call)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::match_single_binding)]
#![allow(clippy::clone_on_copy)]

mod jsonrpc;

use crate::jsonrpc::*;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum JsonRpcMessage {
	Request(JsonRpcRequest<A2aRequest>),
	Response(JsonRpcResponse<A2aResponse>),
}

impl JsonRpcMessage {
	pub fn response(&self) -> Option<&A2aResponse> {
		match self {
			JsonRpcMessage::Request(_) => None,
			JsonRpcMessage::Response(resp) => Some(&resp.result),
		}
	}
}

// TODO: this is not complete, add the rest
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum A2aRequest {
	SendTaskRequest(SendTaskRequest),
	SendSubscribeTaskRequest(SendSubscribeTaskRequest),
	GetTaskRequest(GetTaskRequest),
}

impl A2aRequest {
	pub fn method(&self) -> &'static str {
		match self {
			A2aRequest::SendTaskRequest(i) => i.method.as_string(),
			A2aRequest::SendSubscribeTaskRequest(i) => i.method.as_string(),
			A2aRequest::GetTaskRequest(i) => i.method.as_string(),
		}
	}
	pub fn id(&self) -> String {
		match self {
			A2aRequest::SendTaskRequest(i) => i.params.id.clone(),
			A2aRequest::SendSubscribeTaskRequest(i) => i.params.id.clone(),
			A2aRequest::GetTaskRequest(i) => i.params.id.clone(),
		}
	}
	pub fn session_id(&self) -> Option<String> {
		match self {
			A2aRequest::SendTaskRequest(i) => i.params.session_id.clone(),
			A2aRequest::SendSubscribeTaskRequest(i) => i.params.session_id.clone(),
			A2aRequest::GetTaskRequest(_) => None,
		}
	}
}

// TODO: this is not complete, add the rest
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum A2aResponse {
	SendTaskResponse(Option<Task>),
	SendTaskUpdateResponse(SendTaskStreamingResponseResult),
}

impl A2aResponse {
	pub fn id(&self) -> Option<String> {
		match self {
			A2aResponse::SendTaskResponse(i) => i.as_ref().map(|i| i.id.clone()),
			A2aResponse::SendTaskUpdateResponse(SendTaskStreamingResponseResult::Status(i)) => {
				Some(i.id.clone())
			},
			A2aResponse::SendTaskUpdateResponse(SendTaskStreamingResponseResult::Artifact(i)) => {
				Some(i.id.clone())
			},
			A2aResponse::SendTaskUpdateResponse(SendTaskStreamingResponseResult::None) => None,
		}
	}
}
impl From<SendTaskRequest> for A2aRequest {
	fn from(value: SendTaskRequest) -> Self {
		Self::SendTaskRequest(value)
	}
}
impl From<GetTaskRequest> for A2aRequest {
	fn from(value: GetTaskRequest) -> Self {
		Self::GetTaskRequest(value)
	}
}
// impl From<CancelTaskRequest> for A2aRequest {
// 	fn from(value: CancelTaskRequest) -> Self {
// 		Self::CancelTaskRequest(value)
// 	}
// }
// impl From<SetTaskPushNotificationRequest> for A2aRequest {
// 	fn from(value: SetTaskPushNotificationRequest) -> Self {
// 		Self::SetTaskPushNotificationRequest(value)
// 	}
// }
// impl From<GetTaskPushNotificationRequest> for A2aRequest {
// 	fn from(value: GetTaskPushNotificationRequest) -> Self {
// 		Self::GetTaskPushNotificationRequest(value)
// 	}
// }
// impl From<TaskResubscriptionRequest> for A2aRequest {
// 	fn from(value: TaskResubscriptionRequest) -> Self {
// 		Self::TaskResubscriptionRequest(value)
// 	}
// }

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct AgentAuthentication {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub credentials: Option<String>,
	pub schemes: Vec<String>,
}
impl From<&AgentAuthentication> for AgentAuthentication {
	fn from(value: &AgentAuthentication) -> Self {
		value.clone()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, Default)]
pub struct AgentCapabilities {
	#[serde(rename = "pushNotifications", default)]
	pub push_notifications: bool,
	#[serde(rename = "stateTransitionHistory", default)]
	pub state_transition_history: bool,
	#[serde(default)]
	pub streaming: bool,
}
impl From<&AgentCapabilities> for AgentCapabilities {
	fn from(value: &AgentCapabilities) -> Self {
		value.clone()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct AgentCard {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub authentication: Option<AgentAuthentication>,
	pub capabilities: AgentCapabilities,
	#[serde(
		rename = "defaultInputModes",
		default = "defaults::agent_card_default_input_modes"
	)]
	pub default_input_modes: Vec<String>,
	#[serde(
		rename = "defaultOutputModes",
		default = "defaults::agent_card_default_output_modes"
	)]
	pub default_output_modes: Vec<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	#[serde(
		rename = "documentationUrl",
		default,
		skip_serializing_if = "Option::is_none"
	)]
	pub documentation_url: Option<String>,
	pub name: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub provider: Option<AgentProvider>,
	pub skills: Vec<AgentSkill>,
	pub url: String,
	pub version: String,
}
impl From<&AgentCard> for AgentCard {
	fn from(value: &AgentCard) -> Self {
		value.clone()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct AgentProvider {
	pub organization: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub url: Option<String>,
}
impl From<&AgentProvider> for AgentProvider {
	fn from(value: &AgentProvider) -> Self {
		value.clone()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct AgentSkill {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub examples: Option<Vec<String>>,
	pub id: String,
	#[serde(
		rename = "inputModes",
		default,
		skip_serializing_if = "Option::is_none"
	)]
	pub input_modes: Option<Vec<String>>,
	pub name: String,
	#[serde(
		rename = "outputModes",
		default,
		skip_serializing_if = "Option::is_none"
	)]
	pub output_modes: Option<Vec<String>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub tags: Option<Vec<String>>,
}
impl From<&AgentSkill> for AgentSkill {
	fn from(value: &AgentSkill) -> Self {
		value.clone()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Artifact {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub append: Option<bool>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	#[serde(default)]
	pub index: i64,
	#[serde(rename = "lastChunk", default, skip_serializing_if = "Option::is_none")]
	pub last_chunk: Option<bool>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub metadata: Option<::serde_json::Map<String, ::serde_json::Value>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,
	pub parts: Vec<Part>,
}
impl From<&Artifact> for Artifact {
	fn from(value: &Artifact) -> Self {
		value.clone()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct AuthenticationInfo {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub credentials: Option<String>,
	pub schemes: Vec<String>,
}
impl From<&AuthenticationInfo> for AuthenticationInfo {
	fn from(value: &AuthenticationInfo) -> Self {
		value.clone()
	}
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct DataPart {
	pub data: ::serde_json::Map<String, ::serde_json::Value>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub metadata: Option<::serde_json::Map<String, ::serde_json::Value>>,
	#[serde(rename = "type", default = "defaults::data_part_type")]
	pub type_: String,
}
impl From<&DataPart> for DataPart {
	fn from(value: &DataPart) -> Self {
		value.clone()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, Default)]
pub struct FileContent {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub bytes: Option<String>,
	#[serde(rename = "mimeType", default, skip_serializing_if = "Option::is_none")]
	pub mime_type: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub uri: Option<String>,
}
impl From<&FileContent> for FileContent {
	fn from(value: &FileContent) -> Self {
		value.clone()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct FilePart {
	pub file: FileContent,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub metadata: Option<::serde_json::Map<String, ::serde_json::Value>>,
	#[serde(rename = "type", default = "defaults::file_part_type")]
	pub type_: String,
}
impl From<&FilePart> for FilePart {
	fn from(value: &FilePart) -> Self {
		value.clone()
	}
}

const_string!(GetTaskRequestMethod = "tasks/get");
pub type GetTaskRequest = Request<GetTaskRequestMethod, TaskQueryParams>;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum Id {
	Variant0(i64),
	Variant1(String),
	Variant2,
}
impl From<&Self> for Id {
	fn from(value: &Id) -> Self {
		value.clone()
	}
}
impl From<i64> for Id {
	fn from(value: i64) -> Self {
		Self::Variant0(value)
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct InternalError {
	pub code: i64,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub data: Option<::serde_json::Value>,
	pub message: String,
}
impl From<&InternalError> for InternalError {
	fn from(value: &InternalError) -> Self {
		value.clone()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct InvalidParamsError {
	pub code: i64,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub data: Option<::serde_json::Value>,
	pub message: String,
}
impl From<&InvalidParamsError> for InvalidParamsError {
	fn from(value: &InvalidParamsError) -> Self {
		value.clone()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct InvalidRequestError {
	pub code: i64,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub data: Option<::serde_json::Value>,
	pub message: String,
}
impl From<&InvalidRequestError> for InvalidRequestError {
	fn from(value: &InvalidRequestError) -> Self {
		value.clone()
	}
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Message {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub metadata: Option<::serde_json::Map<String, ::serde_json::Value>>,
	pub parts: Vec<Part>,
	pub role: Role,
}
impl From<&Message> for Message {
	fn from(value: &Message) -> Self {
		value.clone()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct MethodNotFoundError {
	pub code: i64,
	pub data: ::serde_json::Value,
	pub message: String,
}
impl From<&MethodNotFoundError> for MethodNotFoundError {
	fn from(value: &MethodNotFoundError) -> Self {
		value.clone()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum Part {
	TextPart(TextPart),
	FilePart(FilePart),
	DataPart(DataPart),
}
impl From<&Self> for Part {
	fn from(value: &Part) -> Self {
		value.clone()
	}
}
impl From<TextPart> for Part {
	fn from(value: TextPart) -> Self {
		Self::TextPart(value)
	}
}
impl From<FilePart> for Part {
	fn from(value: FilePart) -> Self {
		Self::FilePart(value)
	}
}
impl From<DataPart> for Part {
	fn from(value: DataPart) -> Self {
		Self::DataPart(value)
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct PushNotificationConfig {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub authentication: Option<AuthenticationInfo>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub token: Option<String>,
	pub url: String,
}
impl From<&PushNotificationConfig> for PushNotificationConfig {
	fn from(value: &PushNotificationConfig) -> Self {
		value.clone()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct PushNotificationNotSupportedError {
	pub code: i64,
	pub data: ::serde_json::Value,
	pub message: String,
}
impl From<&PushNotificationNotSupportedError> for PushNotificationNotSupportedError {
	fn from(value: &PushNotificationNotSupportedError) -> Self {
		value.clone()
	}
}
#[derive(
	serde::Deserialize, serde::Serialize, Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd,
)]
pub enum Role {
	#[serde(rename = "user")]
	User,
	#[serde(rename = "agent")]
	Agent,
}
impl From<&Self> for Role {
	fn from(value: &Role) -> Self {
		value.clone()
	}
}
impl Display for Role {
	fn fmt(&self, f: &mut Formatter<'_>) -> ::std::fmt::Result {
		match *self {
			Self::User => write!(f, "user"),
			Self::Agent => write!(f, "agent"),
		}
	}
}
impl ::std::str::FromStr for Role {
	type Err = self::error::ConversionError;
	fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
		match value {
			"user" => Ok(Self::User),
			"agent" => Ok(Self::Agent),
			_ => Err("invalid value".into()),
		}
	}
}
impl TryFrom<&str> for Role {
	type Error = self::error::ConversionError;
	fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
		value.parse()
	}
}
impl TryFrom<&String> for Role {
	type Error = self::error::ConversionError;
	fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
		value.parse()
	}
}
impl TryFrom<String> for Role {
	type Error = self::error::ConversionError;
	fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
		value.parse()
	}
}

const_string!(SendTaskRequestMethod = "tasks/send");
pub type SendTaskRequest = Request<SendTaskRequestMethod, TaskSendParams>;

const_string!(SendSubscribeTaskRequestMethod = "tasks/sendSubscribe");
pub type SendSubscribeTaskRequest = Request<SendSubscribeTaskRequestMethod, TaskSendParams>;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
#[serde(untagged)]
#[derive(Default)]
pub enum SendTaskStreamingResponseResult {
	Status(TaskStatusUpdateEvent),
	Artifact(TaskArtifactUpdateEvent),
	#[default]
	None,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Task {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub artifacts: Option<Vec<Artifact>>,
	pub id: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub metadata: Option<::serde_json::Map<String, ::serde_json::Value>>,
	#[serde(rename = "sessionId", default, skip_serializing_if = "Option::is_none")]
	pub session_id: Option<String>,
	pub status: TaskStatus,
}
impl From<&Task> for Task {
	fn from(value: &Task) -> Self {
		value.clone()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct TaskArtifactUpdateEvent {
	pub artifact: Artifact,
	pub id: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub metadata: Option<::serde_json::Map<String, ::serde_json::Value>>,
}
impl From<&TaskArtifactUpdateEvent> for TaskArtifactUpdateEvent {
	fn from(value: &TaskArtifactUpdateEvent) -> Self {
		value.clone()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct TaskIdParams {
	pub id: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub metadata: Option<::serde_json::Map<String, ::serde_json::Value>>,
}
impl From<&TaskIdParams> for TaskIdParams {
	fn from(value: &TaskIdParams) -> Self {
		value.clone()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct TaskNotCancelableError {
	pub code: i64,
	pub data: ::serde_json::Value,
	pub message: String,
}
impl From<&TaskNotCancelableError> for TaskNotCancelableError {
	fn from(value: &TaskNotCancelableError) -> Self {
		value.clone()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct TaskNotFoundError {
	pub code: i64,
	pub data: ::serde_json::Value,
	pub message: String,
}
impl From<&TaskNotFoundError> for TaskNotFoundError {
	fn from(value: &TaskNotFoundError) -> Self {
		value.clone()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct TaskPushNotificationConfig {
	pub id: String,
	#[serde(rename = "pushNotificationConfig")]
	pub push_notification_config: PushNotificationConfig,
}
impl From<&TaskPushNotificationConfig> for TaskPushNotificationConfig {
	fn from(value: &TaskPushNotificationConfig) -> Self {
		value.clone()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct TaskQueryParams {
	#[serde(
		rename = "historyLength",
		default,
		skip_serializing_if = "Option::is_none"
	)]
	pub history_length: Option<i64>,
	pub id: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub metadata: Option<::serde_json::Map<String, ::serde_json::Value>>,
}
impl From<&TaskQueryParams> for TaskQueryParams {
	fn from(value: &TaskQueryParams) -> Self {
		value.clone()
	}
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct TaskSendParams {
	#[serde(
		rename = "historyLength",
		default,
		skip_serializing_if = "Option::is_none"
	)]
	pub history_length: Option<i64>,
	pub id: String,
	pub message: Message,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub metadata: Option<::serde_json::Map<String, ::serde_json::Value>>,
	#[serde(
		rename = "pushNotification",
		default,
		skip_serializing_if = "Option::is_none"
	)]
	pub push_notification: Option<PushNotificationConfig>,
	#[serde(rename = "sessionId", default, skip_serializing_if = "Option::is_none")]
	pub session_id: Option<String>,
}
impl From<&TaskSendParams> for TaskSendParams {
	fn from(value: &TaskSendParams) -> Self {
		value.clone()
	}
}
#[derive(
	serde::Deserialize, serde::Serialize, Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd,
)]
pub enum TaskState {
	#[serde(rename = "submitted")]
	Submitted,
	#[serde(rename = "working")]
	Working,
	#[serde(rename = "input-required")]
	InputRequired,
	#[serde(rename = "completed")]
	Completed,
	#[serde(rename = "canceled")]
	Canceled,
	#[serde(rename = "failed")]
	Failed,
	#[serde(rename = "unknown")]
	Unknown,
}
impl From<&Self> for TaskState {
	fn from(value: &TaskState) -> Self {
		value.clone()
	}
}
impl Display for TaskState {
	fn fmt(&self, f: &mut Formatter<'_>) -> ::std::fmt::Result {
		match *self {
			Self::Submitted => write!(f, "submitted"),
			Self::Working => write!(f, "working"),
			Self::InputRequired => write!(f, "input-required"),
			Self::Completed => write!(f, "completed"),
			Self::Canceled => write!(f, "canceled"),
			Self::Failed => write!(f, "failed"),
			Self::Unknown => write!(f, "unknown"),
		}
	}
}
impl ::std::str::FromStr for TaskState {
	type Err = self::error::ConversionError;
	fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
		match value {
			"submitted" => Ok(Self::Submitted),
			"working" => Ok(Self::Working),
			"input-required" => Ok(Self::InputRequired),
			"completed" => Ok(Self::Completed),
			"canceled" => Ok(Self::Canceled),
			"failed" => Ok(Self::Failed),
			"unknown" => Ok(Self::Unknown),
			_ => Err("invalid value".into()),
		}
	}
}
impl TryFrom<&str> for TaskState {
	type Error = self::error::ConversionError;
	fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
		value.parse()
	}
}
impl TryFrom<&String> for TaskState {
	type Error = self::error::ConversionError;
	fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
		value.parse()
	}
}
impl TryFrom<String> for TaskState {
	type Error = self::error::ConversionError;
	fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
		value.parse()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct TaskStatus {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub message: Option<Message>,
	pub state: TaskState,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub timestamp: Option<chrono::NaiveDateTime>,
}
impl From<&TaskStatus> for TaskStatus {
	fn from(value: &TaskStatus) -> Self {
		value.clone()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct TaskStatusUpdateEvent {
	#[serde(rename = "final", default)]
	pub final_: bool,
	pub id: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub metadata: Option<::serde_json::Map<String, ::serde_json::Value>>,
	pub status: TaskStatus,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct TextPart {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub metadata: Option<::serde_json::Map<String, ::serde_json::Value>>,
	pub text: String,
	#[serde(rename = "type", default = "defaults::text_part_type")]
	pub type_: String,
}
impl From<&TextPart> for TextPart {
	fn from(value: &TextPart) -> Self {
		value.clone()
	}
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct UnsupportedOperationError {
	pub code: i64,
	pub data: ::serde_json::Value,
	pub message: String,
}
impl From<&UnsupportedOperationError> for UnsupportedOperationError {
	fn from(value: &UnsupportedOperationError) -> Self {
		value.clone()
	}
}
pub mod defaults {
	pub(super) fn agent_card_default_input_modes() -> Vec<String> {
		vec!["text".to_string()]
	}
	pub(super) fn agent_card_default_output_modes() -> Vec<String> {
		vec!["text".to_string()]
	}
	pub(super) fn data_part_type() -> String {
		"data".to_string()
	}
	pub(super) fn file_part_type() -> String {
		"file".to_string()
	}
	pub(super) fn text_part_type() -> String {
		"text".to_string()
	}
}

pub mod error {
	use std::fmt::Display;
	use std::fmt::{Debug, Formatter};
	pub struct ConversionError(::std::borrow::Cow<'static, str>);
	impl ::std::error::Error for ConversionError {}
	impl Display for ConversionError {
		fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), ::std::fmt::Error> {
			Display::fmt(&self.0, f)
		}
	}
	impl Debug for ConversionError {
		fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), ::std::fmt::Error> {
			Debug::fmt(&self.0, f)
		}
	}
	impl From<&'static str> for ConversionError {
		fn from(value: &'static str) -> Self {
			Self(value.into())
		}
	}
	impl From<String> for ConversionError {
		fn from(value: String) -> Self {
			Self(value.into())
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::{A2aResponse, TaskStatusUpdateEvent};

	#[test]
	fn test_serde() {
		let js = serde_json::json! {
		{
			"jsonrpc": "2.0",
			"id": "d1306567eb364c7ba9e7a7b922dba672",
			"result": {
				"id": "8b34914c735a464986e1d5ce5b6ec478",
				"status": {
					"state": "completed",
					"message": {
						"role": "agent",
						"parts": [
							{
								"type": "text",
								"text": "Hello!"
							}
						]
					},
					"timestamp": "2025-04-10T15:07:15.833777"
				},
				"final": false
			}
		}
		};
		let got: crate::JsonRpcMessage = serde_json::from_value(js).unwrap();
	}
}
