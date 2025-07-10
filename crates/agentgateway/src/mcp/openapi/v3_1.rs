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
    
    /// Create a tool from an OpenAPI 3.1 operation
    fn create_tool_from_operation(
        &self,
        operation_id: &str,
        method: &str,
        path: &str,
        operation: &openapiv3_1::path::Operation,
    ) -> Result<(Tool, UpstreamOpenAPICall), ParseError> {
        let description = operation.summary
            .as_ref()
            .or(operation.description.as_ref())
            .unwrap_or(&format!("{} {}", method, path))
            .clone();
        
        // Process parameters to create input schema
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();
        
        if let Some(parameters) = &operation.parameters {
            for param in parameters {
                match self.process_parameter_v3_1(param)? {
                    Some((name, schema, is_required)) => {
                        if is_required {
                            required.push(name.clone());
                        }
                        properties.insert(name, schema);
                    },
                    None => {
                        // Skip parameters we can't process yet
                        continue;
                    }
                }
            }
        }
        
        // Process request body if present
        if let Some(request_body) = &operation.request_body {
            match self.process_request_body_v3_1(request_body)? {
                Some((body_properties, body_required)) => {
                    // Merge request body properties into the main properties
                    for (key, value) in body_properties {
                        properties.insert(key, value);
                    }
                    // Add required fields from request body
                    for req_field in body_required {
                        if !required.contains(&req_field) {
                            required.push(req_field);
                        }
                    }
                },
                None => {
                    // Skip request body we can't process yet
                }
            }
        }
        
        // Create the input schema
        let mut input_schema = serde_json::Map::new();
        input_schema.insert("type".to_string(), json!("object"));
        input_schema.insert("properties".to_string(), json!(properties));
        input_schema.insert("required".to_string(), json!(required));
        
        let tool = Tool {
            annotations: None,
            name: Cow::Owned(operation_id.to_string()),
            description: Some(Cow::Owned(description)),
            input_schema: Arc::new(input_schema),
        };
        
        let upstream = UpstreamOpenAPICall {
            method: method.to_string(),
            path: path.to_string(),
        };
        
        Ok((tool, upstream))
    }
    
    /// Process an OpenAPI 3.1 parameter and convert it to a JSON schema property
    fn process_parameter_v3_1(
        &self,
        parameter: &openapiv3_1::path::Parameter,
    ) -> Result<Option<(String, Value, bool)>, ParseError> {
        // Try to extract parameter information from the openapiv3_1 parameter structure
        // We'll use serde serialization to understand the structure
        
        // Convert the parameter to JSON to examine its structure
        let param_json = serde_json::to_value(parameter)
            .map_err(|e| ParseError::SerdeError(e))?;
        
        // Try to extract common fields
        let name = param_json.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown_param")
            .to_string();
        
        let required = param_json.get("required")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let description = param_json.get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        // Try to extract schema information
        let mut param_schema = json!({
            "type": "string"  // Default to string
        });
        
        if let Some(schema) = param_json.get("schema") {
            // Process the schema with type array handling
            param_schema = self.normalize_schema_v3_1(schema)?;
        }
        
        // Add description if available
        if let Some(desc) = description {
            param_schema["description"] = json!(desc);
        }
        
        // Try to extract parameter location for debugging
        let location = param_json.get("in")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        
        // Add location info to description for debugging
        if let Some(existing_desc) = param_schema.get("description") {
            param_schema["description"] = json!(format!("{} (in: {})", existing_desc.as_str().unwrap_or(""), location));
        } else {
            param_schema["description"] = json!(format!("Parameter in: {}", location));
        }
        
        Ok(Some((name, param_schema, required)))
    }
    
    /// Process an OpenAPI 3.1 request body and convert it to JSON schema properties
    fn process_request_body_v3_1(
        &self,
        request_body: &openapiv3_1::request_body::RequestBody,
    ) -> Result<Option<(serde_json::Map<String, Value>, Vec<String>)>, ParseError> {
        // Convert the request body to JSON to examine its structure
        let request_body_json = serde_json::to_value(request_body)
            .map_err(|e| ParseError::SerdeError(e))?;
        
        // Try to extract content
        if let Some(content) = request_body_json.get("content") {
            // Look for application/json content type
            if let Some(json_content) = content.get("application/json") {
                if let Some(schema) = json_content.get("schema") {
                    return self.process_schema_v3_1(schema);
                }
            }
            
            // If no application/json, try the first available content type
            if let Some(content_obj) = content.as_object() {
                for (content_type, content_data) in content_obj {
                    if let Some(schema) = content_data.get("schema") {
                        println!("Processing request body with content type: {}", content_type);
                        return self.process_schema_v3_1(schema);
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// Convert OpenAPI 3.1 type arrays to compatible schema format
    /// Handles: type: ["string", "null"] -> type: "string", nullable: true
    fn normalize_schema_v3_1(&self, schema: &Value) -> Result<Value, ParseError> {
        let mut normalized = schema.clone();
        
        // Handle type arrays (key 3.1 feature)
        if let Some(type_value) = schema.get("type") {
            if let Some(type_array) = type_value.as_array() {
                // Convert type array to single type + nullable
                let mut main_type = None;
                let mut is_nullable = false;
                
                for type_item in type_array {
                    if let Some(type_str) = type_item.as_str() {
                        if type_str == "null" {
                            is_nullable = true;
                        } else {
                            main_type = Some(type_str);
                        }
                    }
                }
                
                // Set the main type and nullable flag
                if let Some(main_type_str) = main_type {
                    normalized["type"] = json!(main_type_str);
                    if is_nullable {
                        normalized["nullable"] = json!(true);
                    }
                    
                    println!("✓ Converted type array {:?} to type: '{}', nullable: {}", 
                             type_array, main_type_str, is_nullable);
                } else if is_nullable {
                    // Only null type found
                    normalized["type"] = json!("null");
                }
            }
        }
        
        // Copy other schema properties
        if let Some(format) = schema.get("format") {
            normalized["format"] = format.clone();
        }
        
        if let Some(enum_vals) = schema.get("enum") {
            normalized["enum"] = enum_vals.clone();
        }
        
        if let Some(minimum) = schema.get("minimum") {
            normalized["minimum"] = minimum.clone();
        }
        
        if let Some(maximum) = schema.get("maximum") {
            normalized["maximum"] = maximum.clone();
        }
        
        if let Some(items) = schema.get("items") {
            // Recursively normalize array items
            normalized["items"] = self.normalize_schema_v3_1(items)?;
        }
        
        Ok(normalized)
    }
    
    /// Process an OpenAPI 3.1 schema and convert it to properties and required fields
    fn process_schema_v3_1(
        &self,
        schema: &Value,
    ) -> Result<Option<(serde_json::Map<String, Value>, Vec<String>)>, ParseError> {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();
        
        // Check if this is an object schema
        if let Some(schema_type) = schema.get("type") {
            if schema_type.as_str() == Some("object") {
                // Extract properties
                if let Some(props) = schema.get("properties") {
                    if let Some(props_obj) = props.as_object() {
                        for (prop_name, prop_schema) in props_obj {
                            // Normalize each property schema to handle type arrays
                            let normalized_prop = self.normalize_schema_v3_1(prop_schema)?;
                            properties.insert(prop_name.clone(), normalized_prop);
                        }
                    }
                }
                
                // Extract required fields
                if let Some(req_array) = schema.get("required") {
                    if let Some(req_vec) = req_array.as_array() {
                        for req_item in req_vec {
                            if let Some(req_str) = req_item.as_str() {
                                required.push(req_str.to_string());
                            }
                        }
                    }
                }
                
                return Ok(Some((properties, required)));
            }
        }
        
        // If not an object schema, treat the whole thing as a single property
        // This handles cases where the request body is a simple type
        let normalized_schema = self.normalize_schema_v3_1(schema)?;
        properties.insert("body".to_string(), normalized_schema);
        
        Ok(Some((properties, required)))
    }
    
    // TODO: Implement reference resolution methods when we implement the actual 3.1 parsing logic
    // These methods will need to be implemented based on the actual openapiv3_1 crate API structure
}

impl OpenAPISpecification for OpenAPI31Specification {
    fn parse_schema(&self) -> Result<Vec<(Tool, UpstreamOpenAPICall)>, ParseError> {
        let mut tools = Vec::new();
        
        // Iterate through paths
        for (path, path_item) in &self.spec.paths.paths {
            // Handle GET operations
            if let Some(operation) = &path_item.get {
                if let Some(operation_id) = &operation.operation_id {
                    let tool = self.create_tool_from_operation(
                        operation_id,
                        "GET",
                        path,
                        operation,
                    )?;
                    tools.push(tool);
                }
            }
            
            // Handle POST operations
            if let Some(operation) = &path_item.post {
                if let Some(operation_id) = &operation.operation_id {
                    let tool = self.create_tool_from_operation(
                        operation_id,
                        "POST",
                        path,
                        operation,
                    )?;
                    tools.push(tool);
                }
            }
            
            // Handle PUT operations
            if let Some(operation) = &path_item.put {
                if let Some(operation_id) = &operation.operation_id {
                    let tool = self.create_tool_from_operation(
                        operation_id,
                        "PUT",
                        path,
                        operation,
                    )?;
                    tools.push(tool);
                }
            }
            
            // Handle DELETE operations
            if let Some(operation) = &path_item.delete {
                if let Some(operation_id) = &operation.operation_id {
                    let tool = self.create_tool_from_operation(
                        operation_id,
                        "DELETE",
                        path,
                        operation,
                    )?;
                    tools.push(tool);
                }
            }
            
            // Handle PATCH operations
            if let Some(operation) = &path_item.patch {
                if let Some(operation_id) = &operation.operation_id {
                    let tool = self.create_tool_from_operation(
                        operation_id,
                        "PATCH",
                        path,
                        operation,
                    )?;
                    tools.push(tool);
                }
            }
        }
        
        Ok(tools)
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
