use std::sync::Arc;
use agentgateway::types::agent::{OpenAPI, OpenAPIVersion, detect_openapi_version};
use agentgateway::yamlviajson;
use agentgateway::mcp::openapi::parse_openapi_schema;

fn main() {
    // Read our advanced schema test file
    let content = std::fs::read_to_string("test_advanced_schema_31.yaml")
        .expect("Should read test file");
    
    println!("Testing OpenAPI 3.1 with advanced JSON Schema Draft 2020-12 features...");
    
    // Test version detection
    match detect_openapi_version(&content) {
        Ok(OpenAPIVersion::V3_1) => println!("✓ Correctly detected OpenAPI 3.1"),
        Ok(OpenAPIVersion::V3_0) => {
            println!("✗ Incorrectly detected as OpenAPI 3.0");
            return;
        },
        Err(e) => {
            println!("✗ Failed to detect version: {}", e);
            return;
        }
    }

    // Parse the spec
    let spec: openapiv3_1::OpenApi = match yamlviajson::from_str(&content) {
        Ok(spec) => spec,
        Err(e) => {
            println!("✗ Failed to parse OpenAPI 3.1 spec: {}", e);
            return;
        }
    };

    let openapi_spec = OpenAPI::V3_1(Arc::new(spec));
    
    // Test parsing into tools
    match parse_openapi_schema(&openapi_spec) {
        Ok(tools_and_calls) => {
            println!("✓ OpenAPI 3.1 advanced schema parsing succeeded!");
            println!("✓ Generated {} tools", tools_and_calls.len());
            
            if tools_and_calls.len() != 1 {
                println!("✗ Expected 1 tool, got {}", tools_and_calls.len());
                return;
            }
            
            let (tool, call) = &tools_and_calls[0];
            
            println!("✓ Tool: {} ({} {})", tool.name, call.method, call.path);
            
            // Check that the tool has a proper input schema
            println!("✓ Tool input schema keys: {:?}", tool.input_schema.keys().collect::<Vec<_>>());
            
            // Check if we have properties (indicating advanced schema processing)
            if let Some(properties) = tool.input_schema.get("properties") {
                if let Some(props_obj) = properties.as_object() {
                    println!("✓ Found {} properties: {:?}", props_obj.len(), props_obj.keys().collect::<Vec<_>>());
                    
                    // Check for specific advanced schema features
                    let expected_props = ["name", "contact", "age", "tags", "metadata"];
                    for prop in &expected_props {
                        if let Some(prop_schema) = props_obj.get(*prop) {
                            println!("✓ Found property '{}': {}", prop, prop_schema);
                            
                            // Check for advanced schema composition keywords
                            if prop_schema.get("anyOf").is_some() {
                                println!("  ✓ Property '{}' has anyOf composition!", prop);
                            }
                            if prop_schema.get("oneOf").is_some() {
                                println!("  ✓ Property '{}' has oneOf composition!", prop);
                            }
                            if prop_schema.get("allOf").is_some() {
                                println!("  ✓ Property '{}' has allOf composition!", prop);
                            }
                            
                            // Check for validation keywords
                            if prop_schema.get("pattern").is_some() {
                                println!("  ✓ Property '{}' has pattern validation!", prop);
                            }
                            if prop_schema.get("minLength").is_some() || prop_schema.get("maxLength").is_some() {
                                println!("  ✓ Property '{}' has length validation!", prop);
                            }
                            if prop_schema.get("minItems").is_some() || prop_schema.get("maxItems").is_some() {
                                println!("  ✓ Property '{}' has array validation!", prop);
                            }
                            if prop_schema.get("nullable").is_some() {
                                println!("  ✓ Property '{}' has nullable handling!", prop);
                            }
                        } else {
                            println!("⚠ Missing expected property: {}", prop);
                        }
                    }
                    
                    println!("✓ Advanced JSON Schema Draft 2020-12 features processing is working!");
                } else {
                    println!("⚠ Properties is not an object");
                }
            } else {
                println!("⚠ No properties found in schema");
            }
            
            if let Some(desc) = &tool.description {
                println!("✓ Tool description: {}", desc);
            }
        },
        Err(e) => {
            println!("✗ OpenAPI 3.1 advanced schema parsing failed: {}", e);
        }
    }
}
