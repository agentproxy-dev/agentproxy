# Development Patterns

This document describes key architectural patterns used in the AgentGateway codebase that enable extensibility and maintainability.

## Specification Pattern with Behaviors

The **Specification Pattern with Behaviors** is a design pattern that allows for version-specific implementations while maintaining a common interface. This pattern is particularly useful when dealing with evolving specifications that have different versions with varying capabilities.

### Overview

The pattern separates the specification interface from its implementation, allowing different versions to provide their own behaviors while maintaining compatibility through a common abstraction layer.

### Key Components

1. **Specification Trait** - Defines the common interface
2. **Version-Specific Implementations** - Concrete behaviors for each version
3. **Compatibility Layer** - Normalizes differences between versions
4. **Factory Pattern** - Creates appropriate implementations based on version
5. **Common Behaviors** - Shared functionality across versions

### Implementation in OpenAPI Support

The AgentGateway uses this pattern to support both OpenAPI 3.0 and 3.1 specifications, with the ability to easily add future versions.

#### Architecture Diagram

```mermaid
graph TB
    Client[Client Code] --> Factory[OpenAPISpecificationFactory]
    Factory --> |creates| Spec30[OpenAPI30Specification]
    Factory --> |creates| Spec31[OpenAPI31Specification]
    Factory --> |future| SpecFuture[OpenAPIXXSpecification]
    
    Spec30 --> |implements| SpecTrait[OpenAPISpecification Trait]
    Spec31 --> |implements| SpecTrait
    SpecFuture --> |implements| SpecTrait
    
    Spec30 --> |uses| Compat[Compatibility Layer]
    Spec31 --> |uses| Compat
    SpecFuture --> |uses| Compat
    
    Compat --> Compatible30[CompatibleSchema<br/>CompatibleParameter<br/>CompatibleRequestBody]
    
    SpecTrait --> |defines| Methods[parse_schema()<br/>get_server_prefix()<br/>version()]
    
    Spec30 --> |implements| Resolver[SchemaResolver<br/>SchemaBuilder]
    Spec31 --> |implements| Resolver
    
    style Factory fill:#e1f5fe
    style SpecTrait fill:#f3e5f5
    style Compat fill:#e8f5e8
    style Compatible30 fill:#fff3e0
```

#### Core Traits

**OpenAPISpecification Trait** - Main interface for all versions:

```rust
pub trait OpenAPISpecification {
    /// Parse the OpenAPI specification into tools and upstream calls
    fn parse_schema(&self) -> Result<Vec<(Tool, UpstreamOpenAPICall)>, ParseError>;
    
    /// Get the server prefix for this specification
    fn get_server_prefix(&self) -> Result<String, ParseError>;
    
    /// Get the OpenAPI version string
    fn version(&self) -> String;
}
```

**Supporting Traits** for additional behaviors:

```rust
pub trait SchemaResolver {
    fn resolve_schema(&self, reference: &str) -> Result<CompatibleSchema, ParseError>;
    fn resolve_parameter(&self, reference: &str) -> Result<CompatibleParameter, ParseError>;
    fn resolve_request_body(&self, reference: &str) -> Result<CompatibleRequestBody, ParseError>;
}

pub trait SchemaBuilder {
    fn build_schema_property(&self, parameter: &CompatibleParameter) -> Result<(String, JsonObject, bool), ParseError>;
    fn build_json_schema(&self, components: &HashMap<String, Value>) -> Result<JsonObject, ParseError>;
}
```

#### Factory Implementation

The factory pattern creates the appropriate specification behavior:

```rust
pub struct OpenAPISpecificationFactory;

impl OpenAPISpecificationFactory {
    pub fn create_specification(
        openapi: &crate::types::agent::OpenAPI,
    ) -> Box<dyn OpenAPISpecification> {
        match openapi {
            crate::types::agent::OpenAPI::V3_0(spec) => {
                Box::new(super::v3_0::OpenAPI30Specification::new(spec.clone()))
            },
            crate::types::agent::OpenAPI::V3_1(spec) => {
                Box::new(super::v3_1::OpenAPI31Specification::new(spec.clone()))
            },
        }
    }
}
```

#### Compatibility Layer

The compatibility layer normalizes differences between versions:

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct CompatibleSchema {
    pub schema_type: Option<String>,
    pub nullable: bool,  // Normalized from 3.1's type arrays
    pub properties: HashMap<String, Box<CompatibleSchema>>,
    pub items: Option<Box<CompatibleSchema>>,
    // ... other common fields
}

pub trait ToCompatible<T> {
    fn to_compatible(&self) -> Result<T, ParseError>;
}
```

#### Version-Specific Implementations

**OpenAPI 3.0 Implementation:**
- File: `crates/agentgateway/src/mcp/openapi/v3_0.rs`
- Handles OpenAPI 3.0 specific parsing logic
- Implements reference resolution for 3.0 format
- Converts to compatibility layer

**OpenAPI 3.1 Implementation:**
- File: `crates/agentgateway/src/mcp/openapi/v3_1.rs`
- Handles OpenAPI 3.1 specific features like type arrays
- Normalizes JSON Schema Draft 2020-12 features
- Converts `["string", "null"]` to `type: "string", nullable: true`

### Key Benefits

1. **Extensibility** - New versions can be added without changing existing code
2. **Maintainability** - Each version's logic is isolated
3. **Consistency** - Common interface ensures predictable behavior
4. **Compatibility** - Differences are normalized through the compatibility layer
5. **Testability** - Each version can be tested independently

### Usage Example

```rust
// Client code doesn't need to know about versions
pub fn parse_openapi_schema(
    open_api: &OpenAPI,
) -> Result<Vec<(Tool, UpstreamOpenAPICall)>, ParseError> {
    let specification = OpenAPISpecificationFactory::create_specification(open_api);
    specification.parse_schema()
}
```

### Adding New Versions

To add support for a new OpenAPI version (e.g., 4.0):

1. **Create version-specific module**: `v4_0.rs`
2. **Implement the traits**:
   ```rust
   pub struct OpenAPI40Specification {
       spec: Arc<OpenAPIv4_0>,
   }
   
   impl OpenAPISpecification for OpenAPI40Specification {
       fn parse_schema(&self) -> Result<Vec<(Tool, UpstreamOpenAPICall)>, ParseError> {
           // Version 4.0 specific parsing logic
       }
       // ... other methods
   }
   ```
3. **Update the factory**:
   ```rust
   crate::types::agent::OpenAPI::V4_0(spec) => {
       Box::new(super::v4_0::OpenAPI40Specification::new(spec.clone()))
   }
   ```
4. **Extend compatibility layer** if needed for new features
5. **Add tests** for the new version

### Related Files

- **Core Pattern**: `crates/agentgateway/src/mcp/openapi/specification.rs`
- **Factory**: `crates/agentgateway/src/mcp/openapi/specification.rs` (OpenAPISpecificationFactory)
- **Compatibility**: `crates/agentgateway/src/mcp/openapi/compatibility.rs`
- **OpenAPI 3.0**: `crates/agentgateway/src/mcp/openapi/v3_0.rs`
- **OpenAPI 3.1**: `crates/agentgateway/src/mcp/openapi/v3_1.rs`
- **Main Module**: `crates/agentgateway/src/mcp/openapi/mod.rs`

### Testing

Each version implementation includes comprehensive tests:

- **Unit tests** for version-specific parsing logic
- **Integration tests** for end-to-end functionality
- **Compatibility tests** to ensure consistent behavior across versions

Example test files:
- `crates/agentgateway/src/mcp/openapi/test_31.rs` - OpenAPI 3.1 specific tests
- Various test files in the root: `test_openapi_31.yaml`, `test_type_arrays_31.rs`, etc.

### Best Practices

1. **Keep interfaces stable** - Changes to traits affect all implementations
2. **Use compatibility layer** - Don't expose version-specific types to client code
3. **Document version differences** - Clearly explain what each version supports
4. **Test thoroughly** - Each version should have comprehensive test coverage
5. **Handle graceful degradation** - Newer features should degrade gracefully in older versions

This pattern enables the AgentGateway to evolve with OpenAPI specifications while maintaining backward compatibility and providing a consistent developer experience.
