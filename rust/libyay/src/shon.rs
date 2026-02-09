//! SHON (Shell Object Notation) parser for YAY.
//!
//! SHON is a command-line notation for structured data. It parses shell
//! arguments into YAY `Value`s. SHON is activated by positional arguments
//! `[`, `-x`, `-b`, or `-s` in the CLI.
//!
//! See `SHON.md` for the full specification.

use num_bigint::BigInt;
use std::collections::HashMap;
use std::fs;
use std::str::FromStr;

use crate::Value;

/// Error type for SHON parsing.
#[derive(Debug, Clone, PartialEq)]
pub struct ShonError(pub String);

impl std::fmt::Display for ShonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ShonError {}

/// Parse a SHON compound value from CLI arguments starting with `[`, `[]`, or `[--]`.
/// The `args` slice should begin with the trigger token.
/// Returns the parsed value and the number of arguments consumed (including the brackets).
pub fn parse_shon_bracket(args: &[String]) -> Result<(Value, usize), ShonError> {
    if args.is_empty() {
        return Err(ShonError("Expected '[' to start SHON expression".into()));
    }
    match args[0].as_str() {
        "[]" => Ok((Value::Array(Vec::new()), 1)),
        "[--]" => Ok((Value::Object(HashMap::new()), 1)),
        "[" => {
            let mut pos = 1; // skip opening [
            let (value, consumed) = parse_bracket_contents(args, &mut pos)?;
            Ok((value, consumed))
        }
        _ => Err(ShonError("Expected '[' to start SHON expression".into())),
    }
}

/// Parse a SHON `-x <hex>` from CLI arguments.
/// The `args` slice should begin with the `-x` token.
/// Returns the parsed bytes value and the number of arguments consumed (2).
pub fn parse_shon_hex(args: &[String]) -> Result<(Value, usize), ShonError> {
    if args.is_empty() || args[0] != "-x" {
        return Err(ShonError("Expected '-x' for hex bytes".into()));
    }
    if args.len() < 2 {
        return Err(ShonError("-x requires a hex string argument".into()));
    }
    let bytes = parse_hex_token(&args[1])?;
    Ok((Value::Bytes(bytes), 2))
}

/// Parse a SHON `-b <file>` from CLI arguments.
/// Reads the file as raw bytes.
/// Returns the parsed bytes value and the number of arguments consumed (2).
pub fn parse_shon_file_bytes(args: &[String]) -> Result<(Value, usize), ShonError> {
    if args.is_empty() || args[0] != "-b" {
        return Err(ShonError("Expected '-b' for file bytes".into()));
    }
    if args.len() < 2 {
        return Err(ShonError("-b requires a file path argument".into()));
    }
    let bytes = fs::read(&args[1])
        .map_err(|e| ShonError(format!("Cannot read file '{}': {}", args[1], e)))?;
    Ok((Value::Bytes(bytes), 2))
}

/// Parse a SHON `-s <file>` from CLI arguments.
/// Reads the file as a UTF-8 string.
/// Returns the parsed string value and the number of arguments consumed (2).
pub fn parse_shon_file_string(args: &[String]) -> Result<(Value, usize), ShonError> {
    if args.is_empty() || args[0] != "-s" {
        return Err(ShonError("Expected '-s' for file string".into()));
    }
    if args.len() < 2 {
        return Err(ShonError("-s requires a file path argument".into()));
    }
    let content = fs::read_to_string(&args[1])
        .map_err(|e| ShonError(format!("Cannot read file '{}': {}", args[1], e)))?;
    Ok((Value::String(content), 2))
}

/// Parse the contents inside `[ ... ]`.
/// `pos` points to the first token after `[`.
/// Returns the value and the total number of args consumed from the original slice
/// (including the opening `[`).
fn parse_bracket_contents(args: &[String], pos: &mut usize) -> Result<(Value, usize), ShonError> {
    // Check for empty object `[--]`
    if *pos < args.len() && args[*pos] == "--]" {
        *pos += 1;
        return Ok((Value::Object(HashMap::new()), *pos));
    }

    // Check for `[]` (empty array as single token)
    if args[0] == "[]" {
        *pos = 1;
        return Ok((Value::Array(Vec::new()), 1));
    }

    // Peek ahead to determine if this is an object or array.
    // It's an object if the first non-escaped element is a --key.
    let is_object = peek_is_object(args, *pos);

    if is_object {
        parse_object_contents(args, pos)
    } else {
        parse_array_contents(args, pos)
    }
}

/// Look ahead to determine if bracket contents form an object.
/// An object starts with `--key` (a token starting with `--` followed by at least one char).
fn peek_is_object(args: &[String], pos: usize) -> bool {
    if pos >= args.len() {
        return false;
    }
    is_object_key(&args[pos])
}

/// Check if a token is an object key (`--word` where word is non-empty).
fn is_object_key(token: &str) -> bool {
    token.starts_with("--") && token.len() > 2
}

/// Parse object contents: `--key value --key value ... ]`
fn parse_object_contents(args: &[String], pos: &mut usize) -> Result<(Value, usize), ShonError> {
    let mut map = HashMap::new();

    loop {
        if *pos >= args.len() {
            return Err(ShonError("Unclosed '[': expected ']'".into()));
        }

        if args[*pos] == "]" {
            *pos += 1;
            return Ok((Value::Object(map), *pos));
        }

        // Expect --key
        if !is_object_key(&args[*pos]) {
            return Err(ShonError(format!(
                "Expected object key (--key) or ']', got '{}'",
                args[*pos]
            )));
        }

        let key = args[*pos][2..].to_string();
        *pos += 1;

        if *pos >= args.len() {
            return Err(ShonError(format!("Expected value after key '--{}'", key)));
        }

        let value = parse_value(args, pos)?;
        map.insert(key, value);
    }
}

/// Parse array contents: `value value ... ]`
fn parse_array_contents(args: &[String], pos: &mut usize) -> Result<(Value, usize), ShonError> {
    let mut items = Vec::new();

    loop {
        if *pos >= args.len() {
            return Err(ShonError("Unclosed '[': expected ']'".into()));
        }

        if args[*pos] == "]" {
            *pos += 1;
            return Ok((Value::Array(items), *pos));
        }

        let value = parse_value(args, pos)?;
        items.push(value);
    }
}

/// Parse a single SHON value at `args[*pos]`, advancing `*pos` past it.
fn parse_value(args: &[String], pos: &mut usize) -> Result<Value, ShonError> {
    if *pos >= args.len() {
        return Err(ShonError("Expected value, got end of arguments".into()));
    }

    let token = &args[*pos];

    match token.as_str() {
        // Nested array/object
        "[" => {
            *pos += 1;
            let (value, _) = parse_bracket_contents(args, pos)?;
            Ok(value)
        }
        // `[]` as single token
        "[]" => {
            *pos += 1;
            Ok(Value::Array(Vec::new()))
        }
        // `[--]` as single token
        "[--]" => {
            *pos += 1;
            Ok(Value::Object(HashMap::new()))
        }
        // String escape
        "--" => {
            *pos += 1;
            if *pos >= args.len() {
                return Err(ShonError(
                    "'--' requires a following token to escape".into(),
                ));
            }
            let s = args[*pos].clone();
            *pos += 1;
            Ok(Value::String(s))
        }
        // Reserved single-char flags
        "-n" => {
            *pos += 1;
            Ok(Value::Null)
        }
        "-t" => {
            *pos += 1;
            Ok(Value::Bool(true))
        }
        "-f" => {
            *pos += 1;
            Ok(Value::Bool(false))
        }
        "-I" => {
            *pos += 1;
            Ok(Value::Float(f64::INFINITY))
        }
        "-i" => {
            *pos += 1;
            Ok(Value::Float(f64::NEG_INFINITY))
        }
        "-N" => {
            *pos += 1;
            Ok(Value::Float(f64::NAN))
        }
        // Hex bytes
        "-x" => {
            *pos += 1;
            if *pos >= args.len() {
                return Err(ShonError("-x requires a hex string argument".into()));
            }
            let bytes = parse_hex_token(&args[*pos])?;
            *pos += 1;
            Ok(Value::Bytes(bytes))
        }
        // File → bytes
        "-b" => {
            *pos += 1;
            if *pos >= args.len() {
                return Err(ShonError("-b requires a file path argument".into()));
            }
            let path = &args[*pos];
            let bytes = fs::read(path)
                .map_err(|e| ShonError(format!("Cannot read file '{}': {}", path, e)))?;
            *pos += 1;
            Ok(Value::Bytes(bytes))
        }
        // File → string
        "-s" => {
            *pos += 1;
            if *pos >= args.len() {
                return Err(ShonError("-s requires a file path argument".into()));
            }
            let path = &args[*pos];
            let content = fs::read_to_string(path)
                .map_err(|e| ShonError(format!("Cannot read file '{}': {}", path, e)))?;
            *pos += 1;
            Ok(Value::String(content))
        }
        // Unexpected close bracket
        "]" => Err(ShonError("Unexpected ']' without matching '['".into())),
        // Number or string
        _ => {
            *pos += 1;
            parse_atom(token)
        }
    }
}

/// Parse a bare token as a number or string.
fn parse_atom(token: &str) -> Result<Value, ShonError> {
    // Try integer: /^-?[0-9]+$/
    if is_integer(token) {
        return match BigInt::from_str(token) {
            Ok(n) => Ok(Value::Integer(n)),
            Err(_) => Ok(Value::String(token.to_string())),
        };
    }

    // Try float: /^-?[0-9]*\.[0-9]*([eE][+-]?[0-9]+)?$/
    if is_float(token) {
        return match token.parse::<f64>() {
            Ok(f) => Ok(Value::Float(f)),
            Err(_) => Err(ShonError(format!("Invalid number: {}", token))),
        };
    }

    // Try pure exponent form: /^-?[0-9]+[eE][+-]?[0-9]+$/
    if is_exponent_only(token) {
        return match token.parse::<f64>() {
            Ok(f) => Ok(Value::Float(f)),
            Err(_) => Err(ShonError(format!("Invalid number: {}", token))),
        };
    }

    // Everything else is a string
    Ok(Value::String(token.to_string()))
}

/// Check if a token matches the integer pattern: /^-?[0-9]+$/
fn is_integer(token: &str) -> bool {
    let bytes = token.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let start = if bytes[0] == b'-' { 1 } else { 0 };
    if start >= bytes.len() {
        return false;
    }
    bytes[start..].iter().all(|b| b.is_ascii_digit())
}

/// Check if a token matches the float pattern: /^-?[0-9]*\.[0-9]*([eE][+-]?[0-9]+)?$/
fn is_float(token: &str) -> bool {
    let bytes = token.as_bytes();
    if bytes.is_empty() {
        return false;
    }

    let mut i = 0;

    // Optional leading minus
    if i < bytes.len() && bytes[i] == b'-' {
        i += 1;
    }

    // Digits before dot
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }

    // Must have a dot for this pattern
    if i >= bytes.len() || bytes[i] != b'.' {
        return false;
    }
    i += 1; // skip dot

    // Digits after dot
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }

    // Optional exponent
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        if i >= bytes.len() || !bytes[i].is_ascii_digit() {
            return false;
        }
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }

    i == bytes.len()
}

/// Check if a token matches exponent-only float: /^-?[0-9]+[eE][+-]?[0-9]+$/
fn is_exponent_only(token: &str) -> bool {
    let bytes = token.as_bytes();
    if bytes.is_empty() {
        return false;
    }

    let mut i = 0;

    // Optional leading minus
    if i < bytes.len() && bytes[i] == b'-' {
        i += 1;
    }

    // At least one digit before exponent
    let start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == start {
        return false;
    }

    // Must have exponent marker
    if i >= bytes.len() || (bytes[i] != b'e' && bytes[i] != b'E') {
        return false;
    }
    i += 1;

    // Optional sign
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }

    // At least one digit in exponent
    let exp_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == exp_start {
        return false;
    }

    i == bytes.len()
}

/// Parse a hex string token into bytes. Case-insensitive, normalized to lowercase.
fn parse_hex_token(token: &str) -> Result<Vec<u8>, ShonError> {
    if token.is_empty() {
        return Ok(Vec::new());
    }
    if !token.len().is_multiple_of(2) {
        return Err(ShonError(format!(
            "Hex string must have even number of digits, got {}",
            token.len()
        )));
    }
    let lower = token.to_ascii_lowercase();
    let mut bytes = Vec::with_capacity(lower.len() / 2);
    for i in (0..lower.len()).step_by(2) {
        let byte = u8::from_str_radix(&lower[i..i + 2], 16)
            .map_err(|_| ShonError(format!("Invalid hex digits: '{}'", &token[i..i + 2])))?;
        bytes.push(byte);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|s| s.to_string()).collect()
    }

    // ---- Scalars inside brackets ----

    #[test]
    fn test_null() {
        let a = args(&["[", "-n", "]"]);
        let (val, consumed) = parse_shon_bracket(&a).unwrap();
        assert_eq!(val, Value::Array(vec![Value::Null]));
        assert_eq!(consumed, 3);
    }

    #[test]
    fn test_booleans() {
        let a = args(&["[", "-t", "-f", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(
            val,
            Value::Array(vec![Value::Bool(true), Value::Bool(false)])
        );
    }

    #[test]
    fn test_special_floats() {
        let a = args(&["[", "-I", "-i", "-N", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        let arr = val.as_array().unwrap();
        assert_eq!(arr[0], Value::Float(f64::INFINITY));
        assert_eq!(arr[1], Value::Float(f64::NEG_INFINITY));
        assert!(arr[2].as_float().unwrap().is_nan());
    }

    #[test]
    fn test_integers() {
        let a = args(&["[", "42", "-7", "0", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(
            val,
            Value::Array(vec![
                Value::Integer(42.into()),
                Value::Integer((-7).into()),
                Value::Integer(0.into()),
            ])
        );
    }

    #[test]
    fn test_floats() {
        let a = args(&["[", "6.5", ".5", "1.", "-0.0", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        let arr = val.as_array().unwrap();
        assert_eq!(arr[0], Value::Float(6.5));
        assert_eq!(arr[1], Value::Float(0.5));
        assert_eq!(arr[2], Value::Float(1.0));
        let f = arr[3].as_float().unwrap();
        assert!(f == 0.0 && f.is_sign_negative());
    }

    #[test]
    fn test_float_exponent_lowercase() {
        let a = args(&["[", "6.022e23", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(val, Value::Array(vec![Value::Float(6.022e23)]));
    }

    #[test]
    fn test_float_exponent_uppercase() {
        let a = args(&["[", "6.022E23", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(val, Value::Array(vec![Value::Float(6.022e23)]));
    }

    #[test]
    fn test_exponent_only_no_dot() {
        let a = args(&["[", "1e5", "1E5", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(
            val,
            Value::Array(vec![Value::Float(1e5), Value::Float(1e5)])
        );
    }

    #[test]
    fn test_strings() {
        let a = args(&["[", "hello", "world", "localhost:8080", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(
            val,
            Value::Array(vec![
                Value::String("hello".into()),
                Value::String("world".into()),
                Value::String("localhost:8080".into()),
            ])
        );
    }

    // ---- String escaping ----

    #[test]
    fn test_escape_number() {
        let a = args(&["[", "--", "42", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(val, Value::Array(vec![Value::String("42".into())]));
    }

    #[test]
    fn test_escape_flag() {
        let a = args(&["[", "--", "-t", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(val, Value::Array(vec![Value::String("-t".into())]));
    }

    #[test]
    fn test_escape_bracket() {
        let a = args(&["[", "--", "[", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(val, Value::Array(vec![Value::String("[".into())]));
    }

    #[test]
    fn test_escape_double_dash() {
        let a = args(&["[", "--", "--", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(val, Value::Array(vec![Value::String("--".into())]));
    }

    // ---- Hex bytes ----

    #[test]
    fn test_hex_inside_brackets() {
        let a = args(&["[", "-x", "cafe", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(val, Value::Array(vec![Value::Bytes(vec![0xca, 0xfe])]));
    }

    #[test]
    fn test_hex_uppercase() {
        let a = args(&["[", "-x", "CAFE", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(val, Value::Array(vec![Value::Bytes(vec![0xca, 0xfe])]));
    }

    #[test]
    fn test_hex_mixed_case() {
        let a = args(&["[", "-x", "CaFe", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(val, Value::Array(vec![Value::Bytes(vec![0xca, 0xfe])]));
    }

    #[test]
    fn test_hex_empty() {
        let a = args(&["[", "-x", "", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(val, Value::Array(vec![Value::Bytes(vec![])]));
    }

    #[test]
    fn test_hex_root() {
        let a = args(&["-x", "b0b5"]);
        let (val, consumed) = parse_shon_hex(&a).unwrap();
        assert_eq!(val, Value::Bytes(vec![0xb0, 0xb5]));
        assert_eq!(consumed, 2);
    }

    #[test]
    fn test_hex_odd_digits_error() {
        let a = args(&["-x", "abc"]);
        assert!(parse_shon_hex(&a).is_err());
    }

    #[test]
    fn test_hex_invalid_digits_error() {
        let a = args(&["-x", "zzzz"]);
        assert!(parse_shon_hex(&a).is_err());
    }

    #[test]
    fn test_hex_missing_arg_error() {
        let a = args(&["-x"]);
        assert!(parse_shon_hex(&a).is_err());
    }

    // ---- Arrays ----

    #[test]
    fn test_empty_array() {
        let a = args(&["[", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(val, Value::Array(vec![]));
    }

    #[test]
    fn test_empty_array_single_token() {
        let a = args(&["[]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(val, Value::Array(vec![]));
    }

    #[test]
    fn test_nested_arrays() {
        let a = args(&["[", "[", "1", "2", "]", "[", "3", "4", "]", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(
            val,
            Value::Array(vec![
                Value::Array(vec![Value::Integer(1.into()), Value::Integer(2.into())]),
                Value::Array(vec![Value::Integer(3.into()), Value::Integer(4.into())]),
            ])
        );
    }

    // ---- Objects ----

    #[test]
    fn test_simple_object() {
        let a = args(&["[", "--name", "hello", "--count", "42", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        let obj = val.as_object().unwrap();
        assert_eq!(obj.get("name"), Some(&Value::String("hello".into())));
        assert_eq!(obj.get("count"), Some(&Value::Integer(42.into())));
    }

    #[test]
    fn test_empty_object() {
        let a = args(&["[--]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(val, Value::Object(HashMap::new()));
    }

    #[test]
    fn test_object_with_boolean() {
        let a = args(&["[", "--verbose", "-t", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        let obj = val.as_object().unwrap();
        assert_eq!(obj.get("verbose"), Some(&Value::Bool(true)));
    }

    #[test]
    fn test_nested_object_and_array() {
        let a = args(&[
            "[",
            "--servers",
            "[",
            "localhost:8080",
            "localhost:8081",
            "]",
            "--options",
            "[",
            "--verbose",
            "-t",
            "]",
            "]",
        ]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        let obj = val.as_object().unwrap();
        assert_eq!(
            obj.get("servers"),
            Some(&Value::Array(vec![
                Value::String("localhost:8080".into()),
                Value::String("localhost:8081".into()),
            ]))
        );
        let options = obj.get("options").unwrap().as_object().unwrap();
        assert_eq!(options.get("verbose"), Some(&Value::Bool(true)));
    }

    #[test]
    fn test_object_missing_value_error() {
        let a = args(&["[", "--key", "]"]);
        // This should error: --key needs a value, but gets ]
        assert!(parse_shon_bracket(&a).is_err());
    }

    // ---- Bracket errors ----

    #[test]
    fn test_unclosed_bracket() {
        let a = args(&["[", "1", "2"]);
        assert!(parse_shon_bracket(&a).is_err());
    }

    #[test]
    fn test_unmatched_close() {
        let a = args(&["[", "]", "]"]);
        // First ] closes the array, second ] is not consumed — that's fine,
        // parse_shon_bracket only consumes through the matching ].
        let (val, consumed) = parse_shon_bracket(&a).unwrap();
        assert_eq!(val, Value::Array(vec![]));
        assert_eq!(consumed, 2);
    }

    // ---- Consuming flag errors ----

    #[test]
    fn test_escape_at_end_error() {
        let a = args(&["[", "--"]);
        assert!(parse_shon_bracket(&a).is_err());
    }

    #[test]
    fn test_hex_at_end_error() {
        let a = args(&["[", "-x"]);
        assert!(parse_shon_bracket(&a).is_err());
    }

    // ---- file_bytes / file_string errors ----

    #[test]
    fn test_file_bytes_missing_arg() {
        let a = args(&["-b"]);
        assert!(parse_shon_file_bytes(&a).is_err());
    }

    #[test]
    fn test_file_string_missing_arg() {
        let a = args(&["-s"]);
        assert!(parse_shon_file_string(&a).is_err());
    }

    #[test]
    fn test_file_bytes_nonexistent() {
        let a = args(&["-b", "/nonexistent/file/path/abc123"]);
        assert!(parse_shon_file_bytes(&a).is_err());
    }

    #[test]
    fn test_file_string_nonexistent() {
        let a = args(&["-s", "/nonexistent/file/path/abc123"]);
        assert!(parse_shon_file_string(&a).is_err());
    }

    // ---- Mixed content ----

    #[test]
    fn test_array_with_mixed_types() {
        let a = args(&["[", "hello", "42", "3.14", "-t", "-n", "-x", "ff", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        let arr = val.as_array().unwrap();
        assert_eq!(arr[0], Value::String("hello".into()));
        assert_eq!(arr[1], Value::Integer(42.into()));
        assert_eq!(arr[2], Value::Float(3.14));
        assert_eq!(arr[3], Value::Bool(true));
        assert_eq!(arr[4], Value::Null);
        assert_eq!(arr[5], Value::Bytes(vec![0xff]));
    }

    #[test]
    fn test_complex_nested() {
        // yay [ --name hello --values [ 1 2 3 ] --meta [ --active -t ] ]
        let a = args(&[
            "[", "--name", "hello", "--values", "[", "1", "2", "3", "]", "--meta", "[", "--active",
            "-t", "]", "]",
        ]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        let obj = val.as_object().unwrap();
        assert_eq!(obj.get("name"), Some(&Value::String("hello".into())));
        assert_eq!(
            obj.get("values"),
            Some(&Value::Array(vec![
                Value::Integer(1.into()),
                Value::Integer(2.into()),
                Value::Integer(3.into()),
            ]))
        );
        let meta = obj.get("meta").unwrap().as_object().unwrap();
        assert_eq!(meta.get("active"), Some(&Value::Bool(true)));
    }

    // ---- Number disambiguation ----

    #[test]
    fn test_negative_integer_not_flag() {
        // -7 should be integer, not a flag
        let a = args(&["[", "-7", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(val, Value::Array(vec![Value::Integer((-7).into())]));
    }

    #[test]
    fn test_dash_only_is_string() {
        // A bare "-" is a string (not a valid flag)
        let a = args(&["[", "-", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(val, Value::Array(vec![Value::String("-".into())]));
    }

    #[test]
    fn test_unknown_flag_is_string() {
        // -z is not a recognized flag, treat as string
        let a = args(&["[", "-z", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(val, Value::Array(vec![Value::String("-z".into())]));
    }

    #[test]
    fn test_big_integer() {
        let a = args(&["[", "99999999999999999999", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        let arr = val.as_array().unwrap();
        assert_eq!(
            arr[0],
            Value::Integer(BigInt::from_str("99999999999999999999").unwrap())
        );
    }

    // ---- String escaping multiple ----

    #[test]
    fn test_escape_multiple() {
        let a = args(&["[", "--", "42", "--", "-t", "--", "[", "]"]);
        let (val, _) = parse_shon_bracket(&a).unwrap();
        assert_eq!(
            val,
            Value::Array(vec![
                Value::String("42".into()),
                Value::String("-t".into()),
                Value::String("[".into()),
            ])
        );
    }
}
