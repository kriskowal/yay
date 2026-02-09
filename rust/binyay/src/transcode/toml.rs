//! TOML transcoding: convert between YAY values and TOML text.
//!
//! Mapping from TOML to YAY:
//!   - TOML string         -> Value::String
//!   - TOML integer        -> Value::Integer (BigInt)
//!   - TOML float          -> Value::Float
//!   - TOML boolean        -> Value::Bool
//!   - TOML array          -> Value::Array
//!   - TOML table          -> Value::Object
//!   - TOML datetime       -> Value::String (ISO 8601 representation)
//!
//! Mapping from YAY to TOML:
//!   - Value::Null          -> error (TOML has no null)
//!   - Value::Bool          -> TOML boolean
//!   - Value::Integer       -> TOML integer (if fits in i64, otherwise error)
//!   - Value::Float         -> TOML float
//!   - Value::String        -> TOML string
//!   - Value::Array         -> TOML array
//!   - Value::Object        -> TOML table
//!   - Value::Bytes         -> error (TOML has no binary type)
//!
//! Lossy edges:
//!   - TOML has no null type; YAY null values cause an error.
//!   - TOML integers are i64; YAY big integers that overflow will error.
//!   - TOML has no binary type; YAY bytes cause an error.
//!   - TOML floats don't preserve negative zero distinctly (implementation-dependent).
//!   - TOML datetimes become YAY strings (no dedicated datetime type in YAY).
//!   - TOML requires the top-level value to be a table; non-table YAY values error.

use libyay::Value;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use std::collections::HashMap;
use toml_edit::DocumentMut;

/// Decode a TOML string into a YAY Value.
pub fn decode(input: &str) -> Result<Value, String> {
    let doc: DocumentMut = input
        .parse::<DocumentMut>()
        .map_err(|e| format!("TOML parse error: {}", e))?;
    toml_table_to_value(doc.as_table())
}

/// Encode a YAY Value as a TOML string.
pub fn encode(value: &Value) -> Result<String, String> {
    check_toml_compatibility(value)?;
    match value {
        Value::Object(_) => {
            let toml_item = value_to_toml(value)?;
            match toml_item {
                toml_edit::Item::Table(table) => {
                    let mut doc = DocumentMut::new();
                    for (key, value) in table.iter() {
                        doc[key] = value.clone();
                    }
                    Ok(doc.to_string())
                }
                _ => Err("Internal error: expected table".to_string()),
            }
        }
        _ => Err("TOML requires the top-level value to be a table/object".to_string()),
    }
}

fn check_toml_compatibility(value: &Value) -> Result<(), String> {
    match value {
        Value::Null => Err("TOML has no null type".to_string()),
        Value::Bytes(_) => Err("TOML has no binary data type".to_string()),
        Value::Integer(n) => {
            if n.to_i64().is_none() {
                Err(format!("TOML integers must fit in i64; {} is too large", n))
            } else {
                Ok(())
            }
        }
        Value::Array(arr) => {
            for v in arr {
                check_toml_compatibility(v)?;
            }
            Ok(())
        }
        Value::Object(obj) => {
            for v in obj.values() {
                check_toml_compatibility(v)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn toml_table_to_value(table: &toml_edit::Table) -> Result<Value, String> {
    let mut obj = HashMap::new();
    for (key, item) in table.iter() {
        obj.insert(key.to_string(), toml_item_to_value(item)?);
    }
    Ok(Value::Object(obj))
}

fn toml_item_to_value(item: &toml_edit::Item) -> Result<Value, String> {
    match item {
        toml_edit::Item::Value(v) => toml_value_to_yay(v),
        toml_edit::Item::Table(t) => toml_table_to_value(t),
        toml_edit::Item::ArrayOfTables(arr) => {
            let items: Result<Vec<Value>, String> =
                arr.iter().map(|t| toml_table_to_value(t)).collect();
            Ok(Value::Array(items?))
        }
        toml_edit::Item::None => Ok(Value::Null),
    }
}

fn toml_value_to_yay(v: &toml_edit::Value) -> Result<Value, String> {
    match v {
        toml_edit::Value::String(s) => Ok(Value::String(s.value().clone())),
        toml_edit::Value::Integer(i) => Ok(Value::Integer(BigInt::from(*i.value()))),
        toml_edit::Value::Float(f) => Ok(Value::Float(*f.value())),
        toml_edit::Value::Boolean(b) => Ok(Value::Bool(*b.value())),
        toml_edit::Value::Datetime(dt) => {
            // Convert datetime to string representation
            Ok(Value::String(dt.value().to_string()))
        }
        toml_edit::Value::Array(arr) => {
            let items: Result<Vec<Value>, String> = arr.iter().map(toml_value_to_yay).collect();
            Ok(Value::Array(items?))
        }
        toml_edit::Value::InlineTable(table) => {
            let mut obj = HashMap::new();
            for (key, val) in table.iter() {
                obj.insert(key.to_string(), toml_value_to_yay(val)?);
            }
            Ok(Value::Object(obj))
        }
    }
}

fn value_to_toml(value: &Value) -> Result<toml_edit::Item, String> {
    match value {
        Value::Null => Err("TOML has no null type".to_string()),
        Value::Bool(b) => Ok(toml_edit::Item::Value(toml_edit::Value::Boolean(
            toml_edit::Formatted::new(*b),
        ))),
        Value::Integer(n) => {
            let i = n
                .to_i64()
                .ok_or_else(|| format!("Integer {} too large for TOML (i64)", n))?;
            Ok(toml_edit::Item::Value(toml_edit::Value::Integer(
                toml_edit::Formatted::new(i),
            )))
        }
        Value::Float(f) => Ok(toml_edit::Item::Value(toml_edit::Value::Float(
            toml_edit::Formatted::new(*f),
        ))),
        Value::String(s) => Ok(toml_edit::Item::Value(toml_edit::Value::String(
            toml_edit::Formatted::new(s.clone()),
        ))),
        Value::Bytes(_) => Err("TOML has no binary data type".to_string()),
        Value::Array(arr) => {
            let mut toml_arr = toml_edit::Array::new();
            for v in arr {
                match value_to_toml(v)? {
                    toml_edit::Item::Value(val) => toml_arr.push(val),
                    toml_edit::Item::Table(t) => {
                        // Convert table to inline table for array elements
                        let mut inline = toml_edit::InlineTable::new();
                        for (k, item) in t.iter() {
                            if let toml_edit::Item::Value(val) = item {
                                inline.insert(k, val.clone());
                            }
                        }
                        toml_arr.push(toml_edit::Value::InlineTable(inline));
                    }
                    _ => return Err("Unexpected TOML item type in array".to_string()),
                }
            }
            Ok(toml_edit::Item::Value(toml_edit::Value::Array(toml_arr)))
        }
        Value::Object(obj) => {
            let mut table = toml_edit::Table::new();
            let mut keys: Vec<&String> = obj.keys().collect();
            keys.sort();
            for k in keys {
                table.insert(k, value_to_toml(&obj[k])?);
            }
            Ok(toml_edit::Item::Table(table))
        }
    }
}
