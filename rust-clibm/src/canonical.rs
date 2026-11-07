//! Canonical JSON Serialization (RFC 8785 / JCS style)
//!
//! Ensures that any `serde_json::Value` is serialized with deterministically
//! sorted object keys and zero extraneous whitespace so that identical semantic
//! payloads always produce the exact same BLAKE3 cryptographic hash across
//! Linux and Windows platforms.

use serde::Serialize;
use serde_json::{Map, Value};
use anyhow::{Context, Result};

/// Recursively normalizes a `serde_json::Value` by sorting all object keys
/// lexicographically (UTF-8 code unit order).
pub fn normalize_value(val: &Value) -> Value {
    match val {
        Value::Object(map) => {
            let mut sorted_keys: Vec<&String> = map.keys().collect();
            sorted_keys.sort();
            let mut new_map = Map::new();
            for k in sorted_keys {
                if let Some(v) = map.get(k) {
                    new_map.insert(k.clone(), normalize_value(v));
                }
            }
            Value::Object(new_map)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(normalize_value).collect()),
        other => other.clone(),
    }
}

/// Serializes any `Serialize` type into a canonical, deterministic JSON string.
pub fn to_canonical_json<T: Serialize>(value: &T) -> Result<String> {
    let raw_value = serde_json::to_value(value)
        .context("Failed to convert structure to serde_json::Value")?;
    let normalized = normalize_value(&raw_value);
    serde_json::to_string(&normalized)
        .context("Failed to serialize canonical serde_json::Value")
}

/// Computes the 256-bit BLAKE3 hex digest for a canonical JSON envelope
/// consisting of `(Type, Version, Payload)`.
pub fn compute_object_hash<T: Serialize>(
    object_type: &str,
    version: &str,
    payload: &T,
) -> Result<(String, Value)> {
    let payload_val = normalize_value(
        &serde_json::to_value(payload).context("Failed to serialize object payload")?,
    );

    let mut canonical_input = Map::new();
    canonical_input.insert("Payload".to_string(), payload_val.clone());
    canonical_input.insert("Type".to_string(), Value::String(object_type.to_string()));
    canonical_input.insert("Version".to_string(), Value::String(version.to_string()));

    let canonical_bytes = serde_json::to_vec(&Value::Object(canonical_input))
        .context("Failed to encode canonical envelope bytes")?;

    let digest = blake3::hash(&canonical_bytes);
    Ok((digest.to_hex().to_string(), payload_val))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_json_is_key_order_independent() {
        let a = json!({ "z": 1, "a": { "d": true, "b": "hello" } });
        let b = json!({ "a": { "b": "hello", "d": true }, "z": 1 });

        let (hash_a, _) = compute_object_hash("task", "0.1.0", &a).unwrap();
        let (hash_b, _) = compute_object_hash("task", "0.1.0", &b).unwrap();
        assert_eq!(hash_a, hash_b);
    }
}
