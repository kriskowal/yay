//! Tagged values, over the corpus's tagged cases: the MEH formatter
//! normalizes each test/yay/tagged-*.yay and test/meh/tagged-*.meh to YAY
//! that reads as the same value as its YAY twin, YAY refuses each
//! test/meh/tagged-*.meh, and each test/yay/tagged-*.yay survives the writer.
//! The corpus test reads the tagged cases in test/yay and test/nay against
//! their expected values and messages with the rest of the corpus.

use libyay::{encode, format_yay, parse, Format, Value};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// The corpus's tagged cases in test/<kind> ending in .<ext>.
fn cases(kind: &str, ext: &str) -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test")
        .join(kind);
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("the corpus")
        .map(|e| e.unwrap().path())
        .filter(|p| {
            let name = p.file_name().unwrap().to_string_lossy();
            name.starts_with("tagged-") && name.ends_with(&format!(".{}", ext))
        })
        .collect();
    paths.sort();
    paths
}

/// The YAY document whose value a case reads as: itself for a .yay, and
/// test/meh/<name>.yay for test/meh/<name>.loose.meh.
fn twin(path: &std::path::Path) -> PathBuf {
    let name = path.file_name().unwrap().to_string_lossy();
    match name.strip_suffix(".loose.meh") {
        Some(stem) => path.with_file_name(format!("{}.yay", stem)),
        None => path.to_path_buf(),
    }
}

/// The value with tags and bytes spelled out and NaN as a string, so that
/// two readings compare equal when they mean the same.
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

#[test]
fn test_tagged_cases_through_meh() {
    let mut failures = Vec::new();
    let mut paths = cases("yay", "yay");
    paths.extend(cases("meh", "meh"));
    assert!(paths.len() >= 30, "the corpus's tagged cases");
    for path in &paths {
        let src = fs::read_to_string(path).unwrap();
        let expected = desugar(parse(&fs::read_to_string(twin(path)).unwrap()).unwrap());
        let formatted = match format_yay(&src) {
            Ok(f) => f,
            Err(e) => {
                failures.push(format!("{}: MEH refused: {}", path.display(), e));
                continue;
            }
        };
        match parse(&formatted) {
            Ok(v) if desugar(v.clone()) == expected => {}
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
fn test_tagged_meh_cases_refused() {
    // MEH's leniency is not YAY's
    let mut failures = Vec::new();
    let paths = cases("meh", "meh");
    assert!(paths.len() >= 3, "the corpus's MEH tagged cases");
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
    let paths = cases("yay", "yay");
    assert!(paths.len() >= 25, "the corpus's tagged cases");
    for path in &paths {
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

#[test]
fn test_yson_tagged_round_trip() {
    use libyay::parse_yson;
    for path in &cases("yay", "yay") {
        let value = parse(&fs::read_to_string(path).unwrap()).unwrap();
        let yson = encode(&value, Format::Yson);
        let back = parse_yson(&yson).unwrap();
        assert_eq!(
            desugar(back),
            desugar(value),
            "{}: {}",
            path.display(),
            yson
        );
    }
    let tagged = |tag: &str, v: Value| Value::Tagged(tag.to_string(), Box::new(v));
    assert_eq!(
        encode(&tagged("s", Value::from(10i64)), Format::Yson),
        "{\n  \"#tag\": \"s\",\n  \"payload\": \"+10\"\n}"
    );
    for tag in ["", "a)b", "!x", "#y", "+z"] {
        let value = tagged(tag, Value::from(-7i64));
        let yson = encode(&value, Format::Yson);
        assert_eq!(parse_yson(&yson).unwrap(), value, "{}", yson);
    }
    assert!(encode(&tagged("!x", Value::Null), Format::Yson).contains("\"!!x\""));
}

#[test]
fn test_yson_key_escape() {
    use libyay::parse_yson;
    let mut obj = HashMap::new();
    for k in ["#tag", "#x", "*y", "!z", "payload", "-w"] {
        obj.insert(k.to_string(), Value::from(1i64));
    }
    let value = Value::Object(obj);
    let yson = encode(&value, Format::Yson);
    for k in [
        "\"!#tag\"",
        "\"!#x\"",
        "\"!*y\"",
        "\"!!z\"",
        "\"payload\"",
        "\"-w\"",
    ] {
        assert!(yson.contains(k), "{} in {}", k, yson);
    }
    assert_eq!(parse_yson(&yson).unwrap(), value);
    assert!(parse_yson("{\"#x\": 1}").is_err());
    assert!(parse_yson("\"#42\"").is_err());
    assert_eq!(parse_yson("\"-42\"").unwrap(), Value::from(-42i64));
}
