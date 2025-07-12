//! Version-specific adapters for converting OpenAPI types to compatible representations

use std::collections::HashMap;
use openapiv3::{Schema as Schemav3, SchemaKind as SchemaKindv3, Type as Typev3, Parameter as Parameterv3};
use serde_json::Value;

use super::compatibility::{
    CompatibleSchema, CompatibleParameter, CompatibleRequestBody, CompatibleMediaType,
    ParameterLocation, ToCompatible, normalize_single_type
};
use super::ParseError;

// ===== OpenAPI 3.0 Adapters =====

impl ToCompatible<CompatibleSchema> for Schemav3 {
    fn to_compatible(&self) -> Result<CompatibleSchema, ParseError> {
        let mut compatible = CompatibleSchema::default();
        
        // Handle schema data
        compatible.description = self.schema_data.description.clone();
        compatible.default = self.schema_data.default.clone();
        compatible.example = self.schema_data.example.clone();
        compatible.nullable = self.schema_data.nullable;
        
        // Handle external docs, extensions, etc. if needed in the future
        
        // Handle schema kind
        match &self.schema_kind {
            SchemaKindv3::Type(type_def) => {
                match type_def {
                    Typev3::String(string_type) => {
                        compatible.schema_type = Some("string".to_string());
                        compatible.format = match &string_type.format {
                            openapiv3::VariantOrUnknownOrEmpty::Item(f) => Some(format!("{:?}", f)),
                            _ => None,
                        };
                        compatible.min_length = string_type.min_length;
                        compatible.max_length = string_type.max_length;
                        compatible.pattern = string_type.pattern.clone();
                        compatible.enum_values = if string_type.enumeration.is_empty() {
                            None
                        } else {
                            Some(string_type.enumeration.iter().filter_map(|opt| opt.as_ref().map(|s| Value::String(s.clone()))).collect())
                        };
                    },
                    Typev3::Number(number_type) => {
                        compatible.schema_type = Some("number".to_string());
                        compatible.format = match &number_type.format {
                            openapiv3::VariantOrUnknownOrEmpty::Item(f) => Some(format!("{:?}", f)),
                            _ => None,
                        };
                        compatible.minimum = number_type.minimum;
                        compatible.maximum = number_type.maximum;
                        compatible.exclusive_minimum = Some(number_type.exclusive_minimum);
                        compatible.exclusive_maximum = Some(number_type.exclusive_maximum);
                        compatible.enum_values = if number_type.enumeration.is_empty() {
                            None
                        } else {
                            Some(number_type.enumeration.iter().filter_map(|opt| opt.as_ref().map(|n| Value::Number(serde_json::Number::from_f64(*n).unwrap_or_else(|| serde_json::Number::from(0))))).collect())
                        };
                    },
                    Typev3::Integer(integer_type) => {
                        compatible.schema_type = Some("integer".to_string());
                        compatible.format = match &integer_type.format {
                            openapiv3::VariantOrUnknownOrEmpty::Item(f) => Some(format!("{:?}", f)),
                            _ => None,
                        };
                        compatible.minimum = integer_type.minimum.map(|i| i as f64);
                        compatible.maximum = integer_type.maximum.map(|i| i as f64);
                        compatible.exclusive_minimum = Some(integer_type.exclusive_minimum);
                        compatible.exclusive_maximum = Some(integer_type.exclusive_maximum);
                        compatible.enum_values = if integer_type.enumeration.is_empty() {
                            None
                        } else {
                            Some(integer_type.enumeration.iter().filter_map(|opt| opt.as_ref().map(|i| Value::Number(serde_json::Number::from(*i)))).collect())
                        };
                    },
                    Typev3::Object(object_type) => {
                        compatible.schema_type = Some("object".to_string());
                        compatible.required = object_type.required.clone();
                        
                        // Convert properties
                        for (prop_name, prop_schema_ref) in &object_type.properties {
                            // For now, we'll handle direct schemas. Reference resolution will be handled at a higher level
                            if let openapiv3::ReferenceOr::Item(prop_schema) = prop_schema_ref {
                                let prop_compatible = prop_schema.to_compatible()?;
                                compatible.properties.insert(prop_name.clone(), Box::new(prop_compatible));
                            }
                            // References will be resolved by the calling code
                        }
                        
                        // Handle additional properties
                        if let Some(additional_props) = &object_type.additional_properties {
                            match additional_props {
                                openapiv3::AdditionalProperties::Any(true) => {
                                    // Allow any additional properties - we'll represent this as an empty schema
                                    compatible.additional_properties = Some(Box::new(CompatibleSchema::default()));
                                },
                                openapiv3::AdditionalProperties::Any(false) => {
                                    // No additional properties allowed - represented as None
                                    compatible.additional_properties = None;
                                },
                                openapiv3::AdditionalProperties::Schema(schema_ref) => {
                                    if let openapiv3::ReferenceOr::Item(schema) = schema_ref.as_ref() {
                                        let additional_compatible = schema.to_compatible()?;
                                        compatible.additional_properties = Some(Box::new(additional_compatible));
                                    }
                                    // References will be resolved by calling code
                                },
                            }
                        }
                    },
                    Typev3::Array(array_type) => {
                        compatible.schema_type = Some("array".to_string());
                        compatible.min_items = array_type.min_items;
                        compatible.max_items = array_type.max_items;
                        compatible.unique_items = Some(array_type.unique_items);
                        
                        // Handle items schema
                        if let Some(items_ref) = &array_type.items {
                            if let openapiv3::ReferenceOr::Item(items_schema) = items_ref {
                                let items_compatible = items_schema.to_compatible()?;
                                compatible.items = Some(Box::new(items_compatible));
                            }
                            // References will be resolved by calling code
                        }
                    },
                    Typev3::Boolean(_) => {
                        compatible.schema_type = Some("boolean".to_string());
                    },
                }
            },
            SchemaKindv3::OneOf { one_of } => {
                // For compatibility, we'll treat oneOf as the first schema for now
                // This is a simplification but maintains compatibility with 3.0 behavior
                if let Some(first_schema_ref) = one_of.first() {
                    if let openapiv3::ReferenceOr::Item(first_schema) = first_schema_ref {
                        return first_schema.to_compatible();
                    }
                }
                // If no schemas or all references, return a generic object schema
                compatible.schema_type = Some("object".to_string());
            },
            SchemaKindv3::AllOf { all_of } => {
                // For compatibility, we'll merge all schemas into one
                // This is a simplification but maintains basic functionality
                compatible.schema_type = Some("object".to_string());
                
                for schema_ref in all_of {
                    if let openapiv3::ReferenceOr::Item(schema) = schema_ref {
                        let schema_compatible = schema.to_compatible()?;
                        
                        // Merge properties
                        for (prop_name, prop_schema) in schema_compatible.properties {
                            compatible.properties.insert(prop_name, prop_schema);
                        }
                        
                        // Merge required fields
                        for required_field in schema_compatible.required {
                            if !compatible.required.contains(&required_field) {
                                compatible.required.push(required_field);
                            }
                        }
                    }
                }
            },
            SchemaKindv3::AnyOf { any_of } => {
                // For compatibility, we'll treat anyOf as the first schema for now
                if let Some(first_schema_ref) = any_of.first() {
                    if let openapiv3::ReferenceOr::Item(first_schema) = first_schema_ref {
                        return first_schema.to_compatible();
                    }
                }
                // If no schemas or all references, return a generic object schema
                compatible.schema_type = Some("object".to_string());
            },
            SchemaKindv3::Not { not: _ } => {
                // Not schemas are complex to handle in a compatibility layer
                // For now, we'll treat them as generic object schemas
                compatible.schema_type = Some("object".to_string());
            },
            SchemaKindv3::Any(any_schema) => {
                // Handle the "any" schema type which can have various properties
                // For compatibility, we'll treat any schemas as generic object schemas
                // and handle the fields that are available
                
                // If no specific type is set, try to infer from properties
                if compatible.schema_type.is_none() {
                    if !any_schema.properties.is_empty() {
                        compatible.schema_type = Some("object".to_string());
                    } else if any_schema.items.is_some() {
                        compatible.schema_type = Some("array".to_string());
                    } else {
                        // Default to object for any schema
                        compatible.schema_type = Some("object".to_string());
                    }
                }
                
                // Handle properties if this is an object-like schema
                for (prop_name, prop_schema_ref) in &any_schema.properties {
                    if let openapiv3::ReferenceOr::Item(prop_schema) = prop_schema_ref {
                        let prop_compatible = prop_schema.to_compatible()?;
                        compatible.properties.insert(prop_name.clone(), Box::new(prop_compatible));
                    }
                }
                
                // Handle required fields
                compatible.required.extend(any_schema.required.clone());
                
                // Handle array items
                if let Some(items_ref) = &any_schema.items {
                    if let openapiv3::ReferenceOr::Item(items_schema) = items_ref {
                        let items_compatible = items_schema.to_compatible()?;
                        compatible.items = Some(Box::new(items_compatible));
                    }
                }
                
                // Handle other any_schema fields that are available
                compatible.enum_values = compatible.enum_values.or_else(|| Some(any_schema.enumeration.clone()));
                compatible.format = compatible.format.or_else(|| any_schema.format.clone());
                compatible.minimum = compatible.minimum.or(any_schema.minimum);
                compatible.maximum = compatible.maximum.or(any_schema.maximum);
                compatible.exclusive_minimum = compatible.exclusive_minimum.or(any_schema.exclusive_minimum);
                compatible.exclusive_maximum = compatible.exclusive_maximum.or(any_schema.exclusive_maximum);
                compatible.min_length = compatible.min_length.or(any_schema.min_length);
                compatible.max_length = compatible.max_length.or(any_schema.max_length);
                compatible.pattern = compatible.pattern.or_else(|| any_schema.pattern.clone());
                compatible.min_items = compatible.min_items.or(any_schema.min_items);
                compatible.max_items = compatible.max_items.or(any_schema.max_items);
                compatible.unique_items = compatible.unique_items.or(any_schema.unique_items);
            },
        }
        
        Ok(compatible)
    }
}

impl ToCompatible<CompatibleParameter> for Parameterv3 {
    fn to_compatible(&self) -> Result<CompatibleParameter, ParseError> {
        let param_data = self.parameter_data_ref();
        
        // Determine parameter location
        let location = match self {
            Parameterv3::Query { .. } => ParameterLocation::Query,
            Parameterv3::Header { .. } => ParameterLocation::Header,
            Parameterv3::Path { .. } => ParameterLocation::Path,
            Parameterv3::Cookie { .. } => ParameterLocation::Cookie,
        };
        
        // Extract schema from parameter format
        let schema = match &param_data.format {
            openapiv3::ParameterSchemaOrContent::Schema(schema_ref) => {
                match schema_ref {
                    openapiv3::ReferenceOr::Item(schema) => schema.to_compatible()?,
                    openapiv3::ReferenceOr::Reference { .. } => {
                        // References will be resolved by calling code
                        // For now, return a default string schema
                        CompatibleSchema {
                            schema_type: Some("string".to_string()),
                            ..Default::default()
                        }
                    }
                }
            },
            openapiv3::ParameterSchemaOrContent::Content(_content) => {
                // Content-based parameters are more complex
                // For compatibility, we'll treat them as string parameters
                CompatibleSchema {
                    schema_type: Some("string".to_string()),
                    ..Default::default()
                }
            },
        };
        
        Ok(CompatibleParameter {
            name: param_data.name.clone(),
            required: param_data.required,
            schema,
            location,
            description: param_data.description.clone(),
            deprecated: param_data.deprecated,
            allow_empty_value: None, // 3.0 allow_empty_value handling would go here if needed
            style: None, // 3.0 style handling would go here if needed
            explode: None, // 3.0 explode handling would go here if needed
        })
    }
}

// ===== OpenAPI 3.1 Adapters =====
// TODO: Implement OpenAPI 3.1 adapters based on the actual openapiv3_1 crate API
// The openapiv3_1 crate has a different structure than expected, so we need to 
// study the actual API and implement the adapters accordingly.
// For now, we'll focus on getting the specification pattern working correctly.

#[cfg(test)]
mod tests {
    use super::*;

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

    // TODO: Add OpenAPI 3.1 tests when the adapters are implemented
}
