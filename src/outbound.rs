use crate::proto;
use crate::proto::aidp::dev::a2a::target::Target as XdsA2aTarget;
use crate::proto::aidp::dev::common::BackendAuth as XdsAuth;
use crate::proto::aidp::dev::common::BackendTls as XdsTls;
use crate::proto::aidp::dev::mcp::target::Target as McpXdsTarget;
use crate::proto::aidp::dev::mcp::target::target::OpenApiTarget as XdsOpenAPITarget;
use crate::proto::aidp::dev::mcp::target::target::SseTarget as XdsSseTarget;
use crate::proto::aidp::dev::mcp::target::target::Target as XdsTarget;
use openapiv3::OpenAPI;
use rmcp::model::Tool;
use serde::Serialize;
use std::collections::HashMap;
pub mod backend;
pub mod openapi;

#[derive(Clone, Serialize, Debug)]
pub struct Target<T> {
	pub name: String,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub listeners: Vec<String>,
	pub spec: T,
}

impl TryFrom<McpXdsTarget> for Target<McpTargetSpec> {
	type Error = anyhow::Error;

	fn try_from(value: McpXdsTarget) -> Result<Self, Self::Error> {
		let target = match value.target {
			Some(target) => target,
			None => return Err(anyhow::anyhow!("target is None")),
		};
		Ok(Target {
			name: value.name,
			listeners: value.listeners,
			spec: target.try_into()?,
		})
	}
}

#[derive(Clone, Serialize, Debug)]
pub enum McpTargetSpec {
	Sse(SseTargetSpec),
	Stdio {
		cmd: String,
		#[serde(skip_serializing_if = "Vec::is_empty")]
		args: Vec<String>,
		#[serde(skip_serializing_if = "HashMap::is_empty")]
		env: HashMap<String, String>,
	},
	OpenAPI(OpenAPITarget),
}

impl TryFrom<XdsTarget> for McpTargetSpec {
	type Error = anyhow::Error;

	fn try_from(value: XdsTarget) -> Result<Self, Self::Error> {
		let target = match value {
			XdsTarget::Sse(sse) => McpTargetSpec::Sse(sse.try_into()?),
			XdsTarget::Stdio(stdio) => McpTargetSpec::Stdio {
				cmd: stdio.cmd,
				args: stdio.args,
				env: stdio.env,
			},
			XdsTarget::Openapi(openapi) => McpTargetSpec::OpenAPI(openapi.try_into()?),
		};
		Ok(target)
	}
}

#[derive(Clone, Serialize, Debug)]
pub enum A2aTargetSpec {
	Sse(SseTargetSpec),
}

impl TryFrom<XdsA2aTarget> for Target<A2aTargetSpec> {
	type Error = anyhow::Error;

	fn try_from(value: XdsA2aTarget) -> Result<Self, Self::Error> {
		Ok(Target {
			name: value.name,
			listeners: value.listeners,
			spec: A2aTargetSpec::Sse(SseTargetSpec {
				host: value.host,
				port: value.port,
				path: value.path,
				headers: proto::resolve_header_map(&value.headers)?,
				backend_auth: match value.auth {
					Some(auth) => XdsAuth::try_into(auth)?,
					None => None,
				},
				tls: match value.tls {
					Some(tls) => Some(TlsConfig::try_from(tls)?),
					None => None,
				},
			}),
		})
	}
}

#[derive(Clone, Serialize, Debug)]
pub struct SseTargetSpec {
	pub host: String,
	pub port: u32,
	pub path: String,
	#[serde(skip_serializing_if = "HashMap::is_empty")]
	pub headers: HashMap<String, String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub backend_auth: Option<backend::BackendAuthConfig>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tls: Option<TlsConfig>,
}

#[derive(Clone, Serialize, Debug)]
pub struct TlsConfig {
	pub insecure_skip_verify: bool,
}

impl TryFrom<XdsTls> for TlsConfig {
	type Error = anyhow::Error;

	fn try_from(value: XdsTls) -> Result<Self, Self::Error> {
		Ok(TlsConfig {
			insecure_skip_verify: value.insecure_skip_verify,
		})
	}
}

impl TryFrom<XdsSseTarget> for SseTargetSpec {
	type Error = anyhow::Error;

	fn try_from(value: XdsSseTarget) -> Result<Self, Self::Error> {
		Ok(SseTargetSpec {
			host: value.host,
			port: value.port,
			path: value.path,
			headers: proto::resolve_header_map(&value.headers)?,
			backend_auth: match value.auth {
				Some(auth) => XdsAuth::try_into(auth)?,
				None => None,
			},
			tls: match value.tls {
				Some(tls) => Some(TlsConfig::try_from(tls)?),
				None => None,
			},
		})
	}
}

#[derive(Clone, Serialize, Debug)]
pub struct OpenAPITarget {
	pub host: String,
	pub prefix: String,
	pub port: u16,
	pub tools: Vec<(Tool, openapi::UpstreamOpenAPICall)>,
	pub headers: HashMap<String, String>,
	pub backend_auth: Option<backend::BackendAuthConfig>,
}

impl TryFrom<XdsOpenAPITarget> for OpenAPITarget {
	type Error = openapi::ParseError;

	fn try_from(value: XdsOpenAPITarget) -> Result<Self, Self::Error> {
		let schema = value.schema.ok_or(openapi::ParseError::MissingSchema)?;
		let schema_bytes =
			proto::resolve_local_data_source(&schema.source.ok_or(openapi::ParseError::MissingFields)?)?;
		let schema: OpenAPI =
			serde_json::from_slice(&schema_bytes).map_err(openapi::ParseError::SerdeError)?;
		let tools = openapi::parse_openapi_schema(&schema)?;
		let prefix = openapi::get_server_prefix(&schema)?;
		let headers = proto::resolve_header_map(&value.headers)?;
		Ok(OpenAPITarget {
			host: value.host.clone(),
			prefix,
			port: value.port as u16, // TODO: check if this is correct
			tools,
			headers,
			backend_auth: match value.auth {
				Some(auth) => auth
					.try_into()
					.map_err(|_| openapi::ParseError::MissingSchema)?,
				None => None,
			},
		})
	}
}
