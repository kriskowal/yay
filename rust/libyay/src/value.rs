//! YAY value representation.

use num_bigint::BigInt;
use std::collections::HashMap;
use std::fmt;

/// A YAY value.
#[derive(Clone, PartialEq)]
pub enum Value {
    /// Null value.
    Null,
    /// Boolean value.
    Bool(bool),
    /// Arbitrary-precision integer.
    Integer(BigInt),
    /// 64-bit floating-point number.
    Float(f64),
    /// UTF-8 string.
    String(String),
    /// Array of values.
    Array(Vec<Value>),
    /// Object (key-value map).
    Object(HashMap<String, Value>),
    /// Byte array.
    Bytes(Vec<u8>),
}

impl Value {
    /// Returns `true` if this value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Returns the boolean value if this is a `Bool`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns a reference to the integer if this is an `Integer`.
    pub fn as_integer(&self) -> Option<&BigInt> {
        match self {
            Value::Integer(n) => Some(n),
            _ => None,
        }
    }

    /// Returns the float value if this is a `Float`.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Returns a reference to the string if this is a `String`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns a reference to the array if this is an `Array`.
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Returns a reference to the object if this is an `Object`.
    pub fn as_object(&self) -> Option<&HashMap<String, Value>> {
        match self {
            Value::Object(obj) => Some(obj),
            _ => None,
        }
    }

    /// Returns a reference to the bytes if this is a `Bytes`.
    pub fn as_bytes(&self) -> Option<&Vec<u8>> {
        match self {
            Value::Bytes(b) => Some(b),
            _ => None,
        }
    }

    /// Returns a description of why this value cannot be represented in JSON,
    /// or `None` if it can be represented.
    ///
    /// JSON cannot represent:
    /// - Byte arrays (no binary data type)
    /// - Big integers (numbers larger than JavaScript's safe integer range)
    pub fn json_incompatibility(&self) -> Option<&'static str> {
        match self {
            Value::Bytes(_) => Some("byte arrays"),
            // YAY integers are always BigInts, which JSON cannot represent
            Value::Integer(_) => Some("integers (YAY integers are BigInts)"),
            Value::Array(arr) => {
                for v in arr {
                    if let Some(reason) = v.json_incompatibility() {
                        return Some(reason);
                    }
                }
                None
            }
            Value::Object(obj) => {
                for v in obj.values() {
                    if let Some(reason) = v.json_incompatibility() {
                        return Some(reason);
                    }
                }
                None
            }
            _ => None,
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Integer(n) => write!(f, "{}n", n),
            Value::Float(n) => {
                if n.is_nan() {
                    write!(f, "NaN")
                } else if n.is_infinite() {
                    if *n > 0.0 {
                        write!(f, "Infinity")
                    } else {
                        write!(f, "-Infinity")
                    }
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::String(s) => write!(f, "{:?}", s),
            Value::Array(arr) => f.debug_list().entries(arr).finish(),
            Value::Object(obj) => f.debug_map().entries(obj).finish(),
            Value::Bytes(b) => {
                write!(f, "<")?;
                for byte in b {
                    write!(f, "{:02x}", byte)?;
                }
                write!(f, ">")
            }
        }
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<BigInt> for Value {
    fn from(n: BigInt) -> Self {
        Value::Integer(n)
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Integer(BigInt::from(n))
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<Vec<Value>> for Value {
    fn from(arr: Vec<Value>) -> Self {
        Value::Array(arr)
    }
}

impl From<HashMap<String, Value>> for Value {
    fn from(obj: HashMap<String, Value>) -> Self {
        Value::Object(obj)
    }
}

impl From<Vec<u8>> for Value {
    fn from(b: Vec<u8>) -> Self {
        Value::Bytes(b)
    }
}
