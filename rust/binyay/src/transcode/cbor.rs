//! CBOR transcoding: convert between YAY values and CBOR binary data.
//!
//! Mapping from CBOR to YAY:
//!   - CBOR null                  -> Value::Null
//!   - CBOR bool                  -> Value::Bool
//!   - CBOR unsigned/negative int -> Value::Integer (BigInt)
//!   - CBOR float (16/32/64)      -> Value::Float (promoted to f64)
//!   - CBOR text string           -> Value::String
//!   - CBOR byte string           -> Value::Bytes
//!   - CBOR array (det/indet)     -> Value::Array
//!   - CBOR map (det/indet)       -> Value::Object (text string keys only)
//!   - CBOR tag                   -> error (no YAY equivalent)
//!   - CBOR undefined             -> error (no YAY equivalent)
//!   - Any other CBOR value       -> error
//!
//! Mapping from YAY to CBOR:
//!   - Value::Null    -> CBOR null (simple value 22)
//!   - Value::Bool    -> CBOR bool (simple values 20/21)
//!   - Value::Integer -> CBOR integer (smallest encoding that fits)
//!   - Value::Float   -> CBOR float64 (always 9 bytes, never downgraded)
//!   - Value::String  -> CBOR text string (determinate length)
//!   - Value::Bytes   -> CBOR byte string (determinate length)
//!   - Value::Array   -> CBOR array (determinate length)
//!   - Value::Object  -> CBOR map (determinate length, text string keys)
//!
//! Integers that exceed CBOR's native integer range (-2^64 to 2^64-1)
//! produce an error rather than using bignum tags.

use ciborium::value::Value as CborValue;
use libyay::Value;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;

// ---------------------------------------------------------------------------
// Decode (CBOR -> YAY)
// ---------------------------------------------------------------------------

/// Decode CBOR bytes into a YAY Value.
pub fn decode(input: &[u8]) -> Result<Value, String> {
    let cbor_value: CborValue =
        ciborium::de::from_reader(input).map_err(|e| format!("CBOR decode error: {}", e))?;
    cbor_to_value(&cbor_value)
}

fn cbor_to_value(cbor: &CborValue) -> Result<Value, String> {
    match cbor {
        CborValue::Null => Ok(Value::Null),
        CborValue::Bool(b) => Ok(Value::Bool(*b)),
        CborValue::Integer(i) => {
            let n: i128 = (*i).into();
            Ok(Value::Integer(BigInt::from(n)))
        }
        CborValue::Float(f) => Ok(Value::Float(*f)),
        CborValue::Text(s) => Ok(Value::String(s.clone())),
        CborValue::Bytes(b) => Ok(Value::Bytes(b.clone())),
        CborValue::Array(arr) => {
            let items: Result<Vec<Value>, String> = arr.iter().map(cbor_to_value).collect();
            Ok(Value::Array(items?))
        }
        CborValue::Map(pairs) => {
            let mut obj = HashMap::new();
            for (k, v) in pairs {
                let key = match k {
                    CborValue::Text(s) => s.clone(),
                    _ => return Err(format!("CBOR map key must be a text string, got: {:?}", k)),
                };
                obj.insert(key, cbor_to_value(v)?);
            }
            Ok(Value::Object(obj))
        }
        CborValue::Tag(tag, _) => Err(format!(
            "CBOR tagged value (tag {}) has no YAY equivalent",
            tag
        )),
        _ => Err(format!("CBOR value {:?} has no YAY equivalent", cbor)),
    }
}

// ---------------------------------------------------------------------------
// Encode (YAY -> CBOR)
//
// We write CBOR directly rather than going through ciborium's Value type
// because ciborium unconditionally downgrades float64 to float16/float32
// when the value is representable in fewer bytes. The YAY-to-CBOR contract
// requires that all float64 values remain encoded as CBOR float64 (major
// type 7, additional info 27, 8-byte IEEE 754 payload).
// ---------------------------------------------------------------------------

/// Encode a YAY Value as CBOR bytes.
pub fn encode(value: &Value) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    write_value(&mut buf, value)?;
    Ok(buf)
}

fn write_value(buf: &mut Vec<u8>, value: &Value) -> Result<(), String> {
    match value {
        Value::Null => {
            // CBOR simple value 22 = null
            buf.push(0xf6);
            Ok(())
        }
        Value::Bool(b) => {
            // CBOR simple value 20 = false, 21 = true
            buf.push(if *b { 0xf5 } else { 0xf4 });
            Ok(())
        }
        Value::Integer(n) => write_integer(buf, n),
        Value::Float(f) => {
            // Always encode as CBOR float64 (major 7, info 27)
            buf.push(0xfb);
            buf.extend_from_slice(&f.to_be_bytes());
            Ok(())
        }
        Value::String(s) => {
            let bytes = s.as_bytes();
            write_type_and_length(buf, 3, bytes.len() as u64); // major 3 = text string
            buf.extend_from_slice(bytes);
            Ok(())
        }
        Value::Bytes(b) => {
            write_type_and_length(buf, 2, b.len() as u64); // major 2 = byte string
            buf.extend_from_slice(b);
            Ok(())
        }
        Value::Array(arr) => {
            write_type_and_length(buf, 4, arr.len() as u64); // major 4 = array
            for item in arr {
                write_value(buf, item)?;
            }
            Ok(())
        }
        Value::Object(obj) => {
            let mut keys: Vec<&String> = obj.keys().collect();
            keys.sort();
            write_type_and_length(buf, 5, keys.len() as u64); // major 5 = map
            for k in keys {
                // Key: text string
                let key_bytes = k.as_bytes();
                write_type_and_length(buf, 3, key_bytes.len() as u64);
                buf.extend_from_slice(key_bytes);
                // Value
                write_value(buf, &obj[k])?;
            }
            Ok(())
        }
    }
}

/// Write a CBOR major type + length/value argument.
///
/// CBOR encodes the major type in the high 3 bits and uses the low 5 bits
/// plus optional following bytes for the argument:
///   0-23:    argument in the low 5 bits (1 byte total)
///   24:      1-byte argument follows
///   25:      2-byte argument follows
///   26:      4-byte argument follows
///   27:      8-byte argument follows
fn write_type_and_length(buf: &mut Vec<u8>, major: u8, val: u64) {
    let high = major << 5;
    match val {
        0..=23 => {
            buf.push(high | val as u8);
        }
        24..=0xff => {
            buf.push(high | 24);
            buf.push(val as u8);
        }
        0x100..=0xffff => {
            buf.push(high | 25);
            buf.extend_from_slice(&(val as u16).to_be_bytes());
        }
        0x10000..=0xffff_ffff => {
            buf.push(high | 26);
            buf.extend_from_slice(&(val as u32).to_be_bytes());
        }
        _ => {
            buf.push(high | 27);
            buf.extend_from_slice(&val.to_be_bytes());
        }
    }
}

/// Write a YAY integer as the smallest CBOR integer encoding.
///
/// CBOR has two integer major types:
///   - Major 0 (positive): encodes value n directly (represents n)
///   - Major 1 (negative): encodes value n (represents -1 - n)
///
/// The argument uses the smallest encoding that fits.
fn write_integer(buf: &mut Vec<u8>, n: &BigInt) -> Result<(), String> {
    if n.sign() == num_bigint::Sign::Minus {
        // Negative: CBOR major 1 encodes -1 - n, so the argument is |n| - 1
        let abs_minus_1 = (-n) - BigInt::from(1);
        let val = abs_minus_1.to_u64().ok_or_else(|| {
            format!(
                "integer {} exceeds CBOR's native integer range (-2^64 to 2^64-1)",
                n
            )
        })?;
        write_type_and_length(buf, 1, val);
    } else {
        let val = n.to_u64().ok_or_else(|| {
            format!(
                "integer {} exceeds CBOR's native integer range (-2^64 to 2^64-1)",
                n
            )
        })?;
        write_type_and_length(buf, 0, val);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Diagnostic Notation (CBOR -> human-readable text, RFC 8949 §8)
// ---------------------------------------------------------------------------

/// Render CBOR bytes as diagnostic notation (RFC 8949 §8).
///
/// This is a presentation format for reviewing CBOR data in a human-readable
/// form. It renders from the CBOR binary, not from YAY values, so it
/// faithfully represents the actual wire encoding.
pub fn diagnostic(input: &[u8]) -> Result<String, String> {
    let cbor_value: CborValue =
        ciborium::de::from_reader(input).map_err(|e| format!("CBOR decode error: {}", e))?;
    let mut out = String::new();
    diag_value(&mut out, &cbor_value, 0);
    out.push('\n');
    Ok(out)
}

fn diag_value(out: &mut String, val: &CborValue, indent: usize) {
    match val {
        CborValue::Null => out.push_str("null"),
        CborValue::Bool(true) => out.push_str("true"),
        CborValue::Bool(false) => out.push_str("false"),
        CborValue::Integer(i) => {
            let n: i128 = (*i).into();
            write!(out, "{}", n).unwrap();
        }
        CborValue::Float(f) => {
            diag_float(out, *f);
        }
        CborValue::Text(s) => {
            diag_text(out, s);
        }
        CborValue::Bytes(b) => {
            out.push_str("h'");
            for byte in b {
                write!(out, "{:02x}", byte).unwrap();
            }
            out.push('\'');
        }
        CborValue::Array(arr) => {
            diag_array(out, arr, indent);
        }
        CborValue::Map(pairs) => {
            diag_map(out, pairs, indent);
        }
        CborValue::Tag(tag, inner) => {
            write!(out, "{}(", tag).unwrap();
            diag_value(out, inner, indent);
            out.push(')');
        }
        _ => {
            write!(out, "<?unknown {:?}>", val).unwrap();
        }
    }
}

fn diag_float(out: &mut String, f: f64) {
    if f.is_nan() {
        out.push_str("NaN");
    } else if f.is_infinite() {
        if f.is_sign_positive() {
            out.push_str("Infinity");
        } else {
            out.push_str("-Infinity");
        }
    } else if f == 0.0 && f.is_sign_negative() {
        out.push_str("-0.0");
    } else if f.fract() == 0.0 && f.abs() < 1e18 {
        // Print as integer-like float with .0 suffix
        write!(out, "{:.1}", f).unwrap();
    } else {
        // Use full precision
        let s = format!("{}", f);
        out.push_str(&s);
        // Ensure there's a decimal point
        if !s.contains('.') && !s.contains('e') && !s.contains('E') {
            out.push_str(".0");
        }
    }
}

fn diag_text(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                write!(out, "\\u{:04x}", c as u32).unwrap();
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn diag_array(out: &mut String, arr: &[CborValue], indent: usize) {
    if arr.is_empty() {
        out.push_str("[]");
        return;
    }
    // Use compact form for small arrays of simple values
    if arr.len() <= 5 && arr.iter().all(is_simple_value) {
        out.push('[');
        for (i, item) in arr.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            diag_value(out, item, indent);
        }
        out.push(']');
    } else {
        out.push_str("[\n");
        let child_indent = indent + 2;
        for (i, item) in arr.iter().enumerate() {
            for _ in 0..child_indent {
                out.push(' ');
            }
            diag_value(out, item, child_indent);
            if i < arr.len() - 1 {
                out.push(',');
            }
            out.push('\n');
        }
        for _ in 0..indent {
            out.push(' ');
        }
        out.push(']');
    }
}

fn diag_map(out: &mut String, pairs: &[(CborValue, CborValue)], indent: usize) {
    if pairs.is_empty() {
        out.push_str("{}");
        return;
    }
    out.push_str("{\n");
    let child_indent = indent + 2;
    for (i, (k, v)) in pairs.iter().enumerate() {
        for _ in 0..child_indent {
            out.push(' ');
        }
        diag_value(out, k, child_indent);
        out.push_str(": ");
        diag_value(out, v, child_indent);
        if i < pairs.len() - 1 {
            out.push(',');
        }
        out.push('\n');
    }
    for _ in 0..indent {
        out.push(' ');
    }
    out.push('}');
}

fn is_simple_value(val: &CborValue) -> bool {
    matches!(
        val,
        CborValue::Null
            | CborValue::Bool(_)
            | CborValue::Integer(_)
            | CborValue::Float(_)
            | CborValue::Text(_)
            | CborValue::Bytes(_)
    )
}
