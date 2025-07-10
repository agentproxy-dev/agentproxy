use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::sync::Arc;

use http::Method;
use http::header::{ACCEPT, CONTENT_TYPE};
use http_body_util::BodyExt;
use hyper_util::rt::TokioIo;
use openapiv3::{OpenAPI as OpenAPIv3, Parameter as Parameterv3, ReferenceOr as ReferenceOrv3, RequestBody as RequestBodyv3, Schema as Schemav3, SchemaKind as SchemaKindv3, Type as Typev3};
use crate::types::agent::OpenAPI;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use rmcp::model::{JsonObject, Tool};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::instrument;
use url::Url;

use crate::client;
use crate::store::BackendPolicies;
use crate::types::agent::Target;

mod compatibility;
mod adapters;

use compatibility::{CompatibleSchema, CompatibleParameter, CompatibleRequestBody, ParameterLocation, ToCompatible};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UpstreamOpenAPICall {
	pub method: String, /* TODO: Switch to Method, but will require getting rid of Serialize/Deserialize */
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
	#[error("invalid reference: {0}")]
	InvalidReference(String),
	#[error("missing reference")]
	MissingReference(String),
	#[error("unsupported reference")]
	UnsupportedReference(String),
	#[error("information required: {0}")] // Corrected typo from "requireds"
	InformationRequired(String),
	#[error("serde error: {0}")]
	SerdeError(#[from] serde_json::Error),
	#[error("io error: {0}")]
	IoError(#[from] std::io::Error),
	#[error("HTTP request failed: {0}")]
	HttpError(#[from] reqwest::Error),
	#[error("Invalid URL: {0}")]
	InvalidUrl(#[from] url::ParseError),
	#[error("Schema source not specified in OpenAPI target")]
	SchemaSourceMissing,
	#[error(
		"Unsupported schema format or content type from URL {0}. Only JSON and YAML are supported."
	)]
	UnsupportedSchemaFormat(String), // Added URL to message
	#[error("Local schema file path not specified")]
	LocalPathMissing,
	#[error("Local schema inline content not specified or empty")]
	LocalInlineMissing, // Added for inline content
	#[error("Invalid header name or value")]
	InvalidHeader,
	#[error("Header value source not supported (e.g. env_value)")]
	HeaderValueSourceNotSupported(String),
}

pub(crate) fn get_server_prefix(server: &OpenAPI) -> Result<String, ParseError> {
	match server {
		OpenAPI::V3_0(spec) => {
			match spec.servers.len() {
				0 => Ok("/".to_string()),
				1 => Ok(spec.servers[0].url.clone()),
				_ => Err(ParseError::UnsupportedReference(format!(
					"multiple servers are not supported: {:?}",
					spec.servers
				))),
			}
		},
		OpenAPI::V3_1(spec) => {
			let empty_vec = Vec::new();
			let servers = spec.servers.as_ref().unwrap_or(&empty_vec);
			match servers.len() {
				0 => Ok("/".to_string()),
				1 => Ok(servers[0].url.clone()),
				_ => Err(ParseError::UnsupportedReference(format!(
					"multiple servers are not supported (found {} servers)",
					servers.len()
				))),
			}
		},
	}
}


/// Main entry point for parsing OpenAPI schemas.
/// Routes to the appropriate version-specific parser based on the OpenAPI version.
pub fn parse_openapi_schema(
	open_api: &OpenAPI,
) -> Result<Vec<(Tool, UpstreamOpenAPICall)>, ParseError> {
	match open_api {
		OpenAPI::V3_0(spec) => parse_openapi_v3_0_schema(spec),
		OpenAPI::V3_1(spec) => parse_openapi_v3_1_schema(spec),
	}
}

/// Parse OpenAPI 3.0 schema into tools and upstream calls
fn parse_openapi_v3_0_schema(
	open_api: &OpenAPIv3,
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
									"operation_id is required for {path}"
								)))?;

							// Build the schema
							let mut final_schema = JsonSchema::default();

							let body: Option<(String, serde_json::Value, bool)> = match op.request_body.as_ref() {
								Some(body) => {
									let body = resolve_request_body_v3_0(body, open_api)?;
									match body.content.get("application/json") {
										Some(media_type) => {
											let schema_ref = media_type
												.schema
												.as_ref()
												.ok_or(ParseError::MissingReference("application/json".to_string()))?;
											let schema = resolve_nested_schema_v3_0(schema_ref, open_api)?;
											let body_schema =
												serde_json::to_value(schema).map_err(ParseError::SerdeError)?;
											if body.required {
												final_schema.required.push(BODY_NAME.clone());
											}
											final_schema
												.properties
												.insert(BODY_NAME.clone(), body_schema.clone());
											Some((BODY_NAME.clone(), body_schema, body.required))
										},
										None => None,
									}
								},
								None => None,
							};

							if let Some((name, schema, required)) = body {
								if required {
									final_schema.required.push(name.clone());
								}
								final_schema.properties.insert(name.clone(), schema.clone());
							}

							let mut param_schemas: HashMap<ParameterType, Vec<(String, JsonObject, bool)>> =
								HashMap::new();
							op.parameters
								.iter()
								.try_for_each(|p| -> Result<(), ParseError> {
									let item = resolve_parameter_v3_0(p, open_api)?;
									let (name, schema, required) = build_schema_property_v3_0(open_api, item)?;
									match item {
										Parameterv3::Header { .. } => {
											param_schemas
												.entry(ParameterType::Header)
												.or_insert_with(Vec::new)
												.push((name, schema, required));
											Ok(())
										},
										Parameterv3::Query { .. } => {
											param_schemas
												.entry(ParameterType::Query)
												.or_insert_with(Vec::new)
												.push((name, schema, required));
											Ok(())
										},
										Parameterv3::Path { .. } => {
											param_schemas
												.entry(ParameterType::Path)
												.or_insert_with(Vec::new)
												.push((name, schema, required));
											Ok(())
										},
										_ => Err(ParseError::UnsupportedReference(
											"parameter type COOKIE is not supported".to_string(),
										)),
									}
								})?;

							for (param_type, props) in param_schemas {
								let sub_schema = JsonSchema {
									required: props
										.iter()
										.flat_map(|(name, _, req)| if *req { Some(name.clone()) } else { None })
										.collect(),
									properties: props
										.iter()
										.map(|(name, s, _)| (name.clone(), json!(s)))
										.collect(),
									..Default::default()
								};

								if !sub_schema.required.is_empty() {
									final_schema.required.push(param_type.to_string());
								}
								final_schema
									.properties
									.insert(param_type.to_string(), json!(sub_schema));
							}

							let final_json =
								serde_json::to_value(final_schema).map_err(ParseError::SerdeError)?;
							let final_json = final_json
								.as_object()
								.ok_or(ParseError::UnsupportedReference(
									"final schema is not an object".to_string(),
								))?
								.clone();
							let tool = Tool {
								annotations: None,
								name: Cow::Owned(name.clone()),
								description: Some(Cow::Owned(
									op.description
										.as_ref()
										.unwrap_or_else(|| op.summary.as_ref().unwrap_or(&name))
										.to_string(),
								)),
								input_schema: Arc::new(final_json),
							};
							let upstream = UpstreamOpenAPICall {
								method: method.to_string(),
								path: path.clone(),
							};
							Ok((tool, upstream))
						},
					)
					.collect();
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

/// Parse OpenAPI 3.1 schema into tools and upstream calls
/// Currently returns an error with helpful message as 3.1 support is not yet fully implemented
fn parse_openapi_v3_1_schema(
	_open_api: &openapiv3_1::OpenApi,
) -> Result<Vec<(Tool, UpstreamOpenAPICall)>, ParseError> {
	Err(ParseError::InformationRequired(
		"OpenAPI 3.1 parsing is not yet fully implemented. \
		The specification pattern has been set up to support 3.1, but the parsing logic \
		needs to be completed based on the openapiv3_1 crate API structure. \
		Please use OpenAPI 3.0 specifications for now, or help implement the 3.1 parsing logic.".to_string()
	))
}

// Used to index the parameter types for the schema
lazy_static::lazy_static! {
	pub static ref BODY_NAME: String = "body".to_string();
	pub static ref HEADER_NAME: String = "header".to_string();
	pub static ref QUERY_NAME: String = "query".to_string();
	pub static ref PATH_NAME: String = "path".to_string();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ParameterType {
	Header,
	Query,
	Path,
}

impl std::fmt::Display for ParameterType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				ParameterType::Header => "header",
				ParameterType::Query => "query",
				ParameterType::Path => "path",
			}
		)
	}
}

// ===== OpenAPI 3.0 specific functions =====

fn resolve_schema_v3_0<'a>(
	reference: &'a ReferenceOrv3<Schemav3>,
	doc: &'a OpenAPIv3,
) -> Result<&'a Schemav3, ParseError> {
	match reference {
		ReferenceOrv3::Reference { reference } => {
			let reference = reference
				.strip_prefix("#/components/schemas/")
				.ok_or(ParseError::InvalidReference(reference.to_string()))?;
			let components = doc
				.components
				.as_ref()
				.ok_or(ParseError::MissingComponents)?;
			let schema = components
				.schemas
				.get(reference)
				.ok_or(ParseError::MissingReference(reference.to_string()))?;
			resolve_schema_v3_0(schema, doc)
		},
		ReferenceOrv3::Item(schema) => Ok(schema),
	}
}

fn resolve_nested_schema_v3_0<'a>(
	reference: &'a ReferenceOrv3<Schemav3>,
	doc: &'a OpenAPIv3,
) -> Result<Schemav3, ParseError> {
	let base_schema = resolve_schema_v3_0(reference, doc)?;
	let mut resolved_schema = base_schema.clone();

	match &mut resolved_schema.schema_kind {
		SchemaKindv3::Type(Typev3::Object(obj)) => {
			for prop_ref_box in obj.properties.values_mut() {
				let owned_prop_ref_or_box = prop_ref_box.clone();
				let temp_prop_ref = match owned_prop_ref_or_box {
					ReferenceOrv3::Reference { reference } => ReferenceOrv3::Reference { reference },
					ReferenceOrv3::Item(boxed_item) => ReferenceOrv3::Item((*boxed_item).clone()),
				};
				let resolved_prop = resolve_nested_schema_v3_0(&temp_prop_ref, doc)?;
				*prop_ref_box = ReferenceOrv3::Item(Box::new(resolved_prop));
			}
		},
		SchemaKindv3::Type(Typev3::Array(arr)) => {
			if let Some(items_ref_box) = arr.items.as_mut() {
				let owned_items_ref_or_box = items_ref_box.clone();
				let temp_items_ref = match owned_items_ref_or_box {
					ReferenceOrv3::Reference { reference } => ReferenceOrv3::Reference { reference },
					ReferenceOrv3::Item(boxed_item) => ReferenceOrv3::Item((*boxed_item).clone()),
				};
				let resolved_items = resolve_nested_schema_v3_0(&temp_items_ref, doc)?;
				*items_ref_box = ReferenceOrv3::Item(Box::new(resolved_items));
			}
		},
		SchemaKindv3::OneOf { one_of } => {
			for ref_or_schema in one_of.iter_mut() {
				let temp_ref = ref_or_schema.clone();
				let resolved = resolve_nested_schema_v3_0(&temp_ref, doc)?;
				*ref_or_schema = ReferenceOrv3::Item(resolved);
			}
		},
		SchemaKindv3::AllOf { all_of } => {
			for ref_or_schema in all_of.iter_mut() {
				let temp_ref = ref_or_schema.clone();
				let resolved = resolve_nested_schema_v3_0(&temp_ref, doc)?;
				*ref_or_schema = ReferenceOrv3::Item(resolved);
			}
		},
		SchemaKindv3::AnyOf { any_of } => {
			for ref_or_schema in any_of.iter_mut() {
				let temp_ref = ref_or_schema.clone();
				let resolved = resolve_nested_schema_v3_0(&temp_ref, doc)?;
				*ref_or_schema = ReferenceOrv3::Item(resolved);
			}
		},
		SchemaKindv3::Not { not } => {
			let temp_ref = (**not).clone();
			let resolved = resolve_nested_schema_v3_0(&temp_ref, doc)?;
			*not = Box::new(ReferenceOrv3::Item(resolved));
		},
		SchemaKindv3::Any(any_schema) => {
			for prop_ref_box in any_schema.properties.values_mut() {
				let owned_prop_ref_or_box = prop_ref_box.clone();
				let temp_prop_ref = match owned_prop_ref_or_box {
					ReferenceOrv3::Reference { reference } => ReferenceOrv3::Reference { reference },
					ReferenceOrv3::Item(boxed_item) => ReferenceOrv3::Item((*boxed_item).clone()),
				};
				let resolved_prop = resolve_nested_schema_v3_0(&temp_prop_ref, doc)?;
				*prop_ref_box = ReferenceOrv3::Item(Box::new(resolved_prop));
			}
			if let Some(items_ref_box) = any_schema.items.as_mut() {
				let owned_items_ref_or_box = items_ref_box.clone();
				let temp_items_ref = match owned_items_ref_or_box {
					ReferenceOrv3::Reference { reference } => ReferenceOrv3::Reference { reference },
					ReferenceOrv3::Item(boxed_item) => ReferenceOrv3::Item((*boxed_item).clone()),
				};
				let resolved_items = resolve_nested_schema_v3_0(&temp_items_ref, doc)?;
				*items_ref_box = ReferenceOrv3::Item(Box::new(resolved_items));
			}
			for vec_ref in [
				&mut any_schema.one_of,
				&mut any_schema.all_of,
				&mut any_schema.any_of,
			] {
				for ref_or_schema in vec_ref.iter_mut() {
					let temp_ref = ref_or_schema.clone();
					let resolved = resolve_nested_schema_v3_0(&temp_ref, doc)?;
					*ref_or_schema = ReferenceOrv3::Item(resolved);
				}
			}
			if let Some(not_box) = any_schema.not.as_mut() {
				let temp_ref = (**not_box).clone();
				let resolved = resolve_nested_schema_v3_0(&temp_ref, doc)?;
				*not_box = Box::new(ReferenceOrv3::Item(resolved));
			}
		},
		SchemaKindv3::Type(_) => {},
	}

	Ok(resolved_schema)
}

fn resolve_parameter_v3_0<'a>(
	reference: &'a ReferenceOrv3<Parameterv3>,
	doc: &'a OpenAPIv3,
) -> Result<&'a Parameterv3, ParseError> {
	match reference {
		ReferenceOrv3::Reference { reference } => {
			let reference = reference
				.strip_prefix("#/components/parameters/")
				.ok_or(ParseError::MissingReference(reference.to_string()))?;
			let components = doc
				.components
				.as_ref()
				.ok_or(ParseError::MissingComponents)?;
			let parameter = components
				.parameters
				.get(reference)
				.ok_or(ParseError::MissingReference(reference.to_string()))?;
			resolve_parameter_v3_0(parameter, doc)
		},
		ReferenceOrv3::Item(parameter) => Ok(parameter),
	}
}

fn resolve_request_body_v3_0<'a>(
	reference: &'a ReferenceOrv3<RequestBodyv3>,
	doc: &'a OpenAPIv3,
) -> Result<&'a RequestBodyv3, ParseError> {
	match reference {
		ReferenceOrv3::Reference { reference } => {
			let reference = reference
				.strip_prefix("#/components/requestBodies/")
				.ok_or(ParseError::MissingReference(reference.to_string()))?;
			let components = doc
				.components
				.as_ref()
				.ok_or(ParseError::MissingComponents)?;
			let request_body = components
				.request_bodies
				.get(reference)
				.ok_or(ParseError::MissingReference(reference.to_string()))?;
			resolve_request_body_v3_0(request_body, doc)
		},
		ReferenceOrv3::Item(request_body) => Ok(request_body),
	}
}

fn build_schema_property_v3_0(
	open_api: &OpenAPIv3,
	item: &Parameterv3,
) -> Result<(String, JsonObject, bool), ParseError> {
	let p = item.parameter_data_ref();
	let mut schema = match &p.format {
		openapiv3::ParameterSchemaOrContent::Schema(reference) => {
			let resolved_schema = resolve_schema_v3_0(reference, open_api)?;
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
				"content is not supported for parameters: {content:?}"
			)));
		},
	};

	if let Some(desc) = &p.description {
		schema.insert("description".to_string(), json!(desc));
	}

	Ok((p.name.clone(), schema, p.required))
}

// ===== OpenAPI 3.1 specific functions =====
// TODO: Implement OpenAPI 3.1 parsing functions when needed
// The functions would be similar to the 3.0 versions but adapted for the openapiv3_1 crate API

#[derive(Debug, Serialize, Deserialize)]
struct JsonSchema {
	required: Vec<String>,
	properties: JsonObject,
	r#type: String,
}

impl Default for JsonSchema {
	fn default() -> Self {
		Self {
			required: vec![],
			properties: JsonObject::new(),
			r#type: "object".to_string(),
		}
	}
}

#[derive(Debug)]
pub struct Handler {
	pub host: String,
	pub prefix: String,
	pub port: u32,
	pub client: client::Client,
	pub tools: Vec<(Tool, UpstreamOpenAPICall)>,
	pub policies: BackendPolicies,
}

impl Handler {
	/// We need to use the parse the schema to get the correct args.
	/// They are in the json schema under the "properties" key.
	/// Body is under the "body" key.
	/// Headers are under the "header" key.
	/// Query params are under the "query" key.
	/// Path params are under the "path" key.
	///
	/// Query params need to be added to the url as query params.
	/// Headers need to be added to the request headers.
	/// Body needs to be added to the request body.
	/// Path params need to be added to the template params in the path.
	#[instrument(
		level = "debug",
		skip_all,
		fields(
			name=%name,
		),
	)]
	pub async fn call_tool(
		&self,
		name: &str,
		args: Option<JsonObject>,
	) -> Result<String, anyhow::Error> {
		let (_tool, info) = self
			.tools
			.iter()
			.find(|(t, _info)| t.name == name)
			.ok_or_else(|| anyhow::anyhow!("tool {} not found", name))?;

		let args = args.unwrap_or_default();

		// --- Parameter Extraction ---
		let path_params = args
			.get(&*PATH_NAME)
			.and_then(Value::as_object)
			.cloned()
			.unwrap_or_default();
		let query_params = args
			.get(&*QUERY_NAME)
			.and_then(Value::as_object)
			.cloned()
			.unwrap_or_default();
		let header_params = args
			.get(&*HEADER_NAME)
			.and_then(Value::as_object)
			.cloned()
			.unwrap_or_default();
		let body_value = args.get(&*BODY_NAME).cloned();

		// --- URL Construction ---
		let mut path = info.path.clone();
		// Substitute path parameters into the path template
		for (key, value) in &path_params {
			match value {
				Value::String(s_val) => {
					path = path.replace(&format!("{{{key}}}"), s_val);
				},
				Value::Number(n_val) => {
					path = path.replace(&format!("{{{key}}}"), n_val.to_string().as_str());
				},
				_ => {
					tracing::warn!(
						"Path parameter '{}' for tool '{}' is not a string (value: {:?}), skipping substitution",
						key,
						name,
						value
					);
				},
			}
		}

		let base_url = format!(
			"{}://{}:{}{}{}",
			"http", self.host, self.port, self.prefix, path
		);

		// --- Request Building ---
		let method = Method::from_bytes(info.method.to_uppercase().as_bytes()).map_err(|e| {
			anyhow::anyhow!(
				"Invalid HTTP method '{}' for tool '{}': {}",
				info.method,
				name,
				e
			)
		})?;

		// Build query string
		let query_string = if !query_params.is_empty() {
			let mut pairs = Vec::new();
			for (k, v) in query_params.iter() {
				if let Some(s) = v.as_str() {
					pairs.push(format!("{k}={s}"));
				} else {
					tracing::warn!(
						"Query parameter '{}' for tool '{}' is not a string (value: {:?}), skipping",
						k,
						name,
						v
					);
				}
			}
			if !pairs.is_empty() {
				format!("?{}", pairs.join("&"))
			} else {
				String::new()
			}
		} else {
			String::new()
		};

		let uri = format!("{base_url}{query_string}");
		let mut headers = HeaderMap::new();
		let mut rb = http::Request::builder().method(method).uri(uri);

		rb = rb.header(ACCEPT, HeaderValue::from_static("application/json"));
		for (key, value) in &header_params {
			if let Some(s_val) = value.as_str() {
				match (
					HeaderName::from_bytes(key.as_bytes()),
					HeaderValue::from_str(s_val),
				) {
					(Ok(h_name), Ok(h_value)) => {
						rb = rb.header(h_name, h_value);
					},
					(Err(_), _) => tracing::warn!(
						"Invalid header name '{}' for tool '{}', skipping",
						key,
						name
					),
					(_, Err(_)) => tracing::warn!(
						"Invalid header value '{}' for header '{}' in tool '{}', skipping",
						s_val,
						key,
						name
					),
				}
			} else {
				tracing::warn!(
					"Header parameter '{}' for tool '{}' is not a string (value: {:?}), skipping",
					key,
					name,
					value
				);
			}
		}
		// Build request body
		let body = if let Some(body_val) = body_value {
			rb = rb.header(CONTENT_TYPE, HeaderValue::from_static("application/json"));
			serde_json::to_vec(&body_val)?
		} else {
			Vec::new()
		};

		// Build the final request
		let mut request = rb
			.body(body.into())
			.map_err(|e| anyhow::anyhow!("Failed to build request: {}", e))?;

		// Make the request
		let target = Target::try_from((self.host.as_str(), self.port as u16))?;
		let response = self
			.client
			.call(client::Call {
				req: request,
				target,
				transport: self.policies.backend_tls.clone().into(),
			})
			.await?;

		// Read response body
		let status = response.status();
		let body = String::from_utf8(
			axum::body::to_bytes(response.into_body(), 2_097_152)
				.await?
				.to_vec(),
		)?;

		// Check if the request was successful
		if status.is_success() {
			Ok(body)
		} else {
			Err(anyhow::anyhow!(
				"Upstream API call for tool '{}' failed with status {}: {}",
				name,
				status,
				body
			))
		}
	}

	pub fn tools(&self) -> Vec<Tool> {
		self.tools.clone().into_iter().map(|(t, _)| t).collect()
	}
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "test_31.rs"]
mod test_31;
