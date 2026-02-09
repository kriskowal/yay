//! YSON parser and encoder.
//!
//! YSON is a JSON dialect that embeds big integers, special float values, and
//! byte arrays in strings using a prefix to multiplex the string behavior,
//! without altering strings unless they have a symbol prefix.
//!
//! YSON is itself a subset of the Endo SmallCaps encoding (<https://endojs.org>).
//!
//! Encoding:
//! - BigInt: `"#12345678901234567890"` (hash prefix + decimal)
//! - Float specials: `"#NaN"`, `"#Infinity"`, `"#-Infinity"` (hash prefix)
//! - Bytes: `"*cafe"` (asterisk prefix + hex)
//! - Escaped strings: `"!*hello"` (exclamation prefix for strings starting with reserved chars)
//!
//! Reserved prefixes (ASCII `!` through `/`) are escaped with `!`.

use crate::Value;
use num_bigint::BigInt;
use std::collections::HashMap;

/// Parse a YSON string into a YAY Value.
pub fn parse_yson(input: &str) -> Result<Value, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("Empty input".to_string());
    }

    let (value, rest) = parse_value(input)?;
    let rest = rest.trim();
    if !rest.is_empty() {
        return Err(format!("Unexpected content after value: {}", rest));
    }
    Ok(value)
}

fn parse_value(input: &str) -> Result<(Value, &str), String> {
    let input = input.trim_start();

    if input.is_empty() {
        return Err("Unexpected end of input".to_string());
    }

    match input.chars().next().unwrap() {
        'n' => parse_null(input),
        't' => parse_true(input),
        'f' => parse_false(input),
        '"' => parse_string(input),
        '[' => parse_array(input),
        '{' => parse_object(input),
        '-' | '0'..='9' => parse_number(input),
        c => Err(format!("Unexpected character: {}", c)),
    }
}

fn parse_null(input: &str) -> Result<(Value, &str), String> {
    if let Some(rest) = input.strip_prefix("null") {
        Ok((Value::Null, rest))
    } else {
        Err("Expected 'null'".to_string())
    }
}

fn parse_true(input: &str) -> Result<(Value, &str), String> {
    if let Some(rest) = input.strip_prefix("true") {
        Ok((Value::Bool(true), rest))
    } else {
        Err("Expected 'true'".to_string())
    }
}

fn parse_false(input: &str) -> Result<(Value, &str), String> {
    if let Some(rest) = input.strip_prefix("false") {
        Ok((Value::Bool(false), rest))
    } else {
        Err("Expected 'false'".to_string())
    }
}

fn parse_string(input: &str) -> Result<(Value, &str), String> {
    let (s, rest) = parse_json_string(input)?;

    // Check for YSON extensions
    if let Some(first) = s.chars().next() {
        match first {
            '#' => {
                let payload = &s[1..];
                // Special float values
                match payload {
                    "NaN" => return Ok((Value::Float(f64::NAN), rest)),
                    "Infinity" => return Ok((Value::Float(f64::INFINITY), rest)),
                    "-Infinity" => return Ok((Value::Float(f64::NEG_INFINITY), rest)),
                    _ => {}
                }
                // BigInt
                match payload.parse::<BigInt>() {
                    Ok(n) => return Ok((Value::Integer(n), rest)),
                    Err(e) => return Err(format!("Invalid bigint: {}", e)),
                }
            }
            '*' => {
                // Bytes (hex)
                let hex = &s[1..];
                match parse_hex(hex) {
                    Ok(bytes) => return Ok((Value::Bytes(bytes), rest)),
                    Err(e) => return Err(format!("Invalid hex: {}", e)),
                }
            }
            '!' => {
                // Escaped string - remove the escape prefix
                return Ok((Value::String(s[1..].to_string()), rest));
            }
            _ => {}
        }
    }

    Ok((Value::String(s), rest))
}

fn parse_json_string(input: &str) -> Result<(String, &str), String> {
    if !input.starts_with('"') {
        return Err("Expected '\"'".to_string());
    }

    let mut result = String::new();
    let mut chars = input[1..].chars().peekable();
    let mut consumed = 1; // Opening quote

    loop {
        match chars.next() {
            None => return Err("Unterminated string".to_string()),
            Some('"') => {
                consumed += 1;
                break;
            }
            Some('\\') => {
                consumed += 1;
                match chars.next() {
                    None => return Err("Unterminated escape sequence".to_string()),
                    Some('"') => {
                        result.push('"');
                        consumed += 1;
                    }
                    Some('\\') => {
                        result.push('\\');
                        consumed += 1;
                    }
                    Some('/') => {
                        result.push('/');
                        consumed += 1;
                    }
                    Some('b') => {
                        result.push('\x08');
                        consumed += 1;
                    }
                    Some('f') => {
                        result.push('\x0c');
                        consumed += 1;
                    }
                    Some('n') => {
                        result.push('\n');
                        consumed += 1;
                    }
                    Some('r') => {
                        result.push('\r');
                        consumed += 1;
                    }
                    Some('t') => {
                        result.push('\t');
                        consumed += 1;
                    }
                    Some('u') => {
                        consumed += 1;
                        let mut hex = String::new();
                        for _ in 0..4 {
                            match chars.next() {
                                Some(c) if c.is_ascii_hexdigit() => {
                                    hex.push(c);
                                    consumed += 1;
                                }
                                _ => return Err("Invalid unicode escape".to_string()),
                            }
                        }
                        let code =
                            u32::from_str_radix(&hex, 16).map_err(|_| "Invalid unicode escape")?;
                        if let Some(c) = char::from_u32(code) {
                            result.push(c);
                        } else {
                            return Err("Invalid unicode code point".to_string());
                        }
                    }
                    Some(c) => return Err(format!("Invalid escape: \\{}", c)),
                }
            }
            Some(c) => {
                result.push(c);
                consumed += c.len_utf8();
            }
        }
    }

    Ok((result, &input[consumed..]))
}

fn parse_hex(hex: &str) -> Result<Vec<u8>, String> {
    if !hex.len().is_multiple_of(2) {
        return Err("Odd number of hex digits".to_string());
    }

    let mut bytes = Vec::new();
    let mut chars = hex.chars();

    while let (Some(h), Some(l)) = (chars.next(), chars.next()) {
        let high = h.to_digit(16).ok_or("Invalid hex digit")?;
        let low = l.to_digit(16).ok_or("Invalid hex digit")?;
        bytes.push((high * 16 + low) as u8);
    }

    Ok(bytes)
}

fn parse_number(input: &str) -> Result<(Value, &str), String> {
    let mut end = 0;
    let chars: Vec<char> = input.chars().collect();

    // Optional minus
    if end < chars.len() && chars[end] == '-' {
        end += 1;
    }

    // Integer part
    if end < chars.len() && chars[end] == '0' {
        end += 1;
    } else {
        while end < chars.len() && chars[end].is_ascii_digit() {
            end += 1;
        }
    }

    // Fractional part
    if end < chars.len() && chars[end] == '.' {
        end += 1;
        while end < chars.len() && chars[end].is_ascii_digit() {
            end += 1;
        }
    }

    // Exponent
    if end < chars.len() && (chars[end] == 'e' || chars[end] == 'E') {
        end += 1;
        if end < chars.len() && (chars[end] == '+' || chars[end] == '-') {
            end += 1;
        }
        while end < chars.len() && chars[end].is_ascii_digit() {
            end += 1;
        }
    }

    let num_str: String = chars[..end].iter().collect();
    let rest = &input[num_str.len()..];

    // YSON is a JSON dialect - all JSON numbers are float64.
    // Big integers are represented as strings with a # prefix.
    let f: f64 = num_str.parse().map_err(|_| "Invalid number")?;
    Ok((Value::Float(f), rest))
}

fn parse_array(input: &str) -> Result<(Value, &str), String> {
    if !input.starts_with('[') {
        return Err("Expected '['".to_string());
    }

    let mut rest = input[1..].trim_start();
    let mut items = Vec::new();

    if let Some(stripped) = rest.strip_prefix(']') {
        return Ok((Value::Array(items), stripped));
    }

    loop {
        let (value, new_rest) = parse_value(rest)?;
        items.push(value);
        rest = new_rest.trim_start();

        if let Some(stripped) = rest.strip_prefix(']') {
            return Ok((Value::Array(items), stripped));
        } else if rest.starts_with(',') {
            rest = rest[1..].trim_start();
        } else {
            return Err("Expected ',' or ']'".to_string());
        }
    }
}

fn parse_object(input: &str) -> Result<(Value, &str), String> {
    if !input.starts_with('{') {
        return Err("Expected '{'".to_string());
    }

    let mut rest = input[1..].trim_start();
    let mut obj = HashMap::new();

    if let Some(stripped) = rest.strip_prefix('}') {
        return Ok((Value::Object(obj), stripped));
    }

    loop {
        // Parse key
        if !rest.starts_with('"') {
            return Err("Expected string key".to_string());
        }
        let (key, new_rest) = parse_json_string(rest)?;
        rest = new_rest.trim_start();

        // Expect colon
        if !rest.starts_with(':') {
            return Err("Expected ':'".to_string());
        }
        rest = rest[1..].trim_start();

        // Parse value
        let (value, new_rest) = parse_value(rest)?;
        obj.insert(key, value);
        rest = new_rest.trim_start();

        if let Some(stripped) = rest.strip_prefix('}') {
            return Ok((Value::Object(obj), stripped));
        } else if rest.starts_with(',') {
            rest = rest[1..].trim_start();
        } else {
            return Err("Expected ',' or '}'".to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_null() {
        assert_eq!(parse_yson("null").unwrap(), Value::Null);
    }

    #[test]
    fn test_parse_bool() {
        assert_eq!(parse_yson("true").unwrap(), Value::Bool(true));
        assert_eq!(parse_yson("false").unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_parse_number() {
        // YSON is a JSON dialect - all JSON numbers are float64
        assert_eq!(parse_yson("42").unwrap(), Value::Float(42.0));
        assert_eq!(parse_yson("-10").unwrap(), Value::Float(-10.0));
        assert_eq!(parse_yson("3.14").unwrap(), Value::Float(3.14));
        assert_eq!(parse_yson("-1.5e10").unwrap(), Value::Float(-1.5e10));
    }

    #[test]
    fn test_parse_string() {
        assert_eq!(
            parse_yson("\"hello\"").unwrap(),
            Value::String("hello".into())
        );
        assert_eq!(
            parse_yson("\"a\\nb\"").unwrap(),
            Value::String("a\nb".into())
        );
    }

    #[test]
    fn test_parse_bigint() {
        let result = parse_yson("\"#12345678901234567890\"").unwrap();
        assert_eq!(
            result,
            Value::Integer("12345678901234567890".parse().unwrap())
        );
    }

    #[test]
    fn test_parse_bytes() {
        let result = parse_yson("\"*cafe\"").unwrap();
        assert_eq!(result, Value::Bytes(vec![0xca, 0xfe]));
    }

    #[test]
    fn test_parse_float_nan() {
        let result = parse_yson("\"#NaN\"").unwrap();
        assert!(matches!(result, Value::Float(f) if f.is_nan()));
    }

    #[test]
    fn test_parse_float_infinity() {
        let result = parse_yson("\"#Infinity\"").unwrap();
        assert_eq!(result, Value::Float(f64::INFINITY));
    }

    #[test]
    fn test_parse_float_neg_infinity() {
        let result = parse_yson("\"#-Infinity\"").unwrap();
        assert_eq!(result, Value::Float(f64::NEG_INFINITY));
    }

    #[test]
    fn test_parse_escaped_string() {
        let result = parse_yson("\"!*hello\"").unwrap();
        assert_eq!(result, Value::String("*hello".into()));
    }

    #[test]
    fn test_parse_array() {
        let result = parse_yson("[1, 2, 3]").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 3);
    }

    #[test]
    fn test_parse_object() {
        let result = parse_yson("{\"a\": 1, \"b\": 2}").unwrap();
        let obj = result.as_object().unwrap();
        assert_eq!(obj.len(), 2);
    }

    #[test]
    fn test_roundtrip() {
        use crate::encode::{encode, Format};

        let original = Value::Object(HashMap::from([
            ("int".to_string(), Value::Integer(42.into())),
            (
                "bigint".to_string(),
                Value::Integer("12345678901234567890".parse().unwrap()),
            ),
            ("bytes".to_string(), Value::Bytes(vec![0xca, 0xfe])),
            ("string".to_string(), Value::String("hello".into())),
            ("escaped".to_string(), Value::String("*world".into())),
        ]));

        let yson = encode(&original, Format::Yson);
        let parsed = parse_yson(&yson).unwrap();

        // Compare values
        let orig_obj = original.as_object().unwrap();
        let parsed_obj = parsed.as_object().unwrap();

        assert_eq!(orig_obj.len(), parsed_obj.len());
        for (k, v) in orig_obj {
            assert_eq!(parsed_obj.get(k), Some(v));
        }
    }

    #[test]
    fn test_parse_empty_input() {
        assert!(parse_yson("").is_err());
        assert!(parse_yson("   ").is_err());
    }

    #[test]
    fn test_parse_extra_content() {
        assert!(parse_yson("null extra").is_err());
    }

    #[test]
    fn test_parse_unexpected_char() {
        assert!(parse_yson("@invalid").is_err());
    }

    #[test]
    fn test_parse_invalid_null() {
        assert!(parse_yson("nul").is_err());
    }

    #[test]
    fn test_parse_invalid_true() {
        assert!(parse_yson("tru").is_err());
    }

    #[test]
    fn test_parse_invalid_false() {
        assert!(parse_yson("fals").is_err());
    }

    #[test]
    fn test_parse_invalid_bigint() {
        assert!(parse_yson("\"#notanumber\"").is_err());
    }

    #[test]
    fn test_parse_invalid_hex() {
        assert!(parse_yson("\"*xyz\"").is_err());
        assert!(parse_yson("\"*abc\"").is_err()); // Odd number of hex digits
    }

    #[test]
    fn test_parse_unterminated_string() {
        assert!(parse_yson("\"unterminated").is_err());
    }

    #[test]
    fn test_parse_invalid_escape() {
        assert!(parse_yson("\"\\x\"").is_err());
    }

    #[test]
    fn test_parse_invalid_unicode_escape() {
        assert!(parse_yson("\"\\uXXXX\"").is_err());
    }

    #[test]
    fn test_parse_empty_array() {
        let result = parse_yson("[]").unwrap();
        assert_eq!(result.as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_parse_empty_object() {
        let result = parse_yson("{}").unwrap();
        assert_eq!(result.as_object().unwrap().len(), 0);
    }

    #[test]
    fn test_parse_array_missing_bracket() {
        assert!(parse_yson("[1, 2").is_err());
    }

    #[test]
    fn test_parse_object_missing_brace() {
        assert!(parse_yson("{\"a\": 1").is_err());
    }

    #[test]
    fn test_parse_object_missing_colon() {
        assert!(parse_yson("{\"a\" 1}").is_err());
    }

    #[test]
    fn test_parse_object_non_string_key() {
        assert!(parse_yson("{1: 2}").is_err());
    }

    #[test]
    fn test_parse_number_with_exponent() {
        let result = parse_yson("1e10").unwrap();
        assert_eq!(result.as_float().unwrap(), 1e10);

        let result = parse_yson("1E+10").unwrap();
        assert_eq!(result.as_float().unwrap(), 1e10);

        let result = parse_yson("1e-10").unwrap();
        assert_eq!(result.as_float().unwrap(), 1e-10);
    }

    #[test]
    fn test_parse_number_zero() {
        let result = parse_yson("0").unwrap();
        assert_eq!(result.as_float().unwrap(), 0.0);

        let result = parse_yson("0.5").unwrap();
        assert_eq!(result.as_float().unwrap(), 0.5);
    }

    #[test]
    fn test_parse_string_with_escapes() {
        let result = parse_yson("\"a\\\"b\\\\c\\/d\"").unwrap();
        assert_eq!(result.as_str().unwrap(), "a\"b\\c/d");

        let result = parse_yson("\"\\b\\f\\r\\t\"").unwrap();
        assert_eq!(result.as_str().unwrap(), "\x08\x0c\r\t");
    }

    #[test]
    fn test_parse_string_with_unicode() {
        let result = parse_yson("\"\\u0041\"").unwrap();
        assert_eq!(result.as_str().unwrap(), "A");
    }

    #[test]
    fn test_parse_nested_structures() {
        let result = parse_yson("[[1, 2], {\"a\": [3]}]").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }
}
