use openapiv3::{OpenAPI, ReferenceOr, Schema, Parameter};
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
		backend_auth: Option<backend::BackendAuthConfig>,
	},
	Stdio {
		cmd: String,
		args: Vec<String>,
		env: HashMap<String, String>,
	},
	OpenAPI {
		host: String,
		port: u32,
		tools: Vec<(Tool, UpstreamOpenAPICall)>,
	},
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UpstreamOpenAPICall {
	pub method: String, // TODO: Switch to Method, but will require getting rid of Serialize/Deserialize
	pub path: String,
	// todo: params
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
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

pub fn schema_to_tools(schema_bytes: &Vec<u8>) -> Result<Vec<(Tool, UpstreamOpenAPICall)>, ParseError> {
  let schema: OpenAPI = serde_json::from_slice(schema_bytes).map_err(ParseError::SerdeError)?;
  parse_openapi_schema(&schema)
}

fn resolve_schema<'a>(reference: &'a ReferenceOr<Schema>, doc: &'a OpenAPI) -> Result<&'a Schema, ParseError> {
  match reference {
    ReferenceOr::Reference { reference } => {
      let reference = reference.strip_prefix("#/components/schemas/").ok_or(ParseError::MissingReference)?;
      let components: &openapiv3::Components = doc.components.as_ref().ok_or(ParseError::MissingComponents)?;
      let schema = components.schemas.get(reference).ok_or(ParseError::MissingComponent)?;
      resolve_schema(schema, doc)
    }
    ReferenceOr::Item(schema) => Ok(schema),
  }
}

fn resolve_parameter<'a>(reference: &'a ReferenceOr<Parameter>, doc: &'a OpenAPI) -> Result<&'a Parameter, ParseError> {
  match reference {
    ReferenceOr::Reference { reference } => {
      let reference = reference.strip_prefix("#/components/parameters/").ok_or(ParseError::MissingReference)?;
      let components: &openapiv3::Components = doc.components.as_ref().ok_or(ParseError::MissingComponents)?;
      let parameter = components.parameters.get(reference).ok_or(ParseError::MissingComponent)?;
      resolve_parameter(parameter, doc)
    }
    ReferenceOr::Item(parameter) => Ok(parameter),
  }
}


fn parse_openapi_schema(open_api: &OpenAPI) -> Result<Vec<(Tool, UpstreamOpenAPICall)>, ParseError> {
	let tool_defs: Result<Vec<_>, _> =  open_api
		.paths
		.iter()
		.map(|(path, path_info)| -> Result<Vec<(Tool, UpstreamOpenAPICall)>, ParseError> {
			let item = path_info.as_item().ok_or(ParseError::UnsupportedReference(path.to_string()))?;
			let items: Result<Vec<_>, _> = item
				.iter()
				.map(|(method, op)| -> Result<(Tool, UpstreamOpenAPICall), ParseError> {
					let name = op.operation_id.clone().ok_or(ParseError::InformationRequired(format!("operation_id is required for {}", path)))?;
					let props: Result<Vec<_>, _>  = op
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
                    .ok_or(ParseError::UnsupportedReference(format!("parameter {} is not an object", p.name)))?
                    .clone()
                }
                openapiv3::ParameterSchemaOrContent::Content(content) => {
                  return Err(ParseError::UnsupportedReference(format!("content is not supported for parameters: {:?}", content)));
                }
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
				})
				.collect();
      // Rust has a hard time with this...
      let items = items?;
      Ok(items)
		})
		.collect();

  match tool_defs {
    Ok(tool_defs) => Ok(tool_defs.into_iter().flatten().collect()),
    Err(e) => Err(e),
  }

}
