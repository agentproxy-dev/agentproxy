use std::sync::Arc;
use agentgateway::types::agent::{OpenAPI, OpenAPIVersion, detect_openapi_version};
use agentgateway::yamlviajson;
use agentgateway::mcp::openapi::parse_openapi_schema;

fn main() {
    // Simple Petstore-like 3.1 spec
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
      parameters:
        - name: petId
          in: path
          description: ID of pet to return
          required: true
          schema:
            type: integer
            format: int64
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
      parameters:
        - name: petId
          in: path
          description: Pet id to delete
          required: true
          schema:
            type: integer
            format: int64
      responses:
        '400':
          description: Invalid pet value
  /pet/findByStatus:
    get:
      operationId: findPetsByStatus
      summary: Finds Pets by status
      description: Multiple status values can be provided with comma separated strings
      parameters:
        - name: status
          in: query
          description: Status values that need to be considered for filter
          required: false
          schema:
            type: string
            default: available
            enum:
              - available
              - pending
              - sold
      responses:
        '200':
          description: successful operation
        '400':
          description: Invalid status value
"#;

    println!("Testing OpenAPI 3.1 Petstore spec...");
    
    // Test version detection
    match detect_openapi_version(petstore_31) {
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
    let spec: openapiv3_1::OpenApi = match yamlviajson::from_str(petstore_31) {
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
            println!("✓ OpenAPI 3.1 Petstore parsing succeeded!");
            println!("✓ Generated {} tools", tools_and_calls.len());
            
            for (i, (tool, call)) in tools_and_calls.iter().enumerate() {
                println!("  Tool {}: {} ({} {})", i + 1, tool.name, call.method, call.path);
                if let Some(desc) = &tool.description {
                    println!("    Description: {}", desc);
                }
            }
        },
        Err(e) => {
            println!("✗ OpenAPI 3.1 Petstore parsing failed: {}", e);
        }
    }
}
