use crate::xds::mcp::kgateway_dev::target::LocalDataSource;
use crate::xds::mcp::kgateway_dev::target::local_data_source::Source as XdsSource;
use crate::xds::mcp::kgateway_dev::target::target::OpenApiTarget as XdsOpenAPITarget;
use openapiv3::{OpenAPI, Parameter, ReferenceOr, Schema};
use rmcp::model::JsonObject;
use rmcp::model::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
pub mod backend;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Target {
	pub name: String,
	pub spec: TargetSpec,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum TargetSpec {
	Sse {
		host: String,
		port: u32,
		path: String,
		headers: HashMap<String, String>,
		backend_auth: Option<backend::BackendAuthConfig>,
	},
	Stdio {
		cmd: String,
		args: Vec<String>,
		env: HashMap<String, String>,
	},
	OpenAPI(OpenAPITarget),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct OpenAPITarget {
	pub host: String,
	pub prefix: String,
	pub port: u32,
	pub tools: Vec<(Tool, UpstreamOpenAPICall)>,
}

impl TryFrom<XdsOpenAPITarget> for OpenAPITarget {
	type Error = ParseError;

	fn try_from(value: XdsOpenAPITarget) -> Result<Self, Self::Error> {
		let schema = value.schema.ok_or(ParseError::MissingSchema)?;
		let schema_bytes = resolve_local_data_source(&schema)?;
		let schema: OpenAPI = serde_json::from_slice(&schema_bytes).map_err(ParseError::SerdeError)?;
		let tools = parse_openapi_schema(&schema)?;
		let prefix = get_server_prefix(&schema)?;
		Ok(OpenAPITarget {
			host: value.host.clone(),
			prefix: prefix,
			port: value.port,
			tools,
		})
	}
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UpstreamOpenAPICall {
	pub method: String, // TODO: Switch to Method, but will require getting rid of Serialize/Deserialize
	pub path: String,
	// todo: params
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
	#[error("missing fields")]
	MissingFields,
	#[error("missing schema")]
	MissingSchema,
	#[error("missing components")]
	MissingComponents,
	#[error("missing component")]
	MissingComponent,
	#[error("missing reference")]
	MissingReference,
	#[error("unsupported reference")]
	UnsupportedReference(String),
	#[error("information requireds")]
	InformationRequired(String),
	#[error("serde error: {0}")]
	SerdeError(#[from] serde_json::Error),
}

fn get_server_prefix(server: &OpenAPI) -> Result<String, ParseError> {
	match server.servers.len() {
		0 => Ok("/".to_string()),
		1 => Ok(server.servers[0].url.clone()),
		_ => Err(ParseError::UnsupportedReference(format!(
			"multiple servers are not supported: {:?}",
			server.servers
		))),
	}
}

fn resolve_schema<'a>(
	reference: &'a ReferenceOr<Schema>,
	doc: &'a OpenAPI,
) -> Result<&'a Schema, ParseError> {
	match reference {
		ReferenceOr::Reference { reference } => {
			let reference = reference
				.strip_prefix("#/components/schemas/")
				.ok_or(ParseError::MissingReference)?;
			let components: &openapiv3::Components = doc
				.components
				.as_ref()
				.ok_or(ParseError::MissingComponents)?;
			let schema = components
				.schemas
				.get(reference)
				.ok_or(ParseError::MissingComponent)?;
			resolve_schema(schema, doc)
		},
		ReferenceOr::Item(schema) => Ok(schema),
	}
}

fn resolve_parameter<'a>(
	reference: &'a ReferenceOr<Parameter>,
	doc: &'a OpenAPI,
) -> Result<&'a Parameter, ParseError> {
	match reference {
		ReferenceOr::Reference { reference } => {
			let reference = reference
				.strip_prefix("#/components/parameters/")
				.ok_or(ParseError::MissingReference)?;
			let components: &openapiv3::Components = doc
				.components
				.as_ref()
				.ok_or(ParseError::MissingComponents)?;
			let parameter = components
				.parameters
				.get(reference)
				.ok_or(ParseError::MissingComponent)?;
			resolve_parameter(parameter, doc)
		},
		ReferenceOr::Item(parameter) => Ok(parameter),
	}
}

/// We need to rework this and I don't want to forget.
///
/// We need to be able to handle data which can end up in multiple destinations:
/// 1. Headers
/// 2. Body
/// 3. Query Params
/// 4. Templated Path Params
/// 
/// To support this we should create a nested JSON schema which has each of them.
/// That way the client code can properly separate the objects passed by the client.
/// 
fn parse_openapi_schema(
	open_api: &OpenAPI,
) -> Result<Vec<(Tool, UpstreamOpenAPICall)>, ParseError> {
	let tool_defs: Result<Vec<_>, _> = open_api
		.paths
		.iter()
		.map(
			|(path, path_info)| -> Result<Vec<(Tool, UpstreamOpenAPICall)>, ParseError> {
				let item = path_info
					.as_item()
					.ok_or(ParseError::UnsupportedReference(path.to_string()))?;
				let items: Result<Vec<_>, _> = item
					.iter()
					.map(
						|(method, op)| -> Result<(Tool, UpstreamOpenAPICall), ParseError> {
							let name = op
								.operation_id
								.clone()
								.ok_or(ParseError::InformationRequired(format!(
									"operation_id is required for {}",
									path
								)))?;
							let props: Result<Vec<_>, _> = op
								.parameters
								.iter()
								.map(|p| -> Result<(String, JsonObject, bool), ParseError> {
									let item = resolve_parameter(p, open_api)?;
									let p = dbg!(item.parameter_data_ref());
									let mut schema = match &p.format {
										openapiv3::ParameterSchemaOrContent::Schema(reference) => {
											let resolved_schema = resolve_schema(reference, open_api)?;
											serde_json::to_value(resolved_schema)
												.map_err(ParseError::SerdeError)?
												.as_object()
												.ok_or(ParseError::UnsupportedReference(format!(
													"parameter {} is not an object",
													p.name
												)))?
												.clone()
										},
										openapiv3::ParameterSchemaOrContent::Content(content) => {
											return Err(ParseError::UnsupportedReference(format!(
												"content is not supported for parameters: {:?}",
												content
											)));
										},
									};

									if let Some(desc) = &p.description {
										schema.insert("description".to_string(), json!(desc));
									}

									Ok((p.name.clone(), schema, p.required))
								})
								.collect();
							let props = props?;
							let mut schema = JsonObject::new();
							schema.insert("type".to_string(), json!("object"));
							let required: Vec<String> = props
								.iter()
								.flat_map(|(name, _, req)| if *req { Some(name.clone()) } else { None })
								.collect();
							schema.insert("required".to_string(), json!(required));
							let mut schema_props = JsonObject::new();
							for (name, s, _) in props {
								schema_props.insert(name, json!(s));
							}
							schema.insert("properties".to_string(), json!(schema_props));
							let tool = Tool {
								annotations: None,
								name: Cow::Owned(name.clone()),
								description: Some(Cow::Owned(
									op.description
										.as_ref()
										.unwrap_or_else(|| op.summary.as_ref().unwrap_or(&name))
										.to_string(),
								)),
								input_schema: Arc::new(schema),
							};
							let upstream = UpstreamOpenAPICall {
								// method: Method::from_bytes(method.as_ref()).expect("todo"),
								method: method.to_string(),
								path: path.clone(),
							};
							Ok((tool, upstream))
						},
					)
					.collect();
				// Rust has a hard time with this...
				let items = items?;
				Ok(items)
			},
		)
		.collect();

	match tool_defs {
		Ok(tool_defs) => Ok(tool_defs.into_iter().flatten().collect()),
		Err(e) => Err(e),
	}
}

pub fn resolve_local_data_source(
	local_data_source: &LocalDataSource,
) -> Result<Vec<u8>, ParseError> {
	match local_data_source
		.source
		.as_ref()
		.ok_or(ParseError::MissingFields)?
	{
		XdsSource::FilePath(file_path) => {
			let file = std::fs::read(file_path).map_err(|_| ParseError::MissingFields)?;
			Ok(file)
		},
		XdsSource::Inline(inline) => Ok(inline.clone()),
	}
}

#[test]
fn test_parse_openapi_schema() {
	let schema = include_bytes!("../examples/openapi/openapi.json");
	let schema: OpenAPI = serde_json::from_slice(schema).unwrap();
	let tools = parse_openapi_schema(&schema).unwrap();
	println!("{:?}", tools);
}

