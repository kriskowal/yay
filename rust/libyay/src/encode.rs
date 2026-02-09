//! Encode YAY values to various output formats.
//!
//! This module provides functions to convert YAY values into source code
//! literals for various programming languages, as well as YSON format.

use crate::Value;
use std::collections::HashMap;

/// Output format for encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// YAY format (canonical)
    Yay,
    /// JavaScript/TypeScript
    JavaScript,
    /// Go
    Go,
    /// Python
    Python,
    /// Rust
    Rust,
    /// C
    C,
    /// Java
    Java,
    /// Scheme
    Scheme,
    /// JSON (standard)
    Json,
    /// YSON (JSON with YAY extensions, a subset of Endo SmallCaps)
    Yson,
    /// YAML
    Yaml,
    /// TOML
    Toml,
    /// CBOR (binary)
    Cbor,
    /// CBOR diagnostic notation (RFC 8949 §8)
    CborDiag,
}

/// Encode a YAY value to a string in the specified format.
///
/// Note: YAML, TOML, and CBOR formats are handled externally by the CLI tool
/// (binyay) using dedicated libraries. Calling this function with those formats
/// will panic.
pub fn encode(value: &Value, format: Format) -> String {
    match format {
        Format::Yay => encode_yay(value, 0),
        Format::JavaScript => encode_js(value, 0),
        Format::Go => encode_go(value, 0),
        Format::Python => encode_python(value, 0),
        Format::Rust => encode_rust(value, 0),
        Format::C => encode_c(value),
        Format::Java => encode_java(value, 0),
        Format::Scheme => encode_scheme(value),
        Format::Json => encode_json(value, 0),
        Format::Yson => encode_yson(value, 0),
        Format::Yaml | Format::Toml | Format::Cbor | Format::CborDiag => {
            panic!(
                "Format {:?} must be handled by the CLI tool, not libyay::encode",
                format
            )
        }
    }
}

// =============================================================================
// YAY Encoder
// =============================================================================

fn encode_yay(value: &Value, indent: usize) -> String {
    let pad = "  ".repeat(indent);

    match value {
        Value::Null => "null".to_string(),
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::Integer(n) => n.to_string(),
        Value::Float(f) => {
            if f.is_nan() {
                "nan".to_string()
            } else if f.is_infinite() {
                if *f > 0.0 {
                    "infinity".to_string()
                } else {
                    "-infinity".to_string()
                }
            } else if *f == 0.0 && f.is_sign_negative() {
                "-0.0".to_string()
            } else {
                let s = format!("{}", f);
                if s.contains('.') || s.contains('e') {
                    s
                } else {
                    format!("{}.0", s)
                }
            }
        }
        Value::String(s) => encode_yay_string(s),
        Value::Bytes(b) => encode_yay_bytes(b),
        Value::Array(arr) => {
            if arr.is_empty() {
                "[]".to_string()
            } else if can_inline_array(arr) {
                let items: Vec<String> = arr.iter().map(|v| encode_yay(v, 0)).collect();
                format!("[{}]", items.join(", "))
            } else {
                encode_yay_multiline_array(arr, indent)
            }
        }
        Value::Object(obj) => {
            if obj.is_empty() {
                "{}".to_string()
            } else if can_inline_object(obj) {
                let mut keys: Vec<&String> = obj.keys().collect();
                keys.sort();
                let items: Vec<String> = keys
                    .iter()
                    .map(|k| format!("{}: {}", encode_yay_key(k), encode_yay(&obj[*k], 0)))
                    .collect();
                format!("{{{}}}", items.join(", "))
            } else {
                let mut keys: Vec<&String> = obj.keys().collect();
                keys.sort();
                let items: Vec<String> = keys
                    .iter()
                    .map(|k| {
                        let v = &obj[*k];
                        if is_block_value(v) {
                            // Nested block value: put on next line
                            format!(
                                "{}{}:\n{}",
                                pad,
                                encode_yay_key(k),
                                encode_yay(v, indent + 1)
                            )
                        } else {
                            format!(
                                "{}{}: {}",
                                pad,
                                encode_yay_key(k),
                                encode_yay(v, indent + 1)
                            )
                        }
                    })
                    .collect();
                items.join("\n")
            }
        }
    }
}

fn encode_yay_string(s: &str) -> String {
    // Use double quotes and escape special characters
    let mut result = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '/' => result.push_str("\\/"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\x08' => result.push_str("\\b"),
            '\x0c' => result.push_str("\\f"),
            c if c.is_control() => {
                result.push_str(&format!("\\u{{{:X}}}", c as u32));
            }
            c => result.push(c),
        }
    }
    result.push('"');
    result
}

fn encode_yay_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        "<>".to_string()
    } else {
        let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
        format!("<{}>", hex)
    }
}

fn encode_yay_key(key: &str) -> String {
    // Check if key needs quoting
    if key
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        && !key.is_empty()
    {
        key.to_string()
    } else {
        encode_yay_string(key)
    }
}

fn can_inline_array(arr: &[Value]) -> bool {
    arr.len() <= 5 && arr.iter().all(|v| is_simple_value(v))
}

fn can_inline_object(obj: &HashMap<String, Value>) -> bool {
    obj.len() <= 3 && obj.values().all(|v| is_simple_value(v))
}

fn is_simple_value(v: &Value) -> bool {
    matches!(
        v,
        Value::Null
            | Value::Bool(_)
            | Value::Integer(_)
            | Value::Float(_)
            | Value::String(_)
            | Value::Bytes(_)
    )
}

fn is_block_value(v: &Value) -> bool {
    match v {
        Value::Array(arr) => !can_inline_array(arr),
        Value::Object(obj) => !can_inline_object(obj),
        _ => false,
    }
}

fn encode_yay_multiline_array(arr: &[Value], indent: usize) -> String {
    let pad = "  ".repeat(indent);
    let mut result = Vec::new();

    for (i, v) in arr.iter().enumerate() {
        if i == 0 {
            // First item: no leading pad (caller handles it)
            let encoded = encode_yay_array_item(v, indent);
            result.push(format!("- {}", encoded));
        } else {
            let encoded = encode_yay_array_item(v, indent);
            result.push(format!("{}- {}", pad, encoded));
        }
    }

    result.join("\n")
}

fn encode_yay_array_item(v: &Value, indent: usize) -> String {
    match v {
        Value::Array(arr) if !can_inline_array(arr) => {
            // Nested multiline array: first item on same line, rest indented
            let inner_pad = "  ".repeat(indent + 1);
            let mut items = Vec::new();
            for (i, item) in arr.iter().enumerate() {
                let encoded = encode_yay_array_item(item, indent + 1);
                if i == 0 {
                    // First item: add "- " prefix on same line
                    items.push(format!("- {}", encoded));
                } else {
                    items.push(format!("{}- {}", inner_pad, encoded));
                }
            }
            items.join("\n")
        }
        Value::Object(obj) if !can_inline_object(obj) => {
            // Nested multiline object
            let inner_pad = "  ".repeat(indent + 1);
            let mut keys: Vec<&String> = obj.keys().collect();
            keys.sort();
            let items: Vec<String> = keys
                .iter()
                .enumerate()
                .map(|(i, k)| {
                    let v = &obj[*k];
                    if is_block_value(v) {
                        // Block value: put on next line with proper indentation
                        let encoded = encode_yay(v, indent + 2);
                        if i == 0 {
                            format!("{}:\n{}", encode_yay_key(k), encoded)
                        } else {
                            format!("{}{}:\n{}", inner_pad, encode_yay_key(k), encoded)
                        }
                    } else {
                        let encoded = encode_yay(v, indent + 2);
                        if i == 0 {
                            format!("{}: {}", encode_yay_key(k), encoded)
                        } else {
                            format!("{}{}: {}", inner_pad, encode_yay_key(k), encoded)
                        }
                    }
                })
                .collect();
            items.join("\n")
        }
        _ => encode_yay(v, indent + 1),
    }
}

// =============================================================================
// JavaScript Encoder
// =============================================================================

fn encode_js(value: &Value, indent: usize) -> String {
    encode_js_inner(value, indent, true)
}

fn encode_js_inner(value: &Value, indent: usize, is_top_level: bool) -> String {
    let pad = "  ".repeat(indent);
    let pad1 = "  ".repeat(indent + 1);

    match value {
        Value::Null => "null".to_string(),
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::Integer(n) => format!("{}n", n),
        Value::Float(f) => {
            if f.is_nan() {
                "NaN".to_string()
            } else if f.is_infinite() {
                if *f > 0.0 {
                    "Infinity".to_string()
                } else {
                    "-Infinity".to_string()
                }
            } else if *f == 0.0 && f.is_sign_negative() {
                "-0".to_string()
            } else {
                format!("{}", f)
            }
        }
        Value::String(s) => encode_js_string(s),
        Value::Bytes(b) => {
            if b.is_empty() {
                "new Uint8Array(0)".to_string()
            } else {
                let items: Vec<String> = b.iter().map(|byte| format!("0x{:02x}", byte)).collect();
                format!("Uint8Array.from([{}])", items.join(", "))
            }
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                "[]".to_string()
            } else {
                let items: Vec<String> = arr
                    .iter()
                    .map(|v| encode_js_inner(v, indent + 1, false))
                    .collect();
                let joined = items.join(", ");
                // Use multiline if: any item has newlines, or total line too long
                if joined.len() < 60 && !items.iter().any(|i| i.contains('\n')) {
                    format!("[{}]", joined)
                } else {
                    // Multiline with trailing comma
                    format!(
                        "[\n{},\n{}]",
                        items
                            .iter()
                            .map(|i| format!("{}{}", pad1, i))
                            .collect::<Vec<_>>()
                            .join(",\n"),
                        pad
                    )
                }
            }
        }
        Value::Object(obj) => {
            if obj.is_empty() {
                if is_top_level {
                    "({})".to_string()
                } else {
                    "{}".to_string()
                }
            } else {
                let mut keys: Vec<&String> = obj.keys().collect();
                keys.sort();
                let items: Vec<String> = keys
                    .iter()
                    .map(|k| {
                        format!(
                            "{}: {}",
                            encode_js_string(k),
                            encode_js_inner(&obj[*k], indent + 1, false)
                        )
                    })
                    .collect();
                let joined = items.join(", ");
                // Use multiline if: any item has newlines, or total line too long
                if joined.len() < 60 && !items.iter().any(|i| i.contains('\n')) {
                    if is_top_level {
                        format!("({{ {} }})", joined)
                    } else {
                        format!("{{ {} }}", joined)
                    }
                } else {
                    // Multiline with trailing comma
                    if is_top_level {
                        format!(
                            "({{\n{},\n{}}})",
                            items
                                .iter()
                                .map(|i| format!("{}{}", pad1, i))
                                .collect::<Vec<_>>()
                                .join(",\n"),
                            pad
                        )
                    } else {
                        format!(
                            "{{\n{},\n{}}}",
                            items
                                .iter()
                                .map(|i| format!("{}{}", pad1, i))
                                .collect::<Vec<_>>()
                                .join(",\n"),
                            pad
                        )
                    }
                }
            }
        }
    }
}

// =============================================================================
// Go Encoder
// =============================================================================

fn encode_go(value: &Value, indent: usize) -> String {
    let pad = "\t".repeat(indent);
    let pad1 = "\t".repeat(indent + 1);

    match value {
        Value::Null => "nil".to_string(),
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::Integer(n) => format!("big.NewInt({})", n),
        Value::Float(f) => {
            if f.is_nan() {
                "math.NaN()".to_string()
            } else if f.is_infinite() {
                if *f > 0.0 {
                    "math.Inf(1)".to_string()
                } else {
                    "math.Inf(-1)".to_string()
                }
            } else if *f == 0.0 && f.is_sign_negative() {
                "math.Copysign(0, -1)".to_string()
            } else {
                let s = format!("{}", f);
                if s.contains('.') || s.contains('e') {
                    s
                } else {
                    format!("{}.0", s)
                }
            }
        }
        Value::String(s) => encode_json_string(s),
        Value::Bytes(b) => {
            if b.is_empty() {
                "[]byte{}".to_string()
            } else {
                let items: Vec<String> = b.iter().map(|byte| format!("0x{:02x}", byte)).collect();
                format!("[]byte{{{}}}", items.join(", "))
            }
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                "[]any{}".to_string()
            } else {
                let items: Vec<String> = arr.iter().map(|v| encode_go(v, indent + 1)).collect();
                let joined = items.join(", ");
                // Use multiline if too long or any item has newlines
                if joined.len() < 60 && !items.iter().any(|i| i.contains('\n')) {
                    format!("[]any{{{}}}", joined)
                } else {
                    format!(
                        "[]any{{\n{},\n{}}}",
                        items
                            .iter()
                            .map(|i| format!("{}{}", pad1, i))
                            .collect::<Vec<_>>()
                            .join(",\n"),
                        pad
                    )
                }
            }
        }
        Value::Object(obj) => {
            if obj.is_empty() {
                "map[string]any{}".to_string()
            } else {
                let mut keys: Vec<&String> = obj.keys().collect();
                keys.sort();
                let items: Vec<String> = keys
                    .iter()
                    .map(|k| {
                        format!(
                            "{}: {}",
                            encode_json_string(k),
                            encode_go(&obj[*k], indent + 1)
                        )
                    })
                    .collect();
                let joined = items.join(", ");
                // Use multiline if too long or any item has newlines
                if joined.len() < 60 && !items.iter().any(|i| i.contains('\n')) {
                    format!("map[string]any{{{}}}", joined)
                } else {
                    format!(
                        "map[string]any{{\n{},\n{}}}",
                        items
                            .iter()
                            .map(|i| format!("{}{}", pad1, i))
                            .collect::<Vec<_>>()
                            .join(",\n"),
                        pad
                    )
                }
            }
        }
    }
}

// =============================================================================
// Python Encoder
// =============================================================================

fn encode_python(value: &Value, indent: usize) -> String {
    let pad = "    ".repeat(indent);
    let pad1 = "    ".repeat(indent + 1);

    match value {
        Value::Null => "None".to_string(),
        Value::Bool(true) => "True".to_string(),
        Value::Bool(false) => "False".to_string(),
        Value::Integer(n) => n.to_string(),
        Value::Float(f) => {
            if f.is_nan() {
                "float(\"nan\")".to_string()
            } else if f.is_infinite() {
                if *f > 0.0 {
                    "float(\"inf\")".to_string()
                } else {
                    "float(\"-inf\")".to_string()
                }
            } else if *f == 0.0 && f.is_sign_negative() {
                "-0.0".to_string()
            } else {
                let s = format!("{}", f);
                if s.contains('.') || s.contains('e') {
                    s
                } else {
                    format!("{}.0", s)
                }
            }
        }
        Value::String(s) => encode_json_string(s),
        Value::Bytes(b) => {
            if b.is_empty() {
                "b''".to_string()
            } else {
                let hex: String = b.iter().map(|byte| format!("{:02x}", byte)).collect();
                format!("bytes.fromhex(\"{}\")", hex)
            }
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                "[]".to_string()
            } else {
                let items: Vec<String> = arr.iter().map(|v| encode_python(v, 0)).collect();
                let single_line = format!("[{}]", items.join(", "));
                if !items.iter().any(|i| i.contains('\n')) {
                    single_line
                } else {
                    let items: Vec<String> =
                        arr.iter().map(|v| encode_python(v, indent + 1)).collect();
                    format!(
                        "[\n{}\n{}]",
                        items
                            .iter()
                            .map(|i| format!("{}{},", pad1, i))
                            .collect::<Vec<_>>()
                            .join("\n"),
                        pad
                    )
                }
            }
        }
        Value::Object(obj) => {
            if obj.is_empty() {
                "{}".to_string()
            } else {
                let mut keys: Vec<&String> = obj.keys().collect();
                keys.sort();
                let items: Vec<String> = keys
                    .iter()
                    .map(|k| format!("{}: {}", encode_json_string(k), encode_python(&obj[*k], 0)))
                    .collect();
                let single_line = format!("{{{}}}", items.join(", "));
                if !items.iter().any(|i| i.contains('\n')) {
                    single_line
                } else {
                    let items: Vec<String> = keys
                        .iter()
                        .map(|k| {
                            format!(
                                "{}: {}",
                                encode_json_string(k),
                                encode_python(&obj[*k], indent + 1)
                            )
                        })
                        .collect();
                    format!(
                        "{{\n{}\n{}}}",
                        items
                            .iter()
                            .map(|i| format!("{}{},", pad1, i))
                            .collect::<Vec<_>>()
                            .join("\n"),
                        pad
                    )
                }
            }
        }
    }
}

// =============================================================================
// Rust Encoder
// =============================================================================

fn encode_rust(value: &Value, indent: usize) -> String {
    let pad = "    ".repeat(indent);
    let pad1 = "    ".repeat(indent + 1);

    match value {
        Value::Null => "Value::Null".to_string(),
        Value::Bool(b) => format!("Value::Bool({})", b),
        Value::Integer(n) => format!("Value::Integer({}.into())", n),
        Value::Float(f) => {
            if f.is_nan() {
                "Value::Float(f64::NAN)".to_string()
            } else if f.is_infinite() {
                if *f > 0.0 {
                    "Value::Float(f64::INFINITY)".to_string()
                } else {
                    "Value::Float(f64::NEG_INFINITY)".to_string()
                }
            } else if *f == 0.0 && f.is_sign_negative() {
                "Value::Float(-0.0)".to_string()
            } else {
                let s = format!("{}", f);
                if s.contains('.') || s.contains('e') {
                    format!("Value::Float({})", s)
                } else {
                    format!("Value::Float({}.0)", s)
                }
            }
        }
        Value::String(s) => format!("Value::String({}.into())", encode_json_string(s)),
        Value::Bytes(b) => {
            if b.is_empty() {
                "Value::Bytes(vec![])".to_string()
            } else {
                let items: Vec<String> = b.iter().map(|byte| format!("0x{:02x}", byte)).collect();
                format!("Value::Bytes(vec![{}])", items.join(", "))
            }
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                "Value::Array(vec![])".to_string()
            } else {
                let items: Vec<String> = arr.iter().map(|v| encode_rust(v, indent + 1)).collect();
                let single_line = format!("Value::Array(vec![{}])", items.join(", "));
                if single_line.len() < 50 && !single_line.contains('\n') {
                    single_line
                } else {
                    format!(
                        "Value::Array(vec![\n{}\n{}])",
                        items
                            .iter()
                            .map(|i| format!("{}{},", pad1, i))
                            .collect::<Vec<_>>()
                            .join("\n"),
                        pad
                    )
                }
            }
        }
        Value::Object(obj) => {
            if obj.is_empty() {
                "Value::Object(HashMap::new())".to_string()
            } else {
                let mut keys: Vec<&String> = obj.keys().collect();
                keys.sort();
                let items: Vec<String> = keys
                    .iter()
                    .map(|k| {
                        format!(
                            "({}.into(), {})",
                            encode_json_string(k),
                            encode_rust(&obj[*k], indent + 1)
                        )
                    })
                    .collect();
                format!(
                    "Value::Object(HashMap::from([\n{}\n{}]))",
                    items
                        .iter()
                        .map(|i| format!("{}{},", pad1, i))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    pad
                )
            }
        }
    }
}

// =============================================================================
// C Encoder
// =============================================================================

const C_INDENT: &str = "    ";
const C_MAX_LINE: usize = 72;

fn encode_c(value: &Value) -> String {
    encode_c_value(value, 0)
}

fn encode_c_value(value: &Value, indent: usize) -> String {
    match value {
        Value::Null => "yay_null()".to_string(),
        Value::Bool(true) => "yay_bool(true)".to_string(),
        Value::Bool(false) => "yay_bool(false)".to_string(),
        Value::Integer(n) => format!("yay_int({})", n),
        Value::Float(f) => {
            if f.is_nan() {
                "yay_float(NAN)".to_string()
            } else if f.is_infinite() {
                if *f > 0.0 {
                    "yay_float(INFINITY)".to_string()
                } else {
                    "yay_float(-INFINITY)".to_string()
                }
            } else if *f == 0.0 && f.is_sign_negative() {
                "yay_float(-0.0)".to_string()
            } else {
                let s = format!("{}", f);
                if s.contains('.') || s.contains('e') {
                    format!("yay_float({})", s)
                } else {
                    format!("yay_float({}.0)", s)
                }
            }
        }
        Value::String(s) => format!("yay_string({})", encode_c_string(s)),
        Value::Bytes(b) => {
            if b.is_empty() {
                "yay_bytes_from_hex(\"\")".to_string()
            } else {
                let hex: String = b.iter().map(|byte| format!("{:02x}", byte)).collect();
                format!("yay_bytes_from_hex(\"{}\")", hex)
            }
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                "yay_array()".to_string()
            } else {
                let items: Vec<String> =
                    arr.iter().map(|v| encode_c_value(v, indent + 1)).collect();
                format_c_macro("YAY_ARRAY", &items, indent)
            }
        }
        Value::Object(obj) => {
            if obj.is_empty() {
                "yay_object()".to_string()
            } else {
                let mut keys: Vec<&String> = obj.keys().collect();
                keys.sort();
                let items: Vec<String> = keys
                    .iter()
                    .flat_map(|k| vec![encode_c_string(k), encode_c_value(&obj[*k], indent + 1)])
                    .collect();
                format_c_macro("YAY_OBJECT", &items, indent)
            }
        }
    }
}

/// Format a macro call, inlining if short enough, otherwise one arg per line.
/// For YAY_OBJECT, keeps key-value pairs together on the same line when they fit.
fn format_c_macro(name: &str, args: &[String], indent: usize) -> String {
    // Try single-line first
    let single = format!("{}({})", name, args.join(", "));
    let prefix_len = indent * C_INDENT.len();
    if prefix_len + single.len() <= C_MAX_LINE {
        return single;
    }

    let pad = C_INDENT.repeat(indent + 1);
    let mut result = format!("{}(\n", name);

    if name == "YAY_OBJECT" {
        // Group args in pairs: key, value
        for (i, chunk) in args.chunks(2).enumerate() {
            let pair = chunk.join(", ");
            let is_last = i == args.len() / 2 - 1;
            let suffix = if is_last { "" } else { "," };
            result.push_str(&pad);
            result.push_str(&pair);
            result.push_str(suffix);
            result.push('\n');
        }
    } else {
        for (i, arg) in args.iter().enumerate() {
            let is_last = i == args.len() - 1;
            let suffix = if is_last { "" } else { "," };
            result.push_str(&pad);
            result.push_str(arg);
            result.push_str(suffix);
            result.push('\n');
        }
    }

    result.push_str(&C_INDENT.repeat(indent));
    result.push(')');
    result
}

fn encode_c_string(s: &str) -> String {
    let mut result = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\x08' => result.push_str("\\b"),
            '\x0c' => result.push_str("\\f"),
            c if c.is_control() => {
                result.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result.push('"');
    result
}

// =============================================================================
// Java Encoder
// =============================================================================

fn encode_java(value: &Value, indent: usize) -> String {
    let pad = "    ".repeat(indent);
    let pad1 = "    ".repeat(indent + 1);

    match value {
        Value::Null => "null".to_string(),
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::Integer(n) => format!("BigInteger.valueOf({})", n),
        Value::Float(f) => {
            if f.is_nan() {
                "Double.NaN".to_string()
            } else if f.is_infinite() {
                if *f > 0.0 {
                    "Double.POSITIVE_INFINITY".to_string()
                } else {
                    "Double.NEGATIVE_INFINITY".to_string()
                }
            } else if *f == 0.0 && f.is_sign_negative() {
                "-0.0".to_string()
            } else {
                let s = format!("{}", f);
                if s.contains('.') || s.contains('e') {
                    s
                } else {
                    format!("{}.0", s)
                }
            }
        }
        Value::String(s) => encode_java_string(s),
        Value::Bytes(b) => {
            if b.is_empty() {
                "new byte[0]".to_string()
            } else {
                let items: Vec<String> = b
                    .iter()
                    .map(|byte| format!("(byte) 0x{:02x}", byte))
                    .collect();
                format!("new byte[] {{{}}}", items.join(", "))
            }
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                "List.of()".to_string()
            } else {
                // Always try single-line first, only go multi-line if items are multi-line
                let items: Vec<String> = arr.iter().map(|v| encode_java(v, 0)).collect();
                let single_line = format!("List.of({})", items.join(", "));
                if !items.iter().any(|i| i.contains('\n')) {
                    single_line
                } else {
                    let items: Vec<String> =
                        arr.iter().map(|v| encode_java(v, indent + 1)).collect();
                    format!(
                        "List.of(\n{}\n{})",
                        items
                            .iter()
                            .map(|i| format!("{}{}", pad1, i))
                            .collect::<Vec<_>>()
                            .join(",\n"),
                        pad
                    )
                }
            }
        }
        Value::Object(obj) => {
            if obj.is_empty() {
                "Map.of()".to_string()
            } else {
                let mut keys: Vec<&String> = obj.keys().collect();
                keys.sort();
                // Always try single-line first
                let items: Vec<String> = keys
                    .iter()
                    .flat_map(|k| vec![encode_java_string(k), encode_java(&obj[*k], 0)])
                    .collect();
                let single_line = format!("Map.of({})", items.join(", "));
                if !items.iter().any(|i| i.contains('\n')) {
                    single_line
                } else {
                    let pairs: Vec<String> = keys
                        .iter()
                        .map(|k| {
                            format!(
                                "{}{}, {}",
                                pad1,
                                encode_java_string(k),
                                encode_java(&obj[*k], indent + 1)
                            )
                        })
                        .collect();
                    format!("Map.of(\n{}\n{})", pairs.join(",\n"), pad)
                }
            }
        }
    }
}

fn encode_java_string(s: &str) -> String {
    let mut result = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\x08' => result.push_str("\\b"),
            '\x0c' => result.push_str("\\f"),
            c => result.push(c),
        }
    }
    result.push('"');
    result
}

// =============================================================================
// Scheme Encoder
// =============================================================================

fn encode_scheme(value: &Value) -> String {
    match value {
        Value::Null => "'null".to_string(),
        Value::Bool(true) => "#t".to_string(),
        Value::Bool(false) => "#f".to_string(),
        Value::Integer(n) => n.to_string(),
        Value::Float(f) => {
            if f.is_nan() {
                "+nan.0".to_string()
            } else if f.is_infinite() {
                if *f > 0.0 {
                    "+inf.0".to_string()
                } else {
                    "-inf.0".to_string()
                }
            } else if *f == 0.0 && f.is_sign_negative() {
                "-0.0".to_string()
            } else {
                let s = format!("{}", f);
                if s.contains('.') || s.contains('e') {
                    s
                } else {
                    format!("{}.0", s)
                }
            }
        }
        Value::String(s) => encode_scheme_string(s),
        Value::Bytes(b) => {
            if b.is_empty() {
                "(bytevector)".to_string()
            } else {
                let items: Vec<String> = b.iter().map(|byte| byte.to_string()).collect();
                format!("(bytevector {})", items.join(" "))
            }
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                "#()".to_string()
            } else {
                let items: Vec<String> = arr.iter().map(|v| encode_scheme(v)).collect();
                format!("#({})", items.join(" "))
            }
        }
        Value::Object(obj) => {
            if obj.is_empty() {
                "()".to_string()
            } else {
                let mut keys: Vec<&String> = obj.keys().collect();
                keys.sort();
                let items: Vec<String> = keys
                    .iter()
                    .map(|k| {
                        format!(
                            "({} . {})",
                            encode_scheme_string(k),
                            encode_scheme(&obj[*k])
                        )
                    })
                    .collect();
                format!("({})", items.join(" "))
            }
        }
    }
}

fn encode_scheme_string(s: &str) -> String {
    let mut result = String::from("\"");
    for c in s.chars() {
        let code = c as u32;
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '/' => result.push_str("\\/"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\x08' => result.push_str("\\b"),
            '\x0c' => result.push_str("\\f"),
            _ if code > 127 && code <= 0xFFFF => {
                result.push_str(&format!("\\u{:04X}", code));
            }
            _ if code > 0xFFFF => {
                // Emoji or other high code points - keep as literal
                result.push(c);
            }
            c => result.push(c),
        }
    }
    result.push('"');
    result
}

// =============================================================================
// JSON Encoder
// =============================================================================

fn encode_json(value: &Value, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    let pad1 = "  ".repeat(indent + 1);

    match value {
        Value::Null => "null".to_string(),
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::Integer(n) => n.to_string(),
        Value::Float(f) => {
            if f.is_nan() || f.is_infinite() {
                "null".to_string() // JSON doesn't support NaN/Infinity
            } else {
                format!("{}", f)
            }
        }
        Value::String(s) => encode_json_string(s),
        Value::Bytes(_) => "null".to_string(), // JSON doesn't support bytes
        Value::Array(arr) => {
            if arr.is_empty() {
                "[]".to_string()
            } else {
                let items: Vec<String> = arr.iter().map(|v| encode_json(v, indent + 1)).collect();
                format!(
                    "[\n{}\n{}]",
                    items
                        .iter()
                        .map(|i| format!("{}{}", pad1, i))
                        .collect::<Vec<_>>()
                        .join(",\n"),
                    pad
                )
            }
        }
        Value::Object(obj) => {
            if obj.is_empty() {
                "{}".to_string()
            } else {
                let mut keys: Vec<&String> = obj.keys().collect();
                keys.sort();
                let items: Vec<String> = keys
                    .iter()
                    .map(|k| {
                        format!(
                            "{}: {}",
                            encode_json_string(k),
                            encode_json(&obj[*k], indent + 1)
                        )
                    })
                    .collect();
                format!(
                    "{{\n{}\n{}}}",
                    items
                        .iter()
                        .map(|i| format!("{}{}", pad1, i))
                        .collect::<Vec<_>>()
                        .join(",\n"),
                    pad
                )
            }
        }
    }
}

fn encode_json_string(s: &str) -> String {
    let mut result = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\x08' => result.push_str("\\b"),
            '\x0c' => result.push_str("\\f"),
            c if c.is_control() => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result.push('"');
    result
}

/// Encode a string for JavaScript, preferring single quotes when the string
/// contains double quotes but no single quotes (reduces escaping).
fn encode_js_string(s: &str) -> String {
    let has_double = s.contains('"');
    let has_single = s.contains('\'');

    // Prefer single quotes if string has double quotes but no single quotes
    if has_double && !has_single {
        let mut result = String::from("'");
        for c in s.chars() {
            match c {
                '\\' => result.push_str("\\\\"),
                '\n' => result.push_str("\\n"),
                '\r' => result.push_str("\\r"),
                '\t' => result.push_str("\\t"),
                '\x08' => result.push_str("\\b"),
                '\x0c' => result.push_str("\\f"),
                c if c.is_control() => {
                    result.push_str(&format!("\\u{:04x}", c as u32));
                }
                c => result.push(c),
            }
        }
        result.push('\'');
        result
    } else {
        // Use double quotes (escape double quotes if present)
        encode_json_string(s)
    }
}

// =============================================================================
// YSON Encoder
// =============================================================================

fn encode_yson(value: &Value, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    let pad1 = "  ".repeat(indent + 1);

    match value {
        Value::Null => "null".to_string(),
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::Integer(n) => format!("\"#{}\"", n), // BigInt prefix
        Value::Float(f) => {
            if f.is_nan() {
                "\"#NaN\"".to_string()
            } else if f.is_infinite() {
                if *f > 0.0 {
                    "\"#Infinity\"".to_string()
                } else {
                    "\"#-Infinity\"".to_string()
                }
            } else {
                format!("{}", f)
            }
        }
        Value::String(s) => encode_yson_string(s),
        Value::Bytes(b) => {
            // Bytes prefix
            let hex: String = b.iter().map(|byte| format!("{:02x}", byte)).collect();
            format!("\"*{}\"", hex)
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                "[]".to_string()
            } else {
                let items: Vec<String> = arr.iter().map(|v| encode_yson(v, indent + 1)).collect();
                format!(
                    "[\n{}\n{}]",
                    items
                        .iter()
                        .map(|i| format!("{}{}", pad1, i))
                        .collect::<Vec<_>>()
                        .join(",\n"),
                    pad
                )
            }
        }
        Value::Object(obj) => {
            if obj.is_empty() {
                "{}".to_string()
            } else {
                let mut keys: Vec<&String> = obj.keys().collect();
                keys.sort();
                let items: Vec<String> = keys
                    .iter()
                    .map(|k| {
                        format!(
                            "{}: {}",
                            encode_json_string(k),
                            encode_yson(&obj[*k], indent + 1)
                        )
                    })
                    .collect();
                format!(
                    "{{\n{}\n{}}}",
                    items
                        .iter()
                        .map(|i| format!("{}{}", pad1, i))
                        .collect::<Vec<_>>()
                        .join(",\n"),
                    pad
                )
            }
        }
    }
}

fn encode_yson_string(s: &str) -> String {
    // Check if string starts with a reserved prefix (! through /)
    let needs_escape = s
        .chars()
        .next()
        .map(|c| c >= '!' && c <= '/')
        .unwrap_or(false);

    if needs_escape {
        // Escape with ! prefix
        format!("\"!{}\"", &encode_json_string(s)[1..s.len() + 1])
    } else {
        encode_json_string(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_js_integer() {
        let value = Value::Integer(42.into());
        assert_eq!(encode(&value, Format::JavaScript), "42n");
    }

    #[test]
    fn test_encode_go_integer() {
        let value = Value::Integer(42.into());
        assert_eq!(encode(&value, Format::Go), "big.NewInt(42)");
    }

    #[test]
    fn test_encode_yson_bytes() {
        let value = Value::Bytes(vec![0xca, 0xfe]);
        assert_eq!(encode(&value, Format::Yson), "\"*cafe\"");
    }

    #[test]
    fn test_encode_yson_bigint() {
        let value = Value::Integer(12345678901234567890u64.into());
        assert_eq!(encode(&value, Format::Yson), "\"#12345678901234567890\"");
    }

    #[test]
    fn test_encode_yson_escaped_string() {
        let value = Value::String("*hello".into());
        assert_eq!(encode(&value, Format::Yson), "\"!*hello\"");
    }

    #[test]
    fn test_encode_yson_nan() {
        let value = Value::Float(f64::NAN);
        assert_eq!(encode(&value, Format::Yson), "\"#NaN\"");
    }

    #[test]
    fn test_encode_yson_infinity() {
        let value = Value::Float(f64::INFINITY);
        assert_eq!(encode(&value, Format::Yson), "\"#Infinity\"");
    }

    #[test]
    fn test_encode_yson_neg_infinity() {
        let value = Value::Float(f64::NEG_INFINITY);
        assert_eq!(encode(&value, Format::Yson), "\"#-Infinity\"");
    }
}
