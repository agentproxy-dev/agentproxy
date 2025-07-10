//! Compatibility layer for OpenAPI 3.0 and 3.1 support
//! 
//! This module provides a unified interface for handling both OpenAPI 3.0 and 3.1 specifications
//! by normalizing their differences into common internal representations.

use std::collections::HashMap;
use serde_json::Value;
use super::ParseError;

/// Normalized schema representation that works for both OpenAPI versions
#[derive(Debug, Clone, PartialEq)]
pub struct CompatibleSchema {
    /// Schema type (string, number, object, array, etc.)
    pub schema_type: Option<String>,
    /// Whether the schema allows null values (normalized from 3.1's type arrays)
    pub nullable: bool,
    /// Object properties
    pub properties: HashMap<String, Box<CompatibleSchema>>,
    /// Array items schema
    pub items: Option<Box<CompatibleSchema>>,
    /// Required properties for objects
    pub required: Vec<String>,
    /// Schema description
    pub description: Option<String>,
    /// Additional properties handling
    pub additional_properties: Option<Box<CompatibleSchema>>,
    /// Enum values
    pub enum_values: Option<Vec<Value>>,
    /// Format specification
    pub format: Option<String>,
    /// Minimum value for numbers
    pub minimum: Option<f64>,
    /// Maximum value for numbers
    pub maximum: Option<f64>,
    /// Exclusive minimum flag
    pub exclusive_minimum: Option<bool>,
    /// Exclusive maximum flag
    pub exclusive_maximum: Option<bool>,
    /// Minimum length for strings
    pub min_length: Option<usize>,
    /// Maximum length for strings
    pub max_length: Option<usize>,
    /// Pattern for string validation
    pub pattern: Option<String>,
    /// Minimum items for arrays
    pub min_items: Option<usize>,
    /// Maximum items for arrays
    pub max_items: Option<usize>,
    /// Unique items flag for arrays
    pub unique_items: Option<bool>,
    /// Default value
    pub default: Option<Value>,
    /// Example value
    pub example: Option<Value>,
}

impl Default for CompatibleSchema {
    fn default() -> Self {
        Self {
            schema_type: None,
            nullable: false,
            properties: HashMap::new(),
            items: None,
            required: Vec::new(),
            description: None,
            additional_properties: None,
            enum_values: None,
            format: None,
            minimum: None,
            maximum: None,
            exclusive_minimum: None,
            exclusive_maximum: None,
            min_length: None,
            max_length: None,
            pattern: None,
            min_items: None,
            max_items: None,
            unique_items: None,
            default: None,
            example: None,
        }
    }
}

/// Normalized parameter representation
#[derive(Debug, Clone, PartialEq)]
pub struct CompatibleParameter {
    /// Parameter name
    pub name: String,
    /// Whether the parameter is required
    pub required: bool,
    /// Parameter schema
    pub schema: CompatibleSchema,
    /// Parameter location (query, path, header, cookie)
    pub location: ParameterLocation,
    /// Parameter description
    pub description: Option<String>,
    /// Deprecated flag
    pub deprecated: Option<bool>,
    /// Allow empty value flag
    pub allow_empty_value: Option<bool>,
    /// Style for parameter serialization
    pub style: Option<String>,
    /// Explode flag for parameter serialization
    pub explode: Option<bool>,
}

/// Parameter location enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParameterLocation {
    Query,
    Path,
    Header,
    Cookie,
}

impl std::fmt::Display for ParameterLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParameterLocation::Query => write!(f, "query"),
            ParameterLocation::Path => write!(f, "path"),
            ParameterLocation::Header => write!(f, "header"),
            ParameterLocation::Cookie => write!(f, "cookie"),
        }
    }
}

/// Normalized request body representation
#[derive(Debug, Clone, PartialEq)]
pub struct CompatibleRequestBody {
    /// Request body description
    pub description: Option<String>,
    /// Whether the request body is required
    pub required: bool,
    /// Content types and their schemas
    pub content: HashMap<String, CompatibleMediaType>,
}

/// Normalized media type representation
#[derive(Debug, Clone, PartialEq)]
pub struct CompatibleMediaType {
    /// Media type schema
    pub schema: Option<CompatibleSchema>,
    /// Example value
    pub example: Option<Value>,
    /// Multiple examples
    pub examples: HashMap<String, Value>,
}

/// Trait for converting version-specific types to compatible representations
pub trait ToCompatible<T> {
    fn to_compatible(&self) -> Result<T, ParseError>;
}

/// Trait for converting from compatible types back to JSON schema
pub trait FromCompatible<T> {
    fn from_compatible(compatible: &T) -> Result<Self, ParseError>
    where
        Self: Sized;
}

/// Helper function to normalize type arrays from OpenAPI 3.1 to 3.0 format
/// 
/// OpenAPI 3.1 allows type to be an array like ["string", "null"]
/// We normalize this to type: "string", nullable: true
pub fn normalize_type_array(types: &[String]) -> (Option<String>, bool) {
    if types.is_empty() {
        return (None, false);
    }
    
    let mut non_null_types: Vec<&String> = types.iter().filter(|t| *t != "null").collect();
    let has_null = types.iter().any(|t| t == "null");
    
    match non_null_types.len() {
        0 => (None, true), // Only null type
        1 => (Some(non_null_types[0].clone()), has_null),
        _ => {
            // Multiple non-null types - this is more complex than 3.0 supports
            // For compatibility, we'll take the first type and mark as nullable if null is present
            (Some(non_null_types[0].clone()), has_null)
        }
    }
}

/// Helper function to convert a single type string to normalized format
pub fn normalize_single_type(type_str: &str, nullable: bool) -> (Option<String>, bool) {
    if type_str == "null" {
        (None, true)
    } else {
        (Some(type_str.to_string()), nullable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_type_array_single_type() {
        let types = vec!["string".to_string()];
        let (schema_type, nullable) = normalize_type_array(&types);
        assert_eq!(schema_type, Some("string".to_string()));
        assert_eq!(nullable, false);
    }

    #[test]
    fn test_normalize_type_array_nullable() {
        let types = vec!["string".to_string(), "null".to_string()];
        let (schema_type, nullable) = normalize_type_array(&types);
        assert_eq!(schema_type, Some("string".to_string()));
        assert_eq!(nullable, true);
    }

    #[test]
    fn test_normalize_type_array_only_null() {
        let types = vec!["null".to_string()];
        let (schema_type, nullable) = normalize_type_array(&types);
        assert_eq!(schema_type, None);
        assert_eq!(nullable, true);
    }

    #[test]
    fn test_normalize_type_array_multiple_types() {
        let types = vec!["string".to_string(), "number".to_string(), "null".to_string()];
        let (schema_type, nullable) = normalize_type_array(&types);
        assert_eq!(schema_type, Some("string".to_string())); // Takes first non-null type
        assert_eq!(nullable, true);
    }

    #[test]
    fn test_normalize_type_array_empty() {
        let types = vec![];
        let (schema_type, nullable) = normalize_type_array(&types);
        assert_eq!(schema_type, None);
        assert_eq!(nullable, false);
    }

    #[test]
    fn test_normalize_single_type_regular() {
        let (schema_type, nullable) = normalize_single_type("string", false);
        assert_eq!(schema_type, Some("string".to_string()));
        assert_eq!(nullable, false);
    }

    #[test]
    fn test_normalize_single_type_nullable() {
        let (schema_type, nullable) = normalize_single_type("string", true);
        assert_eq!(schema_type, Some("string".to_string()));
        assert_eq!(nullable, true);
    }

    #[test]
    fn test_normalize_single_type_null() {
        let (schema_type, nullable) = normalize_single_type("null", false);
        assert_eq!(schema_type, None);
        assert_eq!(nullable, true);
    }

    #[test]
    fn test_compatible_schema_default() {
        let schema = CompatibleSchema::default();
        assert_eq!(schema.schema_type, None);
        assert_eq!(schema.nullable, false);
        assert!(schema.properties.is_empty());
        assert_eq!(schema.items, None);
        assert!(schema.required.is_empty());
    }

    #[test]
    fn test_parameter_location_display() {
        assert_eq!(ParameterLocation::Query.to_string(), "query");
        assert_eq!(ParameterLocation::Path.to_string(), "path");
        assert_eq!(ParameterLocation::Header.to_string(), "header");
        assert_eq!(ParameterLocation::Cookie.to_string(), "cookie");
    }
}
