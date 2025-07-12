//! OpenAPI 3.0 specification behavior implementation

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use rmcp::model::{JsonObject, Tool};
use serde_json::{Value, json};
use openapiv3::{OpenAPI as OpenAPIv3, Parameter as Parameterv3, ReferenceOr as ReferenceOrv3, RequestBody as RequestBodyv3, Schema as Schemav3};

use super::{ParseError, UpstreamOpenAPICall, BODY_NAME, ParameterType};
use super::compatibility::{CompatibleSchema, CompatibleParameter, CompatibleRequestBody, ToCompatible, ParameterLocation};
use super::specification::{OpenAPISpecification, SchemaResolver, SchemaBuilder, CommonBehavior};

/// OpenAPI 3.0 specification behavior
pub struct OpenAPI30Specification {
    spec: Arc<OpenAPIv3>,
}

impl OpenAPI30Specification {
    pub fn new(spec: Arc<OpenAPIv3>) -> Self {
        Self { spec }
    }
    
    /// Resolve a schema reference to the actual schema
    fn resolve_schema_ref<'a>(&'a self, reference: &'a ReferenceOrv3<Schemav3>) -> Result<&'a Schemav3, ParseError> {
        match reference {
            ReferenceOrv3::Reference { reference } => {
                let reference = reference
                    .strip_prefix("#/components/schemas/")
                    .ok_or(ParseError::InvalidReference(reference.to_string()))?;
                let components = self.spec
                    .components
                    .as_ref()
                    .ok_or(ParseError::MissingComponents)?;
                let schema = components
                    .schemas
                    .get(reference)
                    .ok_or(ParseError::MissingReference(reference.to_string()))?;
                self.resolve_schema_ref(schema)
            },
            ReferenceOrv3::Item(schema) => Ok(schema),
        }
    }
    
    /// Resolve a parameter reference to the actual parameter
    fn resolve_parameter_ref<'a>(&'a self, reference: &'a ReferenceOrv3<Parameterv3>) -> Result<&'a Parameterv3, ParseError> {
        match reference {
            ReferenceOrv3::Reference { reference } => {
                let reference = reference
                    .strip_prefix("#/components/parameters/")
                    .ok_or(ParseError::MissingReference(reference.to_string()))?;
                let components = self.spec
                    .components
                    .as_ref()
                    .ok_or(ParseError::MissingComponents)?;
                let parameter = components
                    .parameters
                    .get(reference)
                    .ok_or(ParseError::MissingReference(reference.to_string()))?;
                self.resolve_parameter_ref(parameter)
            },
            ReferenceOrv3::Item(parameter) => Ok(parameter),
        }
    }
    
    /// Resolve a request body reference to the actual request body
    fn resolve_request_body_ref<'a>(&'a self, reference: &'a ReferenceOrv3<RequestBodyv3>) -> Result<&'a RequestBodyv3, ParseError> {
        match reference {
            ReferenceOrv3::Reference { reference } => {
                let reference = reference
                    .strip_prefix("#/components/requestBodies/")
                    .ok_or(ParseError::MissingReference(reference.to_string()))?;
                let components = self.spec
                    .components
                    .as_ref()
                    .ok_or(ParseError::MissingComponents)?;
                let request_body = components
                    .request_bodies
                    .get(reference)
                    .ok_or(ParseError::MissingReference(reference.to_string()))?;
                self.resolve_request_body_ref(request_body)
            },
            ReferenceOrv3::Item(request_body) => Ok(request_body),
        }
    }
}

impl OpenAPISpecification for OpenAPI30Specification {
    fn parse_schema(&self) -> Result<Vec<(Tool, UpstreamOpenAPICall)>, ParseError> {
        let tool_defs: Result<Vec<_>, _> = self.spec
            .paths
            .iter()
            .map(|(path, path_info)| -> Result<Vec<(Tool, UpstreamOpenAPICall)>, ParseError> {
                let item = path_info
                    .as_item()
                    .ok_or(ParseError::UnsupportedReference(path.to_string()))?;
                let items: Result<Vec<_>, _> = item
                    .iter()
                    .map(|(method, op)| -> Result<(Tool, UpstreamOpenAPICall), ParseError> {
                        let name = op
                            .operation_id
                            .clone()
                            .ok_or(ParseError::InformationRequired(format!(
                                "operation_id is required for {path}"
                            )))?;

                        // Build the schema using the compatibility layer
                        let mut final_schema_components = HashMap::new();
                        let mut required_fields = Vec::new();

                        // Handle request body
                        if let Some(body_ref) = &op.request_body {
                            let body = self.resolve_request_body_ref(body_ref)?;
                            if let Some(media_type) = body.content.get("application/json") {
                                if let Some(schema_ref) = &media_type.schema {
                                    let schema = self.resolve_schema_ref(schema_ref)?;
                                    let compatible_schema = schema.to_compatible()?;
                                    let body_schema = serde_json::to_value(compatible_schema)
                                        .map_err(ParseError::SerdeError)?;
                                    
                                    if body.required {
                                        required_fields.push(BODY_NAME.clone());
                                    }
                                    final_schema_components.insert(BODY_NAME.clone(), body_schema);
                                }
                            }
                        }

                        // Handle parameters
                        let mut param_schemas: HashMap<ParameterType, Vec<(String, Value, bool)>> = HashMap::new();
                        
                        for param_ref in &op.parameters {
                            let param = self.resolve_parameter_ref(param_ref)?;
                            let compatible_param = param.to_compatible()?;
                            
                            let param_type = match compatible_param.location {
                                ParameterLocation::Header => ParameterType::Header,
                                ParameterLocation::Query => ParameterType::Query,
                                ParameterLocation::Path => ParameterType::Path,
                                ParameterLocation::Cookie => return Err(ParseError::UnsupportedReference(
                                    "parameter type COOKIE is not supported".to_string(),
                                )),
                            };
                            
                            let schema_value = serde_json::to_value(&compatible_param.schema)
                                .map_err(ParseError::SerdeError)?;
                            
                            param_schemas
                                .entry(param_type)
                                .or_insert_with(Vec::new)
                                .push((compatible_param.name, schema_value, compatible_param.required));
                        }

                        // Build parameter schemas
                        for (param_type, params) in param_schemas {
                            let mut param_properties = JsonObject::new();
                            let mut param_required = Vec::new();
                            
                            for (name, schema, required) in params {
                                param_properties.insert(name.clone(), schema);
                                if required {
                                    param_required.push(name);
                                }
                            }
                            
                            let param_schema = json!({
                                "type": "object",
                                "properties": param_properties,
                                "required": param_required
                            });
                            
                            if !param_required.is_empty() {
                                required_fields.push(param_type.to_string());
                            }
                            final_schema_components.insert(param_type.to_string(), param_schema);
                        }

                        // Build final schema
                        let final_schema = CommonBehavior::build_json_schema_from_components(
                            &final_schema_components,
                            &required_fields,
                        )?;

                        let tool = Tool {
                            annotations: None,
                            name: Cow::Owned(name.clone()),
                            description: Some(Cow::Owned(
                                op.description
                                    .as_ref()
                                    .unwrap_or_else(|| op.summary.as_ref().unwrap_or(&name))
                                    .to_string(),
                            )),
                            input_schema: Arc::new(final_schema),
                        };
                        
                        let upstream = UpstreamOpenAPICall {
                            method: method.to_string(),
                            path: path.clone(),
                        };
                        
                        Ok((tool, upstream))
                    })
                    .collect();
                items
            })
            .collect();

        match tool_defs {
            Ok(tool_defs) => Ok(tool_defs.into_iter().flatten().collect()),
            Err(e) => Err(e),
        }
    }

    fn get_server_prefix(&self) -> Result<String, ParseError> {
        match self.spec.servers.len() {
            0 => Ok("/".to_string()),
            1 => Ok(self.spec.servers[0].url.clone()),
            _ => Err(ParseError::UnsupportedReference(format!(
                "multiple servers are not supported: {:?}",
                self.spec.servers
            ))),
        }
    }

    fn version(&self) -> String {
        self.spec.openapi.clone()
    }
}

impl SchemaResolver for OpenAPI30Specification {
    fn resolve_schema(&self, reference: &str) -> Result<CompatibleSchema, ParseError> {
        let components = self.spec
            .components
            .as_ref()
            .ok_or(ParseError::MissingComponents)?;
        let schema = components
            .schemas
            .get(reference)
            .ok_or(ParseError::MissingReference(reference.to_string()))?;
        let resolved_schema = self.resolve_schema_ref(schema)?;
        resolved_schema.to_compatible()
    }

    fn resolve_parameter(&self, reference: &str) -> Result<CompatibleParameter, ParseError> {
        let components = self.spec
            .components
            .as_ref()
            .ok_or(ParseError::MissingComponents)?;
        let parameter = components
            .parameters
            .get(reference)
            .ok_or(ParseError::MissingReference(reference.to_string()))?;
        let resolved_parameter = self.resolve_parameter_ref(parameter)?;
        resolved_parameter.to_compatible()
    }

    fn resolve_request_body(&self, reference: &str) -> Result<CompatibleRequestBody, ParseError> {
        let components = self.spec
            .components
            .as_ref()
            .ok_or(ParseError::MissingComponents)?;
        let request_body = components
            .request_bodies
            .get(reference)
            .ok_or(ParseError::MissingReference(reference.to_string()))?;
        let resolved_request_body = self.resolve_request_body_ref(request_body)?;
        
        // Convert to CompatibleRequestBody
        let mut content = HashMap::new();
        for (media_type, media_type_obj) in &resolved_request_body.content {
            let schema = if let Some(schema_ref) = &media_type_obj.schema {
                let resolved_schema = self.resolve_schema_ref(schema_ref)?;
                Some(resolved_schema.to_compatible()?)
            } else {
                None
            };
            
            let compatible_media_type = super::compatibility::CompatibleMediaType {
                schema,
                example: media_type_obj.example.clone(),
                examples: media_type_obj.examples.iter().map(|(k, v)| {
                    let example_value = match v {
                        ReferenceOrv3::Item(example) => example.value.clone().unwrap_or(Value::Null),
                        ReferenceOrv3::Reference { .. } => Value::Null,
                    };
                    (k.clone(), example_value)
                }).collect(),
            };
            
            content.insert(media_type.clone(), compatible_media_type);
        }
        
        Ok(CompatibleRequestBody {
            description: resolved_request_body.description.clone(),
            required: resolved_request_body.required,
            content,
        })
    }
}

impl SchemaBuilder for OpenAPI30Specification {
    fn build_schema_property(&self, parameter: &CompatibleParameter) -> Result<(String, JsonObject, bool), ParseError> {
        let mut schema = serde_json::to_value(&parameter.schema)
            .map_err(ParseError::SerdeError)?
            .as_object()
            .ok_or(ParseError::UnsupportedReference(format!(
                "parameter {} schema is not an object",
                parameter.name
            )))?
            .clone();

        if let Some(desc) = &parameter.description {
            schema.insert("description".to_string(), json!(desc));
        }

        Ok((parameter.name.clone(), schema, parameter.required))
    }

    fn build_json_schema(&self, components: &HashMap<String, Value>) -> Result<JsonObject, ParseError> {
        CommonBehavior::build_json_schema_from_components(components, &[])
    }
}
