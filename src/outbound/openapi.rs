use crate::xds::mcp::kgateway_dev::target::LocalDataSource;
use crate::xds::mcp::kgateway_dev::target::local_data_source::Source as XdsSource;
use openapiv3::{OpenAPI, Parameter, ReferenceOr, Schema, RequestBody, SchemaKind, Type};
use rmcp::model::JsonObject;
use rmcp::model::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;


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
}

pub (crate) fn get_server_prefix(server: &OpenAPI) -> Result<String, ParseError> {
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
			for vec_ref in [&mut any_schema.one_of, &mut any_schema.all_of, &mut any_schema.any_of] {
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
		SchemaKind::Type(_) => {} // Do nothing, already resolved.
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
pub (crate) fn parse_openapi_schema(
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

              let body: Option<(String, serde_json::Value, bool)> =  match op.request_body.as_ref() {
                Some(body) => {
                  let body = resolve_request_body(&body, open_api)?;
                  match body.content.get("application/json") {
                    Some(media_type) => {
                      let schema_ref = media_type.schema.as_ref().ok_or(ParseError::MissingReference("application/json".to_string()))?;
                      let schema = resolve_nested_schema(&schema_ref, open_api)?;
                      let body_schema = serde_json::to_value(schema).map_err(ParseError::SerdeError)?;
                      if body.required {
                        final_schema.required.push(BODY_NAME.clone());
                      }
                      final_schema.properties.insert(BODY_NAME.clone(), body_schema.clone());
                      Some((BODY_NAME.clone(), body_schema, body.required))
                    }
                    None => None
                  }

                }
                None => None,
              };

              match body {
                Some((name, schema, required)) => {
                  if required {
                    final_schema.required.push(name.clone());
                  }
                  final_schema.properties.insert(name.clone(), schema.clone());
                }
                None => {}
              };


              let mut param_schemas: HashMap<ParameterType, Vec<(String, JsonObject, bool)>> = HashMap::new();
							op
								.parameters
								.iter()
								.try_for_each(|p| -> Result<(), ParseError> {
                  let item = resolve_parameter(p, open_api)?;
                  let (name, schema, required) = build_schema_property(open_api, item)?;
                  match item {
                    Parameter::Header { .. } => {
                      param_schemas.entry(ParameterType::Header).or_insert_with(Vec::new).push((name, schema, required));
                      Ok(())
                    },
                    Parameter::Query { .. } => {
                      param_schemas.entry(ParameterType::Query).or_insert_with(Vec::new).push((name, schema, required));
                      Ok(())
                    },
                    Parameter::Path { .. } => {
                      param_schemas.entry(ParameterType::Path).or_insert_with(Vec::new).push((name, schema, required));
                      Ok(())
                    },
                    _ => {
                      return Err(ParseError::UnsupportedReference(format!(
                        "parameter type COOKIE is not supported",
                      )));
                    },
                  }
								})?;
              
              for (param_type, props) in param_schemas {
                let mut sub_schema = JsonSchema::default();
                sub_schema.required = props
                  .iter()
                  .flat_map(|(name, _, req)| if *req { Some(name.clone()) } else { None })
                  .collect();
                
                if !sub_schema.required.is_empty() {
                  final_schema.required.push(param_type.to_string());
                }
                for (name, s, _) in props {
                  sub_schema.properties.insert(name, json!(s));
                }
                final_schema.properties.insert(param_type.to_string(), json!(sub_schema));
              }

              let final_json = serde_json::to_value(final_schema).map_err(ParseError::SerdeError)?;
              let final_json = final_json.as_object().ok_or(ParseError::UnsupportedReference(format!(
                "final schema is not an object",
              )))?.clone();
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
    write!(f, "{}", match self {
      ParameterType::Header => "header",
      ParameterType::Query => "query",
      ParameterType::Path => "path",
    })
  }
}

fn build_schema_property(open_api: &OpenAPI, item: &Parameter) -> Result<(String, JsonObject, bool), ParseError> {
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

pub (crate) fn resolve_local_data_source(
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
	let schema = include_bytes!("../../examples/openapi/openapi.json");
	let schema: OpenAPI = serde_json::from_slice(schema).unwrap();
	let tools = parse_openapi_schema(&schema).unwrap();
	assert_eq!(tools.len(), 19);
  for (tool, upstream) in tools {
    println!("{}", serde_json::to_string_pretty(&tool).unwrap());
    println!("{}", serde_json::to_string_pretty(&upstream).unwrap());
  }
}

