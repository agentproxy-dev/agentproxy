use openapiv3::Paths;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
pub mod backend;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Target {
	pub name: String,
	pub spec: TargetSpec,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum TargetSpec {
	#[serde(rename = "sse")]
	Sse {
		host: String,
		port: u32,
		path: String,
		backend_auth: Option<backend::BackendAuthConfig>,
	},
	#[serde(rename = "stdio")]
	Stdio {
		cmd: String,
		args: Vec<String>,
		env: HashMap<String, String>,
	},
	#[serde(rename = "openapi")]
	OpenAPI {
		host: String,
		port: u32,
		schema: OpenAPISchema,
	},
}
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct OpenAPISchema {
	// The crate OpenAPI type requires a lot more, we only need paths for now so use only a subset of it.
	pub paths: Paths,
}
