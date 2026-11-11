//! Lightweight JSON Schema validator for MCP Tool input validation.

use crate::error::JsonRpcError;
use serde_json::Value;

pub struct SchemaValidator;

impl SchemaValidator {
    /// Validates `args` against the provided JSON schema.
    pub fn validate(args: &Value, schema: &Value) -> Result<(), JsonRpcError> {
        let empty_obj = Value::Object(serde_json::Map::new());
        let args_obj = match args {
            Value::Object(map) => map,
            Value::Null => {
                if let Value::Object(map) = &empty_obj {
                    map
                } else {
                    unreachable!()
                }
            }
            _ => {
                return Err(JsonRpcError::invalid_params(
                    "Tool arguments must be a JSON object",
                ));
            }
        };

        let schema_obj = match schema {
            Value::Object(map) => map,
            _ => return Ok(()), // Non-object schemas are treated as permissive
        };

        // 1. Validate required fields
        if let Some(Value::Array(required)) = schema_obj.get("required") {
            for req in required {
                if let Some(req_name) = req.as_str() {
                    match args_obj.get(req_name) {
                        Some(val) if !val.is_null() => {}
                        _ => {
                            return Err(JsonRpcError::invalid_params(format!(
                                "Missing required argument '{}'",
                                req_name
                            )));
                        }
                    }
                }
            }
        }

        // 2. Validate properties
        if let Some(Value::Object(properties)) = schema_obj.get("properties") {
            for (prop_name, prop_spec) in properties {
                if let Some(arg_val) = args_obj.get(prop_name) {
                    if arg_val.is_null() {
                        continue;
                    }

                    // Check type constraint
                    if let Some(expected_type) = prop_spec.get("type").and_then(|t| t.as_str()) {
                        let type_valid = match expected_type {
                            "string" => arg_val.is_string(),
                            "integer" => arg_val.is_i64() || arg_val.is_u64(),
                            "number" => arg_val.is_number(),
                            "boolean" => arg_val.is_boolean(),
                            "array" => arg_val.is_array(),
                            "object" => arg_val.is_object(),
                            _ => true,
                        };

                        if !type_valid {
                            return Err(JsonRpcError::invalid_params(format!(
                                "Argument '{}' must be of type '{}', found {}",
                                prop_name,
                                expected_type,
                                Self::value_type_name(arg_val)
                            )));
                        }
                    }

                    // Check enum constraint
                    if let Some(Value::Array(enum_values)) = prop_spec.get("enum") {
                        let is_in_enum = enum_values.iter().any(|ev| ev == arg_val);
                        if !is_in_enum {
                            return Err(JsonRpcError::invalid_params(format!(
                                "Argument '{}' has invalid value {:?}; allowed values are {:?}",
                                prop_name, arg_val, enum_values
                            )));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn value_type_name(val: &Value) -> &'static str {
        match val {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(n) => {
                if n.is_i64() || n.is_u64() {
                    "integer"
                } else {
                    "number"
                }
            }
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_required_and_types() {
        let schema = json!({
            "type": "object",
            "required": ["title", "count"],
            "properties": {
                "title": { "type": "string" },
                "count": { "type": "integer" },
                "priority": { "type": "string", "enum": ["low", "high"] }
            }
        });

        // Valid
        let valid = json!({
            "title": "Fix bug",
            "count": 42,
            "priority": "high"
        });
        assert!(SchemaValidator::validate(&valid, &schema).is_ok());

        // Missing required
        let missing = json!({ "title": "Fix bug" });
        let err = SchemaValidator::validate(&missing, &schema).unwrap_err();
        assert!(err.message.contains("Missing required argument 'count'"));

        // Invalid type
        let bad_type = json!({
            "title": 123,
            "count": 42
        });
        let err = SchemaValidator::validate(&bad_type, &schema).unwrap_err();
        assert!(err.message.contains("must be of type 'string'"));

        // Invalid enum
        let bad_enum = json!({
            "title": "Fix bug",
            "count": 42,
            "priority": "medium"
        });
        let err = SchemaValidator::validate(&bad_enum, &schema).unwrap_err();
        assert!(err.message.contains("allowed values"));
    }
}

