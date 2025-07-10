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

        // Test that parsing into tools gives helpful error
        match parse_openapi_schema(&spec) {
            Ok(_) => panic!("✗ Should not succeed in parsing 3.1 tools yet"),
            Err(e) => {
                let error_msg = e.to_string();
                assert!(error_msg.contains("OpenAPI 3.1 parsing is not yet fully implemented"));
                assert!(error_msg.contains("specification pattern"));
                println!("✓ Got expected helpful error message: {}", error_msg);
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
}
