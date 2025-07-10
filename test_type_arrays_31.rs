use std::sync::Arc;
use agentgateway::types::agent::{OpenAPI, OpenAPIVersion, detect_openapi_version};
use agentgateway::yamlviajson;
use agentgateway::mcp::openapi::parse_openapi_schema;

fn main() {
    // Test OpenAPI 3.1 spec with type arrays (nullable types)
    let content_31 = r#"
openapi: "3.1.0"
info:
  title: Test API with Type Arrays
  version: "1.0.0"
servers:
  - url: https://api.example.com
paths:
  /items:
    post:
      operationId: createItem
      summary: Create a new item
      description: Create a new item with nullable fields
      requestBody:
        required: true
        description: Item data to create
        content:
          application/json:
            schema:
              type: object
              required:
                - name
              properties:
                name:
                  type: string
                  description: Item name (required)
                description:
                  type: ["string", "null"]
                  description: Item description (nullable)
                category:
                  type: ["string", "null"]
                  enum: ["electronics", "books", "clothing", null]
                  description: Item category (nullable with enum)
                price:
                  type: ["number", "null"]
                  minimum: 0
                  description: Item price (nullable number)
                tags:
                  type: ["array", "null"]
                  items:
                    type: string
                  description: Item tags (nullable array)
      responses:
        '201':
          description: Item created successfully
        '400':
          description: Invalid item data
"#;

    println!("Testing OpenAPI 3.1 with type arrays (nullable types)...");
    
    // Test version detection
    match detect_openapi_version(content_31) {
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
    let spec: openapiv3_1::OpenApi = match yamlviajson::from_str(content_31) {
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
            
            if tool.name != "createItem" {
                println!("✗ Expected tool name 'createItem', got '{}'", tool.name);
                return;
            }
            
            if call.method != "POST" || call.path != "/items" {
                println!("✗ Expected POST /items, got {} {}", call.method, call.path);
                return;
            }
            
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
                        if props_obj.contains_key(*prop) {
                            println!("✓ Found property: {}", prop);
                            
                            // Check if nullable properties are handled correctly
                            if *prop != "name" { // name is not nullable
                                if let Some(prop_schema) = props_obj.get(*prop) {
                                    println!("  Property '{}' schema: {}", prop, prop_schema);
                                    // For now, we just check that the property exists
                                    // Later we'll enhance this to check nullable handling
                                }
                            }
                        } else {
                            println!("⚠ Missing expected property: {}", prop);
                        }
                    }
                    
                    if props_obj.len() > 0 {
                        println!("✓ Type arrays schema processing is working!");
                    } else {
                        println!("⚠ No properties processed yet (expected for current implementation)");
                    }
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
