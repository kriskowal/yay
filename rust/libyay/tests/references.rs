//! References, against the cases of the design
//! (doc/design/references/cases/): each .yay reads to its .yson, as YAY and
//! through the MEH formatter; each .meh only through MEH; each .nay is
//! refused as YAY, and MEH reads it, if at all, with no reference in it, as
//! 3.0.0 did.

use libyay::{encode, format_yay, parse, parse_json, parse_shon, parse_yson, Format, Value};
use std::fs;
use std::path::{Path, PathBuf};

fn cases_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../doc/design/references/cases")
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

fn expected(path: &Path) -> Value {
    parse_yson(&fs::read_to_string(path.with_extension("yson")).unwrap()).unwrap()
}

fn read_meh(src: &str) -> Result<Value, String> {
    let yay = format_yay(src)?;
    parse(&yay).map_err(|e| e.to_string())
}

fn has_reference(value: &Value) -> bool {
    match value {
        Value::Reference(_) => true,
        Value::Tagged(_, payload) => has_reference(payload),
        Value::Array(arr) => arr.iter().any(has_reference),
        Value::Object(obj) => obj.values().any(has_reference),
        _ => false,
    }
}

#[test]
fn yay_cases_read_to_their_yson() {
    let paths = cases("yay");
    assert!(paths.len() >= 15);
    for path in paths {
        let src = fs::read_to_string(&path).unwrap();
        let want = expected(&path);
        let got = parse(&src).unwrap_or_else(|e| panic!("{}: {}", path.display(), e));
        assert_eq!(got, want, "{}", path.display());
        let meh = read_meh(&src).unwrap_or_else(|e| panic!("{} as MEH: {}", path.display(), e));
        assert_eq!(meh, want, "{} as MEH", path.display());
    }
}

#[test]
fn yay_cases_round_trip_through_yson_and_yay() {
    for path in cases("yay") {
        let value = parse(&fs::read_to_string(&path).unwrap()).unwrap();
        let yson = encode(&value, Format::Yson);
        assert_eq!(
            parse_yson(&yson).unwrap(),
            value,
            "{} via YSON",
            path.display()
        );
        let yay = encode(&value, Format::Yay);
        assert_eq!(
            parse(&yay).unwrap(),
            value,
            "{} via YAY:\n{}",
            path.display(),
            yay
        );
    }
}

#[test]
fn meh_cases_read_only_as_meh() {
    let paths = cases("meh");
    assert!(paths.len() >= 2);
    for path in paths {
        let src = fs::read_to_string(&path).unwrap();
        assert!(parse(&src).is_err(), "{} accepted as YAY", path.display());
        let got = read_meh(&src).unwrap_or_else(|e| panic!("{} as MEH: {}", path.display(), e));
        assert_eq!(got, expected(&path), "{} as MEH", path.display());
    }
}

#[test]
fn nay_cases_are_refused() {
    let paths = cases("nay");
    assert!(paths.len() >= 14);
    for path in paths {
        let src = fs::read_to_string(&path).unwrap();
        assert!(parse(&src).is_err(), "{} accepted as YAY", path.display());
        if let Ok(value) = read_meh(&src) {
            assert!(
                !has_reference(&value),
                "{} read by MEH as {:?}",
                path.display(),
                value
            );
        }
    }
}

#[test]
fn a_reference_is_not_its_text() {
    let r = parse("<https://x>").unwrap();
    assert_eq!(r, Value::Reference("https://x".to_string()));
    assert_ne!(r, Value::String("https://x".to_string()));
    assert_ne!(parse("<https://x/>").unwrap(), r);
}

#[test]
fn scheme_is_lowercase_in_yay_and_lowercased_by_meh() {
    let err = parse("a: <HTTPS://x>").unwrap_err().to_string();
    assert!(err.contains("Uppercase scheme \"HTTPS\""), "{}", err);
    assert_eq!(format_yay("a: <HTTPS://x>\n").unwrap(), "a: <https://x>\n");
    // Bytes keep their messages
    assert!(parse("a: <DEADBEEF>")
        .unwrap_err()
        .to_string()
        .contains("Uppercase hex"));
    assert!(parse("a: <deadbeefXX>").is_err());
}

#[test]
fn yson_and_json_spell_a_reference_with_a_dollar() {
    let r = Value::Reference("https://x".to_string());
    assert_eq!(encode(&r, Format::Yson), "\"$https://x\"");
    assert_eq!(encode(&r, Format::Json), "\"$https://x\"");
    assert_eq!(parse_yson("\"$https://x\"").unwrap(), r);
    assert_eq!(
        parse_json("\"$https://x\"").unwrap(),
        Value::String("$https://x".to_string())
    );
    assert!(parse_yson("\"$x\"").is_err());
    assert!(parse_yson("\"$\"").is_err());
    assert!(parse_yson("\"$https://a b\"").is_err());
    // A string that begins with "$" is escaped, never written as a reference
    let s = Value::String("$https://x".to_string());
    assert_eq!(parse_yson(&encode(&s, Format::Yson)).unwrap(), s);
}

#[test]
fn shon_r_takes_a_reference() {
    let args = |s: &str| s.split(' ').map(String::from).collect::<Vec<_>>();
    let (v, _) = parse_shon(&args("[ -r https://x -T t -r urn:a ]")).unwrap();
    assert_eq!(
        v,
        Value::Array(vec![
            Value::Reference("https://x".to_string()),
            Value::Tagged(
                "t".to_string(),
                Box::new(Value::Reference("urn:a".to_string()))
            ),
        ])
    );
    assert!(parse_shon(&args("[ -r ./x ]")).is_err());
    assert!(parse_shon(&args("[ -r ]")).is_err());
    let (v, _) = parse_shon(&args("[ https://x ]")).unwrap();
    assert_eq!(
        v,
        Value::Array(vec![Value::String("https://x".to_string())])
    );
}
