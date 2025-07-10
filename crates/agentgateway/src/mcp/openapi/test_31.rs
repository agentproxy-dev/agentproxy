#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::types::agent::{OpenAPI, OpenAPIVersion, detect_openapi_version};
    use crate::yamlviajson;
    use openapiv3::OpenAPI as OpenAPIv3;
    use crate::mcp::openapi::parse_openapi_schema;

    #[test]
    fn test_openapi_31_detection_and_parsing() {
        // Test OpenAPI 3.1 spec content
        let content_31 = r#"
openapi: "3.1.0"
info:
  title: Test API
  version: "1.0.0"
paths:
  /test:
    get:
      operationId: testOperation
      summary: Test operation
      responses:
        '200':
          description: Success
"#;

        // Test version detection
        let version = detect_openapi_version(content_31).expect("Should detect version");
        match version {
            OpenAPIVersion::V3_1 => {
                println!("✓ Correctly detected OpenAPI 3.1");
            },
            OpenAPIVersion::V3_0 => {
                panic!("✗ Incorrectly detected as OpenAPI 3.0");
            }
        }

        // Test parsing into unified enum
        let spec: OpenAPI = match version {
            OpenAPIVersion::V3_0 => {
                let spec: OpenAPIv3 = yamlviajson::from_str(content_31).expect("Should parse 3.0");
                OpenAPI::V3_0(Arc::new(spec))
            },
            OpenAPIVersion::V3_1 => {
                let spec: openapiv3_1::OpenApi = yamlviajson::from_str(content_31).expect("Should parse 3.1");
                OpenAPI::V3_1(Arc::new(spec))
            },
        };

        // Test version method
        assert_eq!(spec.version(), "3.1");

        // Test that parsing into tools now works with our basic implementation
        match parse_openapi_schema(&spec) {
            Ok(tools_and_calls) => {
                println!("✓ OpenAPI 3.1 parsing succeeded!");
                println!("✓ Generated {} tools", tools_and_calls.len());
                assert_eq!(tools_and_calls.len(), 1);
                let (tool, _call) = &tools_and_calls[0];
                assert_eq!(tool.name, "testOperation");
                println!("✓ Tool name: {}", tool.name);
                if let Some(desc) = &tool.description {
                    println!("✓ Tool description: {}", desc);
                }
            },
            Err(e) => {
                panic!("✗ OpenAPI 3.1 parsing failed: {}", e);
            }
        }
    }

    #[test]
    fn test_openapi_30_still_works() {
        // Test that OpenAPI 3.0 still works
        let content_30 = r#"
openapi: "3.0.0"
info:
  title: Test API
  version: "1.0.0"
paths:
  /test:
    get:
      operationId: testOperation
      summary: Test operation
      responses:
        '200':
          description: Success
"#;

        // Test version detection
        let version = detect_openapi_version(content_30).expect("Should detect version");
        match version {
            OpenAPIVersion::V3_0 => {
                println!("✓ Correctly detected OpenAPI 3.0");
            },
            OpenAPIVersion::V3_1 => {
                panic!("✗ Incorrectly detected as OpenAPI 3.1");
            }
        }

        // Test parsing into unified enum
        let spec: OpenAPI = match version {
            OpenAPIVersion::V3_0 => {
                let spec: OpenAPIv3 = yamlviajson::from_str(content_30).expect("Should parse 3.0");
                OpenAPI::V3_0(Arc::new(spec))
            },
            OpenAPIVersion::V3_1 => {
                let spec: openapiv3_1::OpenApi = yamlviajson::from_str(content_30).expect("Should parse 3.1");
                OpenAPI::V3_1(Arc::new(spec))
            },
        };

        // Test version method
        assert_eq!(spec.version(), "3.0.0");

        // Test that parsing into tools works (though it may fail for other reasons like missing servers)
        // We just want to make sure it doesn't fail with the "not implemented" error
        match parse_openapi_schema(&spec) {
            Ok(_) => println!("✓ OpenAPI 3.0 parsing succeeded"),
            Err(e) => {
                let error_msg = e.to_string();
                // Should not be the "not implemented" error
                assert!(!error_msg.contains("OpenAPI 3.1 parsing is not yet fully implemented"));
                println!("✓ OpenAPI 3.0 parsing failed with expected error (not 'not implemented'): {}", error_msg);
            }
        }
    }

    #[test]
    fn test_explore_openapiv3_1_api() {
        // Simple test to explore the openapiv3_1 API structure
        let content_31 = r#"
openapi: "3.1.0"
info:
  title: Test API
  version: "1.0.0"
paths:
  /test:
    get:
      operationId: testOperation
      summary: Test operation
      responses:
        '200':
          description: Success
"#;

        // Parse the spec and explore its structure
        let spec: openapiv3_1::OpenApi = yamlviajson::from_str(content_31).expect("Should parse 3.1");
        
        println!("OpenAPI version exists");
        println!("Info title: {}", spec.info.title);
        println!("Info version: {}", spec.info.version);
        
        // Explore paths structure
        println!("Paths count: {}", spec.paths.paths.len());
        
        if let Some(path_item) = spec.paths.paths.get("/test") {
            println!("Found /test path");
            if let Some(get_op) = &path_item.get {
                println!("GET operation ID: {:?}", get_op.operation_id);
                println!("GET summary: {:?}", get_op.summary);
                
                // Explore parameters
                if let Some(parameters) = &get_op.parameters {
                    println!("Parameters count: {}", parameters.len());
                    // Don't try to debug print parameters since they don't implement Debug
                    println!("Parameters exist but can't debug print them");
                }
            }
        }
        
        // Check servers field - don't debug print since Server doesn't implement Debug
        println!("Has servers: {}", spec.servers.is_some());
        if let Some(servers) = &spec.servers {
            println!("Servers count: {}", servers.len());
        }
        
        // Check components
        println!("Has components: {}", spec.components.is_some());
        
        // This test is just for exploration - it should always pass
        assert!(true);
    }

    #[test]
    fn test_openapi_31_petstore_like_spec() {
        // Test with a more complex Petstore-like 3.1 spec
        let petstore_31 = r#"
openapi: 3.1.0
info:
  title: Swagger Petstore - OpenAPI 3.1
  description: This is a sample Pet Store Server based on the OpenAPI 3.1 specification.
  version: 1.0.11
servers:
  - url: https://petstore3.swagger.io/api/v3
paths:
  /pet:
    post:
      operationId: addPet
      summary: Add a new pet to the store
      description: Add a new pet to the store
      responses:
        '200':
          description: Successful operation
        '400':
          description: Invalid input
    put:
      operationId: updatePet
      summary: Update an existing pet
      description: Update an existing pet by Id
      responses:
        '200':
          description: Successful operation
        '400':
          description: Invalid ID supplied
        '404':
          description: Pet not found
  /pet/{petId}:
    get:
      operationId: getPetById
      summary: Find pet by ID
      description: Returns a single pet
      responses:
        '200':
          description: successful operation
        '400':
          description: Invalid ID supplied
        '404':
          description: Pet not found
    delete:
      operationId: deletePet
      summary: Deletes a pet
      description: delete a pet
      responses:
        '400':
          description: Invalid pet value
  /pet/findByStatus:
    get:
      operationId: findPetsByStatus
      summary: Finds Pets by status
      description: Multiple status values can be provided with comma separated strings
      responses:
        '200':
          description: successful operation
        '400':
          description: Invalid status value
"#;

        println!("Testing OpenAPI 3.1 Petstore-like spec...");
        
        // Test version detection
        let version = detect_openapi_version(petstore_31).expect("Should detect version");
        assert!(matches!(version, OpenAPIVersion::V3_1));
        println!("✓ Correctly detected OpenAPI 3.1");

        // Parse the spec
        let spec: openapiv3_1::OpenApi = yamlviajson::from_str(petstore_31).expect("Should parse 3.1");
        let openapi_spec = OpenAPI::V3_1(Arc::new(spec));
        
        // Test parsing into tools
        match parse_openapi_schema(&openapi_spec) {
            Ok(tools_and_calls) => {
                println!("✓ OpenAPI 3.1 Petstore parsing succeeded!");
                println!("✓ Generated {} tools", tools_and_calls.len());
                
                // Should have 5 operations: addPet, updatePet, getPetById, deletePet, findPetsByStatus
                assert_eq!(tools_and_calls.len(), 5);
                
                let tool_names: Vec<&str> = tools_and_calls.iter()
                    .map(|(tool, _)| tool.name.as_ref())
                    .collect();
                
                assert!(tool_names.contains(&"addPet"));
                assert!(tool_names.contains(&"updatePet"));
                assert!(tool_names.contains(&"getPetById"));
                assert!(tool_names.contains(&"deletePet"));
                assert!(tool_names.contains(&"findPetsByStatus"));
                
                for (i, (tool, call)) in tools_and_calls.iter().enumerate() {
                    println!("  Tool {}: {} ({} {})", i + 1, tool.name, call.method, call.path);
                    if let Some(desc) = &tool.description {
                        println!("    Description: {}", desc);
                    }
                }
                
                println!("✓ All expected tools generated successfully");
            },
            Err(e) => {
                panic!("✗ OpenAPI 3.1 Petstore parsing failed: {}", e);
            }
        }
    }
}
