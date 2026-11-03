//! Tagged values, against the cases of the design
//! (doc/design/tagged-values-g/cases/): each .yay parses to its .json, with
//! a tagged value as {"#tag": tag, "payload": payload} and bytes as "*hex",
//! and each .nay is refused; the MEH formatter normalizes each .yay and
//! .meh case to YAY that reads as the same value.

use libyay::{encode, format_yay, parse, parse_json, Format, Value};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn cases_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../doc/design/tagged-values-g/cases")
}

fn cases(ext: &str) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir(cases_dir())
        .expect("the design's cases")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().map_or(false, |e| e == ext))
        .collect();
    paths.sort();
    paths
}

/// The value as the cases' JSON spells it.
fn desugar(value: Value) -> Value {
    match value {
        Value::Tagged(tag, payload) => {
            let mut obj = HashMap::new();
            obj.insert("#tag".to_string(), Value::String(tag));
            obj.insert("payload".to_string(), desugar(*payload));
            Value::Object(obj)
        }
        Value::Bytes(b) => {
            let hex: String = b.iter().map(|byte| format!("{:02x}", byte)).collect();
            Value::String(format!("*{}", hex))
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(desugar).collect()),
        Value::Object(obj) => {
            Value::Object(obj.into_iter().map(|(k, v)| (k, desugar(v))).collect())
        }
        // JSON's numbers are floats
        Value::Integer(n) => Value::Float(n.to_string().parse().unwrap()),
        // The prototype's JSON spells these as strings or bare words
        Value::Float(f) if f.is_nan() => Value::String("NaN".to_string()),
        Value::Float(f) if f.is_infinite() => {
            Value::String(if f > 0.0 { "Infinity" } else { "-Infinity" }.to_string())
        }
        other => other,
    }
}

fn expected(path: &std::path::Path) -> Value {
    let json = fs::read_to_string(path.with_extension("json")).unwrap();
    let json = json.replace("-Infinity", "\"-Infinity\"");
    desugar(parse_json(&json).unwrap())
}

#[test]
fn test_tagged_cases_through_meh() {
    let mut failures = Vec::new();
    let mut paths = cases("yay");
    paths.extend(cases("meh"));
    for path in &paths {
        let src = fs::read_to_string(path).unwrap();
        let formatted = match format_yay(&src) {
            Ok(f) => f,
            Err(e) => {
                failures.push(format!("{}: MEH refused: {}", path.display(), e));
                continue;
            }
        };
        match parse(&formatted) {
            Ok(v) if desugar(v.clone()) == expected(path) => {}
            other => failures.push(format!(
                "{}: formatted\n{}\nread {:?}",
                path.display(),
                formatted,
                other
            )),
        }
        if format_yay(&formatted).as_deref() != Ok(formatted.as_str()) {
            failures.push(format!("{}: formatting twice moves it", path.display()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn test_meh_normalizes_tag_spacing() {
    let cases = [
        ("a: 'x'(t)\n", "a: 'x' (t)\n"),
        ("a: 1  (a)(b)\n", "a: 1 (a) (b)\n"),
        ("a: [1(x), 2]\n", "a: [1 (x), 2]\n"),
        ("a: 1 (a\\)b)\n", "a: 1 (a\\)b)\n"),
        ("a: 1 (\\u{41}\\/)\n", "a: 1 (A/)\n"),
        ("- (a)  (b)\n  - 1\n", "- (a) (b)\n  - 1\n"),
        ("a:  (t)\n  b: 1\n", "a: (t)\n  b: 1\n"),
        ("(p)\nx: 1\n", "(p)\nx: 1\n"),
    ];
    for (meh, yay) in cases {
        assert_eq!(format_yay(meh).unwrap(), yay, "from {:?}", meh);
    }
    for refused in ["(t)\n", "a: (t)\nb: 1\n", "a: (t) 1\n", "- a: (t)\n- 1\n"] {
        assert!(format_yay(refused).is_err(), "{:?} formatted", refused);
    }
}

#[test]
fn test_tagged_cases_accepted() {
    let mut failures = Vec::new();
    let paths = cases("yay");
    assert!(!paths.is_empty());
    for path in &paths {
        let src = fs::read_to_string(path).unwrap();
        let json = fs::read_to_string(path.with_extension("json")).unwrap();
        let json = json.replace("-Infinity", "\"-Infinity\"");
        let expected = parse_json(&json).unwrap();
        match parse(&src) {
            Ok(v) if desugar(v.clone()) == desugar(expected.clone()) => {}
            Ok(v) => failures.push(format!("{}: got {:?}", path.display(), v)),
            Err(e) => failures.push(format!("{}: refused: {}", path.display(), e)),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn test_tagged_cases_refused() {
    let mut failures = Vec::new();
    // MEH's leniency is not YAY's
    let mut paths = cases("nay");
    paths.extend(cases("meh"));
    assert!(!paths.is_empty());
    for path in &paths {
        let src = fs::read_to_string(path).unwrap();
        if let Ok(v) = parse(&src) {
            failures.push(format!("{}: accepted as {:?}", path.display(), v));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn test_tagged_cases_write_and_read_back() {
    let mut failures = Vec::new();
    for path in &cases("yay") {
        let value = parse(&fs::read_to_string(path).unwrap()).unwrap();
        let written = encode(&value, Format::Yay);
        match parse(&written) {
            Ok(v) if desugar(v.clone()) == desugar(value.clone()) => {}
            other => failures.push(format!(
                "{}: wrote\n{}\nread {:?}",
                path.display(),
                written,
                other
            )),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn test_tagged_writer_normal_form() {
    let tagged = |tag: &str, v: Value| Value::Tagged(tag.to_string(), Box::new(v));
    let cases = [
        (tagged("s", Value::from(10i64)), "10 (s)"),
        (tagged("a)b", Value::from(3i64)), "3 (a\\)b)"),
        (
            tagged("back\\slash\t\"", Value::from("x")),
            "\"x\" (back\\\\slash\\t\")",
        ),
        (
            tagged("utc", tagged("datetime", Value::from("t"))),
            "\"t\" (datetime) (utc)",
        ),
        (tagged("", Value::from(vec![Value::from(1i64)])), "[1] ()"),
    ];
    for (value, expected) in cases {
        assert_eq!(encode(&value, Format::Yay), expected);
    }
    let block = tagged(
        "outer",
        tagged(
            "inner",
            Value::from(vec![Value::from(vec![Value::Null]); 6]),
        ),
    );
    assert_eq!(
        encode(&block, Format::Yay),
        "(outer) (inner)\n- [null]\n- [null]\n- [null]\n- [null]\n- [null]\n- [null]"
    );
}
