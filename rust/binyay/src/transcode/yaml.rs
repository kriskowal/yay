//! YAML transcoding: convert between YAY values and YAML text.
//!
//! Mapping from YAML to YAY:
//!   - YAML null          -> Value::Null
//!   - YAML bool          -> Value::Bool
//!   - YAML integer       -> Value::Integer (BigInt)
//!   - YAML float         -> Value::Float
//!   - YAML string        -> Value::String
//!   - YAML sequence      -> Value::Array
//!   - YAML mapping       -> Value::Object
//!   - YAML !!binary tag  -> Value::Bytes (base64-decoded)
//!
//! Mapping from YAY to YAML:
//!   - Value::Null         -> YAML null
//!   - Value::Bool         -> YAML bool
//!   - Value::Integer      -> YAML integer (arbitrary precision as string if > i64)
//!   - Value::Float        -> YAML float (including .nan, .inf, -.inf)
//!   - Value::String       -> YAML string
//!   - Value::Array        -> YAML sequence
//!   - Value::Object       -> YAML mapping
//!   - Value::Bytes        -> YAML !!binary (base64-encoded)

use base64::prelude::*;
use libyay::Value;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use std::collections::HashMap;

/// Decode a YAML string into a YAY Value.
pub fn decode(input: &str) -> Result<Value, String> {
    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(input).map_err(|e| format!("YAML parse error: {}", e))?;
    yaml_to_value(&yaml_value)
}

/// Encode a YAY Value as a YAML string.
pub fn encode(value: &Value) -> Result<String, String> {
    let yaml_value = value_to_yaml(value)?;
    serde_yaml::to_string(&yaml_value).map_err(|e| format!("YAML encode error: {}", e))
}

fn yaml_to_value(yaml: &serde_yaml::Value) -> Result<Value, String> {
    match yaml {
        serde_yaml::Value::Null => Ok(Value::Null),
        serde_yaml::Value::Bool(b) => Ok(Value::Bool(*b)),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Integer(BigInt::from(i)))
            } else if let Some(u) = n.as_u64() {
                Ok(Value::Integer(BigInt::from(u)))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::Float(f))
            } else {
                Err(format!("Unsupported YAML number: {:?}", n))
            }
        }
        serde_yaml::Value::String(s) => Ok(Value::String(s.clone())),
        serde_yaml::Value::Sequence(seq) => {
            let items: Result<Vec<Value>, String> = seq.iter().map(yaml_to_value).collect();
            Ok(Value::Array(items?))
        }
        serde_yaml::Value::Mapping(map) => {
            let mut obj = HashMap::new();
            for (k, v) in map {
                let key = match k {
                    serde_yaml::Value::String(s) => s.clone(),
                    serde_yaml::Value::Number(n) => format!("{}", n),
                    serde_yaml::Value::Bool(b) => format!("{}", b),
                    serde_yaml::Value::Null => "null".to_string(),
                    _ => return Err(format!("Unsupported YAML mapping key type: {:?}", k)),
                };
                obj.insert(key, yaml_to_value(v)?);
            }
            Ok(Value::Object(obj))
        }
        serde_yaml::Value::Tagged(tagged) => {
            // Handle !!binary / !binary tag (serde_yaml normalizes the leading !'s)
            let tag_str = tagged.tag.to_string();
            let bare_tag = tag_str.trim_start_matches('!');
            if bare_tag == "binary" {
                if let serde_yaml::Value::String(s) = &tagged.value {
                    let clean: String = s.chars().filter(|c| !c.is_whitespace()).collect();
                    let bytes = BASE64_STANDARD
                        .decode(&clean)
                        .map_err(|e| format!("Invalid base64 in !!binary: {}", e))?;
                    return Ok(Value::Bytes(bytes));
                }
            }
            // For other tags, try to decode the inner value
            yaml_to_value(&tagged.value)
        }
    }
}

fn value_to_yaml(value: &Value) -> Result<serde_yaml::Value, String> {
    match value {
        Value::Null => Ok(serde_yaml::Value::Null),
        Value::Bool(b) => Ok(serde_yaml::Value::Bool(*b)),
        Value::Integer(n) => {
            // Try to fit in i64 first, then u64
            if let Some(i) = n.to_i64() {
                Ok(serde_yaml::Value::Number(serde_yaml::Number::from(i)))
            } else if let Some(u) = n.to_u64() {
                Ok(serde_yaml::Value::Number(serde_yaml::Number::from(u)))
            } else {
                // Big integer beyond i64/u64: emit as string
                // YAML doesn't have native arbitrary-precision integers
                Ok(serde_yaml::Value::String(n.to_string()))
            }
        }
        Value::Float(f) => Ok(serde_yaml::Value::Number(serde_yaml::Number::from(*f))),
        Value::String(s) => Ok(serde_yaml::Value::String(s.clone())),
        Value::Bytes(b) => {
            let b64 = BASE64_STANDARD.encode(b);
            Ok(serde_yaml::Value::Tagged(Box::new(
                serde_yaml::value::TaggedValue {
                    tag: serde_yaml::value::Tag::new("!!binary"),
                    value: serde_yaml::Value::String(b64),
                },
            )))
        }
        Value::Array(arr) => {
            let items: Result<Vec<serde_yaml::Value>, String> =
                arr.iter().map(value_to_yaml).collect();
            Ok(serde_yaml::Value::Sequence(items?))
        }
        Value::Object(obj) => {
            let mut map = serde_yaml::Mapping::new();
            let mut keys: Vec<&String> = obj.keys().collect();
            keys.sort();
            for k in keys {
                map.insert(
                    serde_yaml::Value::String(k.clone()),
                    value_to_yaml(&obj[k])?,
                );
            }
            Ok(serde_yaml::Value::Mapping(map))
        }
    }
}
