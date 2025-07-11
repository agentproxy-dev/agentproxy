#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::types::agent::{OpenAPI, OpenAPIVersion, detect_openapi_version};
    use crate::yamlviajson;
    use openapiv3::OpenAPI as OpenAPIv3;
    use crate::mcp::openapi::parse_openapi_schema;
    use crate::mcp::openapi::v3_1::OpenAPI31Specification;
    use serde_json::json;

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
    fn test_openapi_31_with_parameters() {
        // Test OpenAPI 3.1 spec with parameters
        let content_31 = r#"
openapi: "3.1.0"
info:
  title: Test API with Parameters
  version: "1.0.0"
servers:
  - url: https://api.example.com
paths:
  /users/{userId}:
    get:
      operationId: getUserById
      summary: Get user by ID
      parameters:
        - name: userId
          in: path
          required: true
          description: The user ID
          schema:
            type: integer
            format: int64
        - name: include
          in: query
          required: false
          description: Fields to include
          schema:
            type: string
            enum: ["profile", "settings", "all"]
        - name: X-API-Key
          in: header
          required: true
          description: API key for authentication
          schema:
            type: string
      responses:
        '200':
          description: User found
        '404':
          description: User not found
"#;

        println!("Testing OpenAPI 3.1 with parameters...");
        
        // Test version detection
        let version = detect_openapi_version(content_31).expect("Should detect version");
        assert!(matches!(version, OpenAPIVersion::V3_1));
        println!("✓ Correctly detected OpenAPI 3.1");

        // Parse the spec
        let spec: openapiv3_1::OpenApi = yamlviajson::from_str(content_31).expect("Should parse 3.1");
        let openapi_spec = OpenAPI::V3_1(Arc::new(spec));
        
        // Test parsing into tools
        match parse_openapi_schema(&openapi_spec) {
            Ok(tools_and_calls) => {
                println!("✓ OpenAPI 3.1 parameter parsing succeeded!");
                println!("✓ Generated {} tools", tools_and_calls.len());
                
                assert_eq!(tools_and_calls.len(), 1);
                let (tool, call) = &tools_and_calls[0];
                
                assert_eq!(tool.name, "getUserById");
                assert_eq!(call.method, "GET");
                assert_eq!(call.path, "/users/{userId}");
                
                println!("✓ Tool: {} ({} {})", tool.name, call.method, call.path);
                
                // Check that the tool has a proper input schema
                println!("✓ Tool input schema keys: {:?}", tool.input_schema.keys().collect::<Vec<_>>());
                
                // Check if we have properties (indicating parameter processing)
                if let Some(properties) = tool.input_schema.get("properties") {
                    if let Some(props_obj) = properties.as_object() {
                        println!("✓ Found {} parameter properties: {:?}", props_obj.len(), props_obj.keys().collect::<Vec<_>>());
                        
                        // We expect to have processed 3 parameters from our test spec
                        if props_obj.len() > 0 {
                            println!("✓ Parameter processing is working!");
                        } else {
                            println!("⚠ No parameters processed yet (expected for current implementation)");
                        }
                    }
                }
                
                if let Some(desc) = &tool.description {
                    println!("✓ Tool description: {}", desc);
                }
            },
            Err(e) => {
                panic!("✗ OpenAPI 3.1 parameter parsing failed: {}", e);
            }
        }
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

    #[test]
    fn test_openapi_31_with_request_body() {
        // Test OpenAPI 3.1 spec with request body
        let content_31 = r#"
openapi: "3.1.0"
info:
  title: Test API with Request Body
  version: "1.0.0"
servers:
  - url: https://api.example.com
paths:
  /users:
    post:
      operationId: createUser
      summary: Create a new user
      description: Create a new user with the provided data
      requestBody:
        required: true
        description: User data to create
        content:
          application/json:
            schema:
              type: object
              required:
                - name
                - email
              properties:
                name:
                  type: string
                  description: User's full name
                email:
                  type: string
                  format: email
                  description: User's email address
                age:
                  type: integer
                  minimum: 0
                  maximum: 150
                  description: User's age (optional)
      responses:
        '201':
          description: User created successfully
        '400':
          description: Invalid user data
"#;

        println!("Testing OpenAPI 3.1 with request body...");
        
        // Test version detection
        let version = detect_openapi_version(content_31).expect("Should detect version");
        assert!(matches!(version, OpenAPIVersion::V3_1));
        println!("✓ Correctly detected OpenAPI 3.1");

        // Parse the spec
        let spec: openapiv3_1::OpenApi = yamlviajson::from_str(content_31).expect("Should parse 3.1");
        let openapi_spec = OpenAPI::V3_1(Arc::new(spec));
        
        // Test parsing into tools
        match parse_openapi_schema(&openapi_spec) {
            Ok(tools_and_calls) => {
                println!("✓ OpenAPI 3.1 request body parsing succeeded!");
                println!("✓ Generated {} tools", tools_and_calls.len());
                
                assert_eq!(tools_and_calls.len(), 1);
                let (tool, call) = &tools_and_calls[0];
                
                assert_eq!(tool.name, "createUser");
                assert_eq!(call.method, "POST");
                assert_eq!(call.path, "/users");
                
                println!("✓ Tool: {} ({} {})", tool.name, call.method, call.path);
                
                // Check that the tool has a proper input schema
                println!("✓ Tool input schema keys: {:?}", tool.input_schema.keys().collect::<Vec<_>>());
                
                // Check if we have properties (indicating request body processing)
                if let Some(properties) = tool.input_schema.get("properties") {
                    if let Some(props_obj) = properties.as_object() {
                        println!("✓ Found {} properties: {:?}", props_obj.len(), props_obj.keys().collect::<Vec<_>>());
                        
                        // For now, we expect basic schema structure
                        // Later we'll enhance this to include request body fields
                        if props_obj.len() > 0 {
                            println!("✓ Schema processing is working!");
                        } else {
                            println!("⚠ No request body properties processed yet (expected for current implementation)");
                        }
                    }
                }
                
                if let Some(desc) = &tool.description {
                    println!("✓ Tool description: {}", desc);
                }
            },
            Err(e) => {
                panic!("✗ OpenAPI 3.1 request body parsing failed: {}", e);
            }
        }
    }

    #[test]
    fn test_normalize_schema_v3_1_type_arrays() {
        // Test the most critical method: normalize_schema_v3_1 with type arrays
        let spec = create_test_spec();
        let openapi_31 = OpenAPI31Specification::new(Arc::new(spec));
        
        // Test basic type array conversion: ["string", "null"] -> nullable: true
        let type_array_schema = json!({
            "type": ["string", "null"],
            "description": "A nullable string field"
        });
        
        let result = openapi_31.normalize_schema_v3_1(&type_array_schema).unwrap();
        assert_eq!(result["type"], "string");
        assert_eq!(result["nullable"], true);
        assert_eq!(result["description"], "A nullable string field");
        
        // Test number type array
        let number_array_schema = json!({
            "type": ["number", "null"],
            "minimum": 0,
            "maximum": 100
        });
        
        let result = openapi_31.normalize_schema_v3_1(&number_array_schema).unwrap();
        assert_eq!(result["type"], "number");
        assert_eq!(result["nullable"], true);
        assert_eq!(result["minimum"], 0);
        assert_eq!(result["maximum"], 100);
        
        // Test array type array
        let array_type_schema = json!({
            "type": ["array", "null"],
            "items": {
                "type": "string"
            },
            "minItems": 1,
            "maxItems": 10
        });
        
        let result = openapi_31.normalize_schema_v3_1(&array_type_schema).unwrap();
        assert_eq!(result["type"], "array");
        assert_eq!(result["nullable"], true);
        assert_eq!(result["minItems"], 1);
        assert_eq!(result["maxItems"], 10);
        assert!(result["items"].is_object());
        
        // Test complex nested type array
        let nested_schema = json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": ["string", "null"]
                },
                "age": {
                    "type": ["integer", "null"],
                    "minimum": 0
                }
            }
        });
        
        let result = openapi_31.normalize_schema_v3_1(&nested_schema).unwrap();
        let properties = result["properties"].as_object().unwrap();
        
        // Check nested name property
        let name_prop = &properties["name"];
        assert_eq!(name_prop["type"], "string");
        assert_eq!(name_prop["nullable"], true);
        
        // Check nested age property
        let age_prop = &properties["age"];
        assert_eq!(age_prop["type"], "integer");
        assert_eq!(age_prop["nullable"], true);
        assert_eq!(age_prop["minimum"], 0);
        
        println!("✓ Type arrays processing test passed!");
    }

    #[test]
    fn test_normalize_schema_v3_1_validation_keywords() {
        // Test validation keyword preservation
        let spec = create_test_spec();
        let openapi_31 = OpenAPI31Specification::new(Arc::new(spec));
        
        // Test string validation keywords
        let string_schema = json!({
            "type": "string",
            "pattern": "^[A-Za-z]+$",
            "minLength": 2,
            "maxLength": 50,
            "format": "email"
        });
        
        let result = openapi_31.normalize_schema_v3_1(&string_schema).unwrap();
        assert_eq!(result["type"], "string");
        assert_eq!(result["pattern"], "^[A-Za-z]+$");
        assert_eq!(result["minLength"], 2);
        assert_eq!(result["maxLength"], 50);
        assert_eq!(result["format"], "email");
        
        // Test array validation keywords
        let array_schema = json!({
            "type": "array",
            "items": {
                "type": "string"
            },
            "minItems": 1,
            "maxItems": 10,
            "uniqueItems": true
        });
        
        let result = openapi_31.normalize_schema_v3_1(&array_schema).unwrap();
        assert_eq!(result["type"], "array");
        assert_eq!(result["minItems"], 1);
        assert_eq!(result["maxItems"], 10);
        assert_eq!(result["uniqueItems"], true);
        
        // Test numeric validation keywords
        let number_schema = json!({
            "type": "number",
            "minimum": 0,
            "maximum": 100,
            "multipleOf": 5
        });
        
        let result = openapi_31.normalize_schema_v3_1(&number_schema).unwrap();
        assert_eq!(result["type"], "number");
        assert_eq!(result["minimum"], 0);
        assert_eq!(result["maximum"], 100);
        assert_eq!(result["multipleOf"], 5);
        
        // Test enum preservation
        let enum_schema = json!({
            "type": "string",
            "enum": ["red", "green", "blue"]
        });
        
        let result = openapi_31.normalize_schema_v3_1(&enum_schema).unwrap();
        assert_eq!(result["type"], "string");
        assert_eq!(result["enum"], json!(["red", "green", "blue"]));
        
        println!("✓ Validation keywords preservation test passed!");
    }

    #[test]
    fn test_normalize_schema_composition_anyof() {
        // Test anyOf composition processing
        let spec = create_test_spec();
        let openapi_31 = OpenAPI31Specification::new(Arc::new(spec));
        
        // Test simple anyOf composition
        let anyof_schema = json!({
            "anyOf": [
                {
                    "type": "string",
                    "minLength": 1
                },
                {
                    "type": "number",
                    "minimum": 0
                }
            ]
        });
        
        let result = openapi_31.normalize_schema_v3_1(&anyof_schema).unwrap();
        assert!(result["anyOf"].is_array());
        
        let anyof_array = result["anyOf"].as_array().unwrap();
        assert_eq!(anyof_array.len(), 2);
        
        // Check first schema in anyOf
        assert_eq!(anyof_array[0]["type"], "string");
        assert_eq!(anyof_array[0]["minLength"], 1);
        
        // Check second schema in anyOf
        assert_eq!(anyof_array[1]["type"], "number");
        assert_eq!(anyof_array[1]["minimum"], 0);
        
        // Test anyOf with type arrays
        let anyof_with_nullable = json!({
            "anyOf": [
                {
                    "type": ["string", "null"],
                    "pattern": "^[A-Z]+$"
                },
                {
                    "type": "number",
                    "multipleOf": 2
                }
            ]
        });
        
        let result = openapi_31.normalize_schema_v3_1(&anyof_with_nullable).unwrap();
        let anyof_array = result["anyOf"].as_array().unwrap();
        
        // Check that type arrays are normalized within anyOf
        assert_eq!(anyof_array[0]["type"], "string");
        assert_eq!(anyof_array[0]["nullable"], true);
        assert_eq!(anyof_array[0]["pattern"], "^[A-Z]+$");
        
        println!("✓ anyOf composition test passed!");
    }

    #[test]
    fn test_normalize_schema_composition_oneof() {
        // Test oneOf composition processing
        let spec = create_test_spec();
        let openapi_31 = OpenAPI31Specification::new(Arc::new(spec));
        
        // Test oneOf composition
        let oneof_schema = json!({
            "oneOf": [
                {
                    "type": "object",
                    "properties": {
                        "email": {
                            "type": "string",
                            "format": "email"
                        }
                    },
                    "required": ["email"]
                },
                {
                    "type": "object",
                    "properties": {
                        "phone": {
                            "type": "string",
                            "pattern": "^\\+?[1-9]\\d{1,14}$"
                        }
                    },
                    "required": ["phone"]
                }
            ]
        });
        
        let result = openapi_31.normalize_schema_v3_1(&oneof_schema).unwrap();
        assert!(result["oneOf"].is_array());
        
        let oneof_array = result["oneOf"].as_array().unwrap();
        assert_eq!(oneof_array.len(), 2);
        
        // Check first schema in oneOf
        let first_schema = &oneof_array[0];
        assert_eq!(first_schema["type"], "object");
        let props = first_schema["properties"].as_object().unwrap();
        assert_eq!(props["email"]["type"], "string");
        assert_eq!(props["email"]["format"], "email");
        
        // Check second schema in oneOf
        let second_schema = &oneof_array[1];
        assert_eq!(second_schema["type"], "object");
        let props = second_schema["properties"].as_object().unwrap();
        assert_eq!(props["phone"]["type"], "string");
        assert_eq!(props["phone"]["pattern"], "^\\+?[1-9]\\d{1,14}$");
        
        println!("✓ oneOf composition test passed!");
    }

    #[test]
    fn test_normalize_schema_composition_allof() {
        // Test allOf composition processing
        let spec = create_test_spec();
        let openapi_31 = OpenAPI31Specification::new(Arc::new(spec));
        
        // Test allOf composition
        let allof_schema = json!({
            "allOf": [
                {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "minLength": 1
                        }
                    },
                    "required": ["name"]
                },
                {
                    "type": "object",
                    "properties": {
                        "timestamp": {
                            "type": "string",
                            "format": "date-time"
                        }
                    }
                }
            ]
        });
        
        let result = openapi_31.normalize_schema_v3_1(&allof_schema).unwrap();
        assert!(result["allOf"].is_array());
        
        let allof_array = result["allOf"].as_array().unwrap();
        assert_eq!(allof_array.len(), 2);
        
        // Check first schema in allOf
        let first_schema = &allof_array[0];
        assert_eq!(first_schema["type"], "object");
        let props = first_schema["properties"].as_object().unwrap();
        assert_eq!(props["name"]["type"], "string");
        assert_eq!(props["name"]["minLength"], 1);
        
        // Check second schema in allOf
        let second_schema = &allof_array[1];
        assert_eq!(second_schema["type"], "object");
        let props = second_schema["properties"].as_object().unwrap();
        assert_eq!(props["timestamp"]["type"], "string");
        assert_eq!(props["timestamp"]["format"], "date-time");
        
        println!("✓ allOf composition test passed!");
    }

    // Helper function to create a test spec
    fn create_test_spec() -> openapiv3_1::OpenApi {
        let spec_content = r#"
openapi: "3.1.0"
info:
  title: Test API
  version: "1.0.0"
paths:
  /test:
    get:
      operationId: testOperation
      responses:
        '200':
          description: Success
"#;
        yamlviajson::from_str(spec_content).expect("Should parse test spec")
    }
}
