use http::{Method, header::ACCEPT};
use openapiv3::{OpenAPI, Parameter, ReferenceOr, RequestBody, Schema, SchemaKind, Type};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use rmcp::model::JsonObject;
use rmcp::model::Tool;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::json;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

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
	#[error("invalid reference: {0}")]
	InvalidReference(String),
	#[error("missing reference")]
	MissingReference(String),
	#[error("unsupported reference")]
	UnsupportedReference(String),
	#[error("information requireds")]
	InformationRequired(String),
	#[error("serde error: {0}")]
	SerdeError(#[from] serde_json::Error),
	#[error("io error: {0}")]
	IoError(#[from] std::io::Error),
}

pub(crate) fn get_server_prefix(server: &OpenAPI) -> Result<String, ParseError> {
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
				.ok_or(ParseError::InvalidReference(reference.to_string()))?;
			let components: &openapiv3::Components = doc
				.components
				.as_ref()
				.ok_or(ParseError::MissingComponents)?;
			let schema = components
				.schemas
				.get(reference)
				.ok_or(ParseError::MissingReference(reference.to_string()))?;
			resolve_schema(schema, doc)
		},
		ReferenceOr::Item(schema) => Ok(schema),
	}
}

/// Recursively resolves all nested schema references (`$ref`) within a given schema,
/// returning a new `Schema` object with all references replaced by their corresponding items.
fn resolve_nested_schema<'a>(
	reference: &'a ReferenceOr<Schema>,
	doc: &'a OpenAPI,
) -> Result<Schema, ParseError> {
	// 1. Resolve the initial reference to get the base Schema object (immutable borrow)
	let base_schema = resolve_schema(reference, doc)?;

	// 2. Clone the base schema to create a mutable owned version we can modify
	let mut resolved_schema = base_schema.clone();

	// 3. Match on the kind and recursively resolve + update the mutable clone
	match &mut resolved_schema.schema_kind {
		SchemaKind::Type(Type::Object(obj)) => {
			for prop_ref_box in obj.properties.values_mut() {
				let owned_prop_ref_or_box = prop_ref_box.clone();
				let temp_prop_ref = match owned_prop_ref_or_box {
					ReferenceOr::Reference { reference } => ReferenceOr::Reference { reference },
					ReferenceOr::Item(boxed_item) => ReferenceOr::Item((*boxed_item).clone()),
				};
				let resolved_prop = resolve_nested_schema(&temp_prop_ref, doc)?;
				*prop_ref_box = ReferenceOr::Item(Box::new(resolved_prop));
			}
		},
		SchemaKind::Type(Type::Array(arr)) => {
			if let Some(items_ref_box) = arr.items.as_mut() {
				let owned_items_ref_or_box = items_ref_box.clone();
				let temp_items_ref = match owned_items_ref_or_box {
					ReferenceOr::Reference { reference } => ReferenceOr::Reference { reference },
					ReferenceOr::Item(boxed_item) => ReferenceOr::Item((*boxed_item).clone()),
				};
				let resolved_items = resolve_nested_schema(&temp_items_ref, doc)?;
				*items_ref_box = ReferenceOr::Item(Box::new(resolved_items));
			}
		},
		// Handle combiners (OneOf, AllOf, AnyOf) with separate arms
		SchemaKind::OneOf { one_of } => {
			for ref_or_schema in one_of.iter_mut() {
				let temp_ref = ref_or_schema.clone();
				let resolved = resolve_nested_schema(&temp_ref, doc)?;
				*ref_or_schema = ReferenceOr::Item(resolved);
			}
		},
		SchemaKind::AllOf { all_of } => {
			for ref_or_schema in all_of.iter_mut() {
				let temp_ref = ref_or_schema.clone();
				let resolved = resolve_nested_schema(&temp_ref, doc)?;
				*ref_or_schema = ReferenceOr::Item(resolved);
			}
		},
		SchemaKind::AnyOf { any_of } => {
			for ref_or_schema in any_of.iter_mut() {
				let temp_ref = ref_or_schema.clone();
				let resolved = resolve_nested_schema(&temp_ref, doc)?;
				*ref_or_schema = ReferenceOr::Item(resolved);
			}
		},
		SchemaKind::Not { not } => {
			let temp_ref = (**not).clone();
			let resolved = resolve_nested_schema(&temp_ref, doc)?;
			*not = Box::new(ReferenceOr::Item(resolved));
		},
		SchemaKind::Any(any_schema) => {
			// Properties
			for prop_ref_box in any_schema.properties.values_mut() {
				let owned_prop_ref_or_box = prop_ref_box.clone();
				let temp_prop_ref = match owned_prop_ref_or_box {
					ReferenceOr::Reference { reference } => ReferenceOr::Reference { reference },
					ReferenceOr::Item(boxed_item) => ReferenceOr::Item((*boxed_item).clone()),
				};
				let resolved_prop = resolve_nested_schema(&temp_prop_ref, doc)?;
				*prop_ref_box = ReferenceOr::Item(Box::new(resolved_prop));
			}
			// Items
			if let Some(items_ref_box) = any_schema.items.as_mut() {
				let owned_items_ref_or_box = items_ref_box.clone();
				let temp_items_ref = match owned_items_ref_or_box {
					ReferenceOr::Reference { reference } => ReferenceOr::Reference { reference },
					ReferenceOr::Item(boxed_item) => ReferenceOr::Item((*boxed_item).clone()),
				};
				let resolved_items = resolve_nested_schema(&temp_items_ref, doc)?;
				*items_ref_box = ReferenceOr::Item(Box::new(resolved_items));
			}
			// oneOf, allOf, anyOf
			for vec_ref in [
				&mut any_schema.one_of,
				&mut any_schema.all_of,
				&mut any_schema.any_of,
			] {
				for ref_or_schema in vec_ref.iter_mut() {
					let temp_ref = ref_or_schema.clone();
					let resolved = resolve_nested_schema(&temp_ref, doc)?;
					*ref_or_schema = ReferenceOr::Item(resolved);
				}
			}
			// not
			if let Some(not_box) = any_schema.not.as_mut() {
				let temp_ref = (**not_box).clone();
				let resolved = resolve_nested_schema(&temp_ref, doc)?;
				*not_box = Box::new(ReferenceOr::Item(resolved));
			}
		},
		// Base types (String, Number, Integer, Boolean) - no nested schemas to resolve further
		SchemaKind::Type(_) => {}, // Do nothing, already resolved.
	}

	// 4. Return the modified owned schema
	Ok(resolved_schema)
}

fn resolve_parameter<'a>(
	reference: &'a ReferenceOr<Parameter>,
	doc: &'a OpenAPI,
) -> Result<&'a Parameter, ParseError> {
	match reference {
		ReferenceOr::Reference { reference } => {
			let reference = reference
				.strip_prefix("#/components/parameters/")
				.ok_or(ParseError::MissingReference(reference.to_string()))?;
			let components: &openapiv3::Components = doc
				.components
				.as_ref()
				.ok_or(ParseError::MissingComponents)?;
			let parameter = components
				.parameters
				.get(reference)
				.ok_or(ParseError::MissingReference(reference.to_string()))?;
			resolve_parameter(parameter, doc)
		},
		ReferenceOr::Item(parameter) => Ok(parameter),
	}
}

fn resolve_request_body<'a>(
	reference: &'a ReferenceOr<RequestBody>,
	doc: &'a OpenAPI,
) -> Result<&'a RequestBody, ParseError> {
	match reference {
		ReferenceOr::Reference { reference } => {
			let reference = reference
				.strip_prefix("#/components/requestBodies/")
				.ok_or(ParseError::MissingReference(reference.to_string()))?;
			let components: &openapiv3::Components = doc
				.components
				.as_ref()
				.ok_or(ParseError::MissingComponents)?;
			let request_body = components
				.request_bodies
				.get(reference)
				.ok_or(ParseError::MissingReference(reference.to_string()))?;
			resolve_request_body(request_body, doc)
		},
		ReferenceOr::Item(request_body) => Ok(request_body),
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
pub(crate) fn parse_openapi_schema(
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

							// Build the schema
							let mut final_schema = JsonSchema::default();

							let body: Option<(String, serde_json::Value, bool)> = match op.request_body.as_ref() {
								Some(body) => {
									let body = resolve_request_body(body, open_api)?;
									match body.content.get("application/json") {
										Some(media_type) => {
											let schema_ref = media_type
												.schema
												.as_ref()
												.ok_or(ParseError::MissingReference("application/json".to_string()))?;
											let schema = resolve_nested_schema(schema_ref, open_api)?;
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
									let item = resolve_parameter(p, open_api)?;
									let (name, schema, required) = build_schema_property(open_api, item)?;
									match item {
										Parameter::Header { .. } => {
											param_schemas
												.entry(ParameterType::Header)
												.or_insert_with(Vec::new)
												.push((name, schema, required));
											Ok(())
										},
										Parameter::Query { .. } => {
											param_schemas
												.entry(ParameterType::Query)
												.or_insert_with(Vec::new)
												.push((name, schema, required));
											Ok(())
										},
										Parameter::Path { .. } => {
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

fn build_schema_property(
	open_api: &OpenAPI,
	item: &Parameter,
) -> Result<(String, JsonObject, bool), ParseError> {
	let p = item.parameter_data_ref();
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
}

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
	pub scheme: String,
	pub host: String,
	pub prefix: String,
	pub port: u16,
	pub client: reqwest::Client,
	pub tools: Vec<(Tool, UpstreamOpenAPICall)>,
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
		let (_tool, info) = self // Renamed `_` to `_tool` to avoid unused variable warning, but kept for clarity
			.tools
			.iter()
			.find(|(t, _info)| t.name == name)
			.ok_or_else(|| anyhow::anyhow!("tool {} not found", name))?;

		let args = args.unwrap_or_default(); // Default to empty map if None

		// --- Parameter Extraction ---
		// Extract parameters, defaulting to empty maps if the corresponding key ("path", "query", "header") is missing or not an object.
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
		// Body is the direct value associated with the "body" key.
		let body_value = args.get(&*BODY_NAME).cloned();

		// --- URL Construction ---
		let mut path = info.path.clone();
		// Substitute path parameters into the path template.
		for (key, value) in &path_params {
			match value {
				Value::String(s_val) => {
					path = path.replace(&format!("{{{}}}", key), s_val);
				},
				Value::Number(n_val) => {
					path = path.replace(&format!("{{{}}}", key), n_val.to_string().as_str());
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
			self.scheme, self.host, self.port, self.prefix, path
		);

		// Prepare query parameters for reqwest, filtering out non-string values.
		let query_params_reqwest: Vec<(String, String)> = query_params
			.into_iter()
			.filter_map(|(k, v)| match v.as_str() {
				Some(s) => Some((k.clone(), s.to_string())),
				None => {
					tracing::warn!(
						"Query parameter '{}' for tool '{}' is not a string (value: {:?}), skipping",
						k,
						name,
						v
					);
					None
				},
			})
			.collect();

		// --- Header Construction ---
		let mut headers = HeaderMap::new();
		headers.insert(ACCEPT, HeaderValue::from_str("application/json").unwrap());
		// Build the header map, ensuring keys and values are valid.
		for (key, value) in &header_params {
			if let Some(s_val) = value.as_str() {
				// Headers must be strings.
				match (
					HeaderName::from_bytes(key.as_bytes()),
					HeaderValue::from_str(s_val),
				) {
					(Ok(h_name), Ok(h_value)) => {
						headers.insert(h_name, h_value);
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

		// --- Request Building ---
		// Parse the HTTP method.
		let method = Method::from_bytes(info.method.to_uppercase().as_bytes()).map_err(|e| {
			anyhow::anyhow!(
				"Invalid HTTP method '{}' for tool '{}': {}",
				info.method,
				name,
				e
			)
		})?;

		// Start building the request with method and URL.
		tracing::info!("base_url: {}", base_url);
		let url = reqwest::Url::parse(&base_url)?;
		let mut request_builder = self.client.request(method, url);

		// Add query parameters if any exist.
		if !query_params_reqwest.is_empty() {
			request_builder = request_builder.query(&query_params_reqwest);
		}

		// Add headers if any exist.
		if !headers.is_empty() {
			request_builder = request_builder.headers(headers);
		}

		// Add JSON body if `body_value` is Some.
		if let Some(body_val) = body_value {
			request_builder = request_builder.json(&body_val);
		}

		tracing::info!("Sending request: {:?}", request_builder);

		// --- Send Request & Get Response ---
		let response = request_builder.send().await?;
		let status = response.status();
		// Read the response body as text.
		let response_text = response.text().await?;

		// Check if the request was successful.
		if status.is_success() {
			Ok(response_text)
		} else {
			// Return an error if the status code indicates failure.
			Err(anyhow::anyhow!(
				"Upstream API call for tool '{}' failed with status {}: {}",
				name,
				status,
				response_text
			))
		}
	}

	pub fn tools(&self) -> Vec<Tool> {
		self.tools.clone().into_iter().map(|(t, _)| t).collect()
	}
}

#[test]
fn test_parse_openapi_schema() {
	let schema = include_bytes!("../../examples/openapi/openapi.json");
	let schema: OpenAPI = serde_json::from_slice(schema).unwrap();
	let tools = parse_openapi_schema(&schema).unwrap();
	assert_eq!(tools.len(), 19);
	for (tool, upstream) in tools {
		println!("{}", serde_json::to_string_pretty(&tool).unwrap());
		println!("{}", serde_json::to_string_pretty(&upstream).unwrap());
	}
}

#[cfg(test)]
mod tests {
	use super::*; // Import items from parent module
	use reqwest::Client;
	use rmcp::model::Tool;
	use serde_json::json;
	use std::borrow::Cow;
	use std::sync::Arc;
	use wiremock::matchers::{body_json, header, method, path, query_param};
	use wiremock::{Mock, MockServer, ResponseTemplate};

	// Helper to create a handler and mock server for tests
	async fn setup() -> (MockServer, Handler) {
		let server = MockServer::start().await;
		let host = server.uri();
		let client = Client::new();

		// Define a sample tool for testing
		let test_tool_get = Tool {
			name: Cow::Borrowed("get_user"),
			description: Some(Cow::Borrowed("Get user details")), // Added description
			input_schema: Arc::new(
				json!({ // Define a simple schema for testing
						"type": "object",
						"properties": {
								"path": {
										"type": "object",
										"properties": {
												"user_id": {"type": "string"}
										},
										"required": ["user_id"]
								},
								"query": {
										"type": "object",
										"properties": {
												"verbose": {"type": "string"}
										}
								},
								"header": {
										"type": "object",
										"properties": {
												"X-Request-ID": {"type": "string"}
										}
								}
						},
						"required": ["path"] // Only path is required for this tool
				})
				.as_object()
				.unwrap()
				.clone(),
			),
			annotations: None,
		};
		let upstream_call_get = UpstreamOpenAPICall {
			method: "GET".to_string(),
			path: "/users/{user_id}".to_string(),
		};

		let test_tool_post = Tool {
			name: Cow::Borrowed("create_user"),
			description: Some(Cow::Borrowed("Create a new user")), // Added description
			input_schema: Arc::new(
				json!({ // Schema for POST
						"type": "object",
						"properties": {
								"body": {
										"type": "object",
										"properties": {
												"name": {"type": "string"},
												"email": {"type": "string"}
										},
										"required": ["name", "email"]
								},
								 "query": {
										"type": "object",
										"properties": {
												"source": {"type": "string"}
										}
								},
								 "header": {
										"type": "object",
										"properties": {
												"X-API-Key": {"type": "string"}
										}
								}
						},
						"required": ["body"] // Body is required
				})
				.as_object()
				.unwrap()
				.clone(),
			),
			annotations: None,
		};
		let upstream_call_post = UpstreamOpenAPICall {
			method: "POST".to_string(),
			path: "/users".to_string(),
		};

		let parsed = reqwest::Url::parse(&host).unwrap();

		let handler = Handler {
			scheme: parsed.scheme().to_string(),
			host: parsed.host().unwrap().to_string(),
			prefix: "".to_string(),
			port: parsed.port().unwrap_or(8080),
			client,
			tools: vec![
				(test_tool_get, upstream_call_get),
				(test_tool_post, upstream_call_post),
			],
		};

		(server, handler)
	}

	#[tokio::test]
	async fn test_call_tool_get_simple_success() {
		let (server, handler) = setup().await;

		let user_id = "123";
		let expected_response = json!({ "id": user_id, "name": "Test User" });

		Mock::given(method("GET"))
			.and(path(format!("/users/{}", user_id)))
			.respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
			.mount(&server)
			.await;

		let args = json!({ "path": { "user_id": user_id } });
		let result = handler
			.call_tool("get_user", Some(args.as_object().unwrap().clone()))
			.await;

		assert!(result.is_ok());
		assert_eq!(result.unwrap(), expected_response.to_string());
	}

	#[tokio::test]
	async fn test_call_tool_get_with_query() {
		let (server, handler) = setup().await;

		let user_id = "456";
		let verbose_flag = "true";
		let expected_response =
			json!({ "id": user_id, "name": "Test User", "details": "Verbose details" });

		Mock::given(method("GET"))
			.and(path(format!("/users/{}", user_id)))
			.and(query_param("verbose", verbose_flag))
			.respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
			.mount(&server)
			.await;

		let args = json!({ "path": { "user_id": user_id }, "query": { "verbose": verbose_flag } });
		let result = handler
			.call_tool("get_user", Some(args.as_object().unwrap().clone()))
			.await;

		assert!(result.is_ok());
		assert_eq!(result.unwrap(), expected_response.to_string());
	}

	#[tokio::test]
	async fn test_call_tool_get_with_header() {
		let (server, handler) = setup().await;

		let user_id = "789";
		let request_id = "req-abc";
		let expected_response = json!({ "id": user_id, "name": "Another User" });

		Mock::given(method("GET"))
			.and(path(format!("/users/{}", user_id)))
			.and(header("X-Request-ID", request_id))
			.respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
			.mount(&server)
			.await;

		let args = json!({ "path": { "user_id": user_id }, "header": { "X-Request-ID": request_id } });
		let result = handler
			.call_tool("get_user", Some(args.as_object().unwrap().clone()))
			.await;

		assert!(result.is_ok());
		assert_eq!(result.unwrap(), expected_response.to_string());
	}

	#[tokio::test]
	async fn test_call_tool_post_with_body() {
		let (server, handler) = setup().await;

		let request_body = json!({ "name": "New User", "email": "new@example.com" });
		let expected_response = json!({ "id": "xyz", "name": "New User", "email": "new@example.com" });

		Mock::given(method("POST"))
			.and(path("/users"))
			.and(body_json(&request_body))
			.respond_with(ResponseTemplate::new(201).set_body_json(&expected_response))
			.mount(&server)
			.await;

		let args = json!({ "body": request_body });
		let result = handler
			.call_tool("create_user", Some(args.as_object().unwrap().clone()))
			.await;

		assert!(result.is_ok());
		assert_eq!(result.unwrap(), expected_response.to_string());
	}

	#[tokio::test]
	async fn test_call_tool_post_all_params() {
		let (server, handler) = setup().await;

		let request_body = json!({ "name": "Complete User", "email": "complete@example.com" });
		let api_key = "secret-key";
		let source = "test-suite";
		let expected_response = json!({ "id": "comp-123", "name": "Complete User" });

		Mock::given(method("POST"))
			.and(path("/users"))
			.and(query_param("source", source))
			.and(header("X-API-Key", api_key))
			.and(body_json(&request_body))
			.respond_with(ResponseTemplate::new(201).set_body_json(&expected_response))
			.mount(&server)
			.await;

		let args = json!({
				"body": request_body,
				"query": { "source": source },
				"header": { "X-API-Key": api_key }
		});
		let result = handler
			.call_tool("create_user", Some(args.as_object().unwrap().clone()))
			.await;

		assert!(result.is_ok());
		assert_eq!(result.unwrap(), expected_response.to_string());
	}

	#[tokio::test]
	async fn test_call_tool_tool_not_found() {
		let (_server, handler) = setup().await; // Mock server not needed

		let args = json!({});
		let result = handler
			.call_tool("nonexistent_tool", Some(args.as_object().unwrap().clone()))
			.await;

		assert!(result.is_err());
		assert!(
			result
				.unwrap_err()
				.to_string()
				.contains("tool nonexistent_tool not found")
		);
	}

	#[tokio::test]
	async fn test_call_tool_upstream_error() {
		let (server, handler) = setup().await;

		let user_id = "error-user";
		let error_response = json!({ "error": "User not found" });

		Mock::given(method("GET"))
			.and(path(format!("/users/{}", user_id)))
			.respond_with(ResponseTemplate::new(404).set_body_json(&error_response))
			.mount(&server)
			.await;

		let args = json!({ "path": { "user_id": user_id } });
		let result = handler
			.call_tool("get_user", Some(args.as_object().unwrap().clone()))
			.await;

		assert!(result.is_err());
		let err = result.unwrap_err();
		assert!(err.to_string().contains("failed with status 404 Not Found"));
		assert!(err.to_string().contains(&error_response.to_string()));
	}

	#[tokio::test]
	async fn test_call_tool_invalid_header_value() {
		let (server, handler) = setup().await;

		let user_id = "header-issue";
		// Mock is set up but won't be hit because header construction fails client-side
		Mock::given(method("GET"))
			.and(path(format!("/users/{}", user_id)))
			.respond_with(ResponseTemplate::new(200).set_body_json(json!({ "id": user_id })))
			.mount(&server)
			.await;

		// Intentionally provide a non-string header value
		let args = json!({
				"path": { "user_id": user_id },
				"header": { "X-Request-ID": 12345 } // Invalid header value (not a string)
		});

		// We expect the call to succeed, but the invalid header should be skipped (and logged)
		// The mock doesn't expect the header, so if the request goes through without it, it passes.
		let result = handler
			.call_tool("get_user", Some(args.as_object().unwrap().clone()))
			.await;
		assert!(result.is_ok()); // Check that the call still succeeds despite the bad header
		assert_eq!(result.unwrap(), json!({ "id": user_id }).to_string());
		// We can't easily assert the log message here, but manual inspection of logs would show the warning.
	}

	#[tokio::test]
	async fn test_call_tool_invalid_query_param_value() {
		let (server, handler) = setup().await;

		let user_id = "query-issue";
		// Mock is set up but won't be hit with the invalid query param
		Mock::given(method("GET"))
			.and(path(format!("/users/{}", user_id)))
			// IMPORTANT: We don't .and(query_param(...)) here because the invalid param is skipped
			.respond_with(ResponseTemplate::new(200).set_body_json(json!({ "id": user_id })))
			.mount(&server)
			.await;

		// Intentionally provide a non-string query value
		let args = json!({
				"path": { "user_id": user_id },
				"query": { "verbose": true } // Invalid query value (not a string)
		});

		// We expect the call to succeed, but the invalid query param should be skipped (and logged)
		let result = handler
			.call_tool("get_user", Some(args.as_object().unwrap().clone()))
			.await;
		assert!(result.is_ok());
		assert_eq!(result.unwrap(), json!({ "id": user_id }).to_string());
	}

	#[tokio::test]
	async fn test_call_tool_invalid_path_param_value() {
		let (server, handler) = setup().await;

		let invalid_user_id = json!(12345); // Not a string
		// Mock is set up for the *literal* path, as substitution will fail
		Mock::given(method("GET"))
			.and(path("/users/{user_id}")) // Path doesn't get substituted
			.respond_with(
				ResponseTemplate::new(404) // Or whatever the server does with a literal {user_id}
					.set_body_string("Not Found - Literal Path"),
			)
			.mount(&server)
			.await;

		let args = json!({
				"path": { "user_id": invalid_user_id }
		});

		// The call might succeed at the HTTP level but might return an error from the server,
		// or potentially fail if the path is fundamentally invalid after non-substitution.
		// Here we assume the server returns 404 for the literal path.
		let result = handler
			.call_tool("get_user", Some(args.as_object().unwrap().clone()))
			.await;

		// Depending on server behavior for the literal path, this might be Ok or Err.
		// If server returns 404 for the literal path:
		assert!(result.is_err());
		assert!(
			result
				.unwrap_err()
				.to_string()
				.contains("failed with status 404 Not Found")
		);

		// If the request *itself* failed before sending (e.g., invalid URL formed),
		// the error might be different.
	}
}
