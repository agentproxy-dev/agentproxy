use openapiv3_1::OpenApi;

fn main() {
    let yaml_content = r#"
openapi: 3.1.0
info:
  title: Simple API
  version: 1.0.0
paths:
  /test:
    get:
      operationId: testGet
      summary: Test endpoint
      responses:
        '200':
          description: Success
"#;

    // Parse the OpenAPI 3.1 spec
    match yamlviajson::from_str::<OpenApi>(yaml_content) {
        Ok(spec) => {
            println!("Successfully parsed OpenAPI 3.1 spec!");
            println!("OpenAPI version: {:?}", spec.openapi);
            println!("Info title: {:?}", spec.info.title);
            println!("Info version: {:?}", spec.info.version);
            
            // Explore paths
            println!("Paths: {:?}", spec.paths.paths.keys().collect::<Vec<_>>());
            
            // Try to access a specific path
            if let Some(path_item) = spec.paths.paths.get("/test") {
                println!("Found /test path");
                if let Some(get_op) = &path_item.get {
                    println!("GET operation ID: {:?}", get_op.operation_id);
                    println!("GET summary: {:?}", get_op.summary);
                }
            }
            
            // Check if servers field exists
            println!("Servers: {:?}", spec.servers);
            
            // Check components
            println!("Components: {:?}", spec.components.is_some());
        },
        Err(e) => {
            println!("Failed to parse: {}", e);
        }
    }
}
