//! Specification pattern for OpenAPI parsing behaviors
//! 
//! This module defines the traits and behaviors for parsing different OpenAPI versions.
//! Each version implements the same interface but with version-specific logic.

use std::collections::HashMap;
use rmcp::model::{JsonObject, Tool};
use serde_json::Value;

use super::{ParseError, UpstreamOpenAPICall};
use super::compatibility::{CompatibleSchema, CompatibleParameter, CompatibleRequestBody};

/// Trait defining the behavior for parsing OpenAPI specifications
pub trait OpenAPISpecification {
    /// Parse the OpenAPI specification into tools and upstream calls
    fn parse_schema(&self) -> Result<Vec<(Tool, UpstreamOpenAPICall)>, ParseError>;
    
    /// Get the server prefix for this specification
    fn get_server_prefix(&self) -> Result<String, ParseError>;
    
    /// Get the OpenAPI version string
    fn version(&self) -> String;
}

/// Trait for resolving schema references within a specification
pub trait SchemaResolver {
    /// Resolve a schema reference to a compatible schema
    fn resolve_schema(&self, reference: &str) -> Result<CompatibleSchema, ParseError>;
    
    /// Resolve a parameter reference to a compatible parameter
    fn resolve_parameter(&self, reference: &str) -> Result<CompatibleParameter, ParseError>;
    
    /// Resolve a request body reference to a compatible request body
    fn resolve_request_body(&self, reference: &str) -> Result<CompatibleRequestBody, ParseError>;
}

/// Trait for building schema properties from parameters
pub trait SchemaBuilder {
    /// Build a schema property from a compatible parameter
    fn build_schema_property(&self, parameter: &CompatibleParameter) -> Result<(String, JsonObject, bool), ParseError>;
    
    /// Build a complete JSON schema from components
    fn build_json_schema(&self, components: &HashMap<String, Value>) -> Result<JsonObject, ParseError>;
}

/// Common functionality shared between OpenAPI versions
pub struct CommonBehavior;

impl CommonBehavior {
    /// Build a JSON schema from schema components
    pub fn build_json_schema_from_components(
        components: &HashMap<String, Value>,
        required_fields: &[String],
    ) -> Result<JsonObject, ParseError> {
        let mut schema = JsonObject::new();
        schema.insert("type".to_string(), Value::String("object".to_string()));
        schema.insert("required".to_string(), Value::Array(
            required_fields.iter().map(|s| Value::String(s.clone())).collect()
        ));
        schema.insert("properties".to_string(), Value::Object(
            components.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        ));
        Ok(schema)
    }
    
    /// Extract parameter type from location
    pub fn parameter_type_from_location(location: &str) -> Result<String, ParseError> {
        match location {
            "query" => Ok("query".to_string()),
            "path" => Ok("path".to_string()),
            "header" => Ok("header".to_string()),
            "cookie" => Err(ParseError::UnsupportedReference(
                "parameter type COOKIE is not supported".to_string(),
            )),
            _ => Err(ParseError::UnsupportedReference(format!(
                "unsupported parameter location: {}", location
            ))),
        }
    }
}

/// Factory for creating OpenAPI specification behaviors
pub struct OpenAPISpecificationFactory;

impl OpenAPISpecificationFactory {
    /// Create a specification behavior for the given OpenAPI version
    pub fn create_specification(
        openapi: &crate::types::agent::OpenAPI,
    ) -> Box<dyn OpenAPISpecification> {
        match openapi {
            crate::types::agent::OpenAPI::V3_0(spec) => {
                Box::new(super::v3_0::OpenAPI30Specification::new(spec.clone()))
            },
            crate::types::agent::OpenAPI::V3_1(spec) => {
                Box::new(super::v3_1::OpenAPI31Specification::new(spec.clone()))
            },
        }
    }
}
