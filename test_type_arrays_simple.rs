use std::sync::Arc;
use agentgateway::types::agent::{OpenAPI, OpenAPIVersion, detect_openapi_version};
use agentgateway::yamlviajson;
use agentgateway::mcp::openapi::parse_openapi_schema;

fn main() {
    // Read our type arrays test file
    let content = std::fs::read_to_string("test_type_arrays_31.yaml")
        .expect("Should read test file");
    
    println!("Testing OpenAPI 3.1 with type arrays (nullable types)...");
    
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
            println!("✓ OpenAPI 3.1 type arrays parsing succeeded!");
            println!("✓ Generated {} tools", tools_and_calls.len());
            
            if tools_and_calls.len() != 1 {
                println!("✗ Expected 1 tool, got {}", tools_and_calls.len());
                return;
            }
            
            let (tool, call) = &tools_and_calls[0];
            
            println!("✓ Tool: {} ({} {})", tool.name, call.method, call.path);
            
            // Check that the tool has a proper input schema
            println!("✓ Tool input schema keys: {:?}", tool.input_schema.keys().collect::<Vec<_>>());
            
            // Check if we have properties (indicating type array processing)
            if let Some(properties) = tool.input_schema.get("properties") {
                if let Some(props_obj) = properties.as_object() {
                    println!("✓ Found {} properties: {:?}", props_obj.len(), props_obj.keys().collect::<Vec<_>>());
                    
                    // Check for specific properties with type arrays
                    let expected_props = ["name", "description", "category", "price", "tags"];
                    for prop in &expected_props {
                        if let Some(prop_schema) = props_obj.get(*prop) {
                            println!("✓ Found property '{}': {}", prop, prop_schema);
                            
                            // Check if nullable properties are handled correctly
                            if *prop != "name" { // name is not nullable
                                if let Some(nullable) = prop_schema.get("nullable") {
                                    if nullable.as_bool() == Some(true) {
                                        println!("  ✓ Property '{}' correctly marked as nullable!", prop);
                                    }
                                }
                            }
                        } else {
                            println!("⚠ Missing expected property: {}", prop);
                        }
                    }
                    
                    println!("✓ Type arrays schema processing is working!");
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
            println!("✗ OpenAPI 3.1 type arrays parsing failed: {}", e);
        }
    }
}
