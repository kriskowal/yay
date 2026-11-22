//! References, over the corpus's reference cases, the files named
//! reference-* in test/yay, test/meh and test/nay: each YAY case reads the
//! same through the MEH formatter and survives YSON and YAY; each MEH case
//! reads only through MEH, to its .yay; each NAY case is refused as YAY, and
//! MEH reads it, if at all, with no reference in it. fixtures.rs reads the
//! same cases against their expected values and messages.

use libyay::{
    encode, format_yay, parse, parse_json, parse_shon, parse_yson, Format, Reference, Value,
};
use std::fs;
use std::path::PathBuf;

fn test_dir(kind: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test")
        .join(kind)
}

/// The corpus's reference cases in test/<kind> with the extension.
fn cases(kind: &str, ext: &str) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir(test_dir(kind))
        .expect("the corpus")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().map_or(false, |e| e == ext))
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("reference-")
        })
        .collect();
    paths.sort();
    paths
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
fn yay_cases_read_the_same_as_meh() {
    let paths = cases("yay", "yay");
    assert!(paths.len() >= 15);
    for path in paths {
        let src = fs::read_to_string(&path).unwrap();
        let want = parse(&src).unwrap_or_else(|e| panic!("{}: {}", path.display(), e));
        let meh = read_meh(&src).unwrap_or_else(|e| panic!("{} as MEH: {}", path.display(), e));
        assert_eq!(meh, want, "{} as MEH", path.display());
    }
}

#[test]
fn yay_cases_round_trip_through_yson_and_yay() {
    for path in cases("yay", "yay") {
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

/// test/meh/<name>.<variant>.meh reads as MEH to test/meh/<name>.yay.
#[test]
fn meh_cases_read_only_as_meh() {
    let paths = cases("meh", "meh");
    assert!(paths.len() >= 2);
    for path in paths {
        let src = fs::read_to_string(&path).unwrap();
        assert!(parse(&src).is_err(), "{} accepted as YAY", path.display());
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        let name = stem.rsplit_once('.').map_or(stem.as_str(), |(n, _)| n);
        let expected = test_dir("meh").join(format!("{}.yay", name));
        let want = parse(&fs::read_to_string(&expected).unwrap())
            .unwrap_or_else(|e| panic!("{}: {}", expected.display(), e));
        let got = read_meh(&src).unwrap_or_else(|e| panic!("{} as MEH: {}", path.display(), e));
        assert_eq!(got, want, "{} as MEH", path.display());
    }
}

#[test]
fn nay_cases_are_refused() {
    let paths = cases("nay", "nay");
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
    assert_eq!(r, Value::reference("https://x").unwrap());
    assert_ne!(r, Value::String("https://x".to_string()));
    assert_ne!(parse("<https://x/>").unwrap(), r);
}

#[test]
fn a_space_after_the_open_angle_is_refused() {
    for src in ["a: < ff>", "a: < https://x>"] {
        let err = parse(src).unwrap_err().to_string();
        assert!(
            err.contains("Unexpected space after \"<\""),
            "{}: {}",
            src,
            err
        );
    }
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
    let r = Value::reference("https://x").unwrap();
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
            Value::reference("https://x").unwrap(),
            Value::Tagged(
                "t".to_string(),
                Box::new(Value::reference("urn:a").unwrap())
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

const NOT_REFERENCES: [&str; 7] = [
    "",
    "not a uri",
    "HTTPS://x",
    "urn:a b",
    "https://a>b",
    "https://x/%zz",
    "./a.yay",
];

#[test]
fn a_reference_is_made_only_from_a_reference_text() {
    for uri in NOT_REFERENCES {
        let err = Reference::new(uri).unwrap_err();
        assert_eq!(err.text(), uri);
        assert!(err.to_string().starts_with("Not a reference: "), "{}", err);
        assert!(Value::reference(uri).is_err(), "{:?}", uri);
        assert!(Reference::try_from(uri).is_err(), "{:?}", uri);
        assert!(Reference::try_from(uri.to_string()).is_err(), "{:?}", uri);
    }
    for uri in [
        "urn:x",
        "https://example.com/a?b=c#d",
        "urn:%C3%A9",
        "urn:é",
    ] {
        let r = Reference::new(uri).unwrap();
        assert_eq!(r.as_str(), uri);
        assert_eq!(parse(&format!("<{}>", uri)).unwrap(), Value::Reference(r));
    }
}

#[test]
fn yson_refuses_a_reference_that_is_not_one() {
    for uri in NOT_REFERENCES {
        let yson = encode(&Value::String(format!("${}", uri)), Format::Json);
        assert!(parse_yson(&yson).is_err(), "{}", yson);
    }
}
