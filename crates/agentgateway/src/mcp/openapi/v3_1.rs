//! OpenAPI 3.1 specification behavior implementation

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use rmcp::model::{JsonObject, Tool};
use serde_json::{Value, json};
use openapiv3_1::OpenApi as OpenAPIv3_1;

use super::{ParseError, UpstreamOpenAPICall, BODY_NAME, ParameterType};
use super::compatibility::{CompatibleSchema, CompatibleParameter, CompatibleRequestBody, ParameterLocation, ToCompatible};
use super::specification::{OpenAPISpecification, SchemaResolver, SchemaBuilder, CommonBehavior};

/// OpenAPI 3.1 specification behavior
pub struct OpenAPI31Specification {
    spec: Arc<OpenAPIv3_1>,
}

impl OpenAPI31Specification {
    pub fn new(spec: Arc<OpenAPIv3_1>) -> Self {
        Self { spec }
    }
    
    // TODO: Implement reference resolution methods when we implement the actual 3.1 parsing logic
    // These methods will need to be implemented based on the actual openapiv3_1 crate API structure
}

impl OpenAPISpecification for OpenAPI31Specification {
    fn parse_schema(&self) -> Result<Vec<(Tool, UpstreamOpenAPICall)>, ParseError> {
        // Return a helpful error message indicating that 3.1 support is in progress
        // The specification pattern infrastructure is now complete, but we need to 
        // implement the actual 3.1 parsing logic based on the openapiv3_1 crate API
        Err(ParseError::InformationRequired(
            "OpenAPI 3.1 parsing is currently being implemented using the specification pattern. \
            The compatibility layer and behavior injection system is now in place. \
            The openapiv3_1 crate has a different API structure than initially expected, \
            so the actual parsing logic needs to be implemented based on the real API. \
            Please use OpenAPI 3.0 specifications for now.".to_string()
        ))
    }

    fn get_server_prefix(&self) -> Result<String, ParseError> {
        let empty_vec = Vec::new();
        let servers = self.spec.servers.as_ref().unwrap_or(&empty_vec);
        match servers.len() {
            0 => Ok("/".to_string()),
            1 => Ok(servers[0].url.clone()),
            _ => Err(ParseError::UnsupportedReference(format!(
                "multiple servers are not supported (found {} servers)",
                servers.len()
            ))),
        }
    }

    fn version(&self) -> String {
        "3.1".to_string()
    }
}

impl SchemaResolver for OpenAPI31Specification {
    fn resolve_schema(&self, _reference: &str) -> Result<CompatibleSchema, ParseError> {
        // TODO: Implement OpenAPI 3.1 schema resolution
        // This would involve:
        // 1. Finding the schema in the components section
        // 2. Converting it to a CompatibleSchema using the ToCompatible trait
        // 3. Handling 3.1-specific features like type arrays
        Err(ParseError::InformationRequired(
            "OpenAPI 3.1 schema resolution not yet implemented".to_string()
        ))
    }

    fn resolve_parameter(&self, _reference: &str) -> Result<CompatibleParameter, ParseError> {
        // TODO: Implement OpenAPI 3.1 parameter resolution
        Err(ParseError::InformationRequired(
            "OpenAPI 3.1 parameter resolution not yet implemented".to_string()
        ))
    }

    fn resolve_request_body(&self, _reference: &str) -> Result<CompatibleRequestBody, ParseError> {
        // TODO: Implement OpenAPI 3.1 request body resolution
        Err(ParseError::InformationRequired(
            "OpenAPI 3.1 request body resolution not yet implemented".to_string()
        ))
    }
}

impl SchemaBuilder for OpenAPI31Specification {
    fn build_schema_property(&self, parameter: &CompatibleParameter) -> Result<(String, JsonObject, bool), ParseError> {
        // This can use the same logic as 3.0 since we're working with CompatibleParameter
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
        // This can use the same logic as 3.0 since we're working with compatible types
        CommonBehavior::build_json_schema_from_components(components, &[])
    }
}

// TODO: Implement the actual OpenAPI 3.1 parsing logic
// The key areas that need implementation:
//
// 1. Schema conversion from openapiv3_1::Schema to CompatibleSchema
//    - Handle type arrays like ["string", "null"] -> nullable: true
//    - Convert JSON Schema Draft 2020-12 features to compatible format
//
// 2. Parameter conversion from openapiv3_1 parameter types to CompatibleParameter
//    - Handle the different parameter structure in 3.1
//
// 3. Request body conversion
//    - Handle 3.1-specific request body features
//
// 4. Reference resolution
//    - Implement proper $ref resolution for 3.1 schemas
//
// The specification pattern is now in place, so this implementation can be done
// incrementally while maintaining the same interface as the 3.0 implementation.
