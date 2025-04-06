use openapiv3::Paths;
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct OpenAPISchema {
	// The crate OpenAPI type requires a lot more, we only need paths for now so use only a subset of it.
	pub paths: Paths,
}

pub fn parse_openapi_schema(schema: &OpenAPISchema) -> Vec<(Tool, UpstreamOpenAPICall)> {
	schema
		.paths
		.iter()
		.flat_map(|(path, path_info)| {
			let item = path_info.as_item().unwrap();
			item
				.iter()
				.map(|(method, op)| {
					let name = op.operation_id.clone().expect("TODO");
					let props: Vec<_> = op
						.parameters
						.iter()
						.map(|p| {
							let item = dbg!(p).as_item().unwrap();
							let p = dbg!(item.parameter_data_ref());
							let mut schema = JsonObject::new();
							if let openapiv3::ParameterSchemaOrContent::Schema(openapiv3::ReferenceOr::Item(s)) =
								&p.format
							{
								schema = serde_json::to_value(s)
									.expect("TODO")
									.as_object()
									.expect("TODO")
									.clone();
							}
							if let Some(desc) = &p.description {
								schema.insert("description".to_string(), json!(desc));
							}

							(p.name.clone(), schema, p.required)
						})
						.collect();
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
					(tool, upstream)
				})
				.collect::<Vec<_>>()
		})
		.collect()
}
