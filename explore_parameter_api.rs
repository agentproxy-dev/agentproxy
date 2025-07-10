use openapiv3_1::OpenApi;

fn main() {
    let yaml_content = r#"
openapi: 3.1.0
info:
  title: Test API
  version: 1.0.0
paths:
  /users/{userId}:
    get:
      operationId: getUserById
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
      responses:
        '200':
          description: Success
"#;

    match yamlviajson::from_str::<OpenApi>(yaml_content) {
        Ok(spec) => {
            println!("Successfully parsed OpenAPI 3.1 spec!");
            
            if let Some(path_item) = spec.paths.paths.get("/users/{userId}") {
                if let Some(get_op) = &path_item.get {
                    if let Some(parameters) = &get_op.parameters {
                        println!("Found {} parameters", parameters.len());
                        
                        for (i, param) in parameters.iter().enumerate() {
                            println!("Parameter {}: (can't debug print - no Debug trait)", i + 1);
                            // We can't debug print the parameter directly
                            // Let's try to access fields we know might exist
                        }
                    }
                }
            }
        },
        Err(e) => {
            println!("Failed to parse: {}", e);
        }
    }
}
