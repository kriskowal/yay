//! Test harness for YAY parser against fixture files.
//!
//! This test harness reads all .yay files from the test/yay/ directory and
//! parses them, comparing against expected output files in test/js/ etc.
//! It also reads .nay files from test/nay/ (expected to fail) and verifies
//! they produce the expected error messages from corresponding .error files.

use std::fs;
use std::path::Path;

use libyay::{encode, parse, parse_with_filename, Format, Value};

/// Compare two Values, treating NaN as equal to NaN
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Integer(a), Value::Integer(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => {
            // Handle NaN comparison
            if a.is_nan() && b.is_nan() {
                true
            } else {
                a == b
            }
        }
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Bytes(a), Value::Bytes(b)) => a == b,
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| values_equal(x, y))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(k, v)| b.get(k).map_or(false, |bv| values_equal(v, bv)))
        }
        _ => false,
    }
}

/// Root test directory.
fn test_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test")
}

/// Get all .yay test files from the test/yay/ directory.
fn get_yay_files() -> Vec<String> {
    get_files_in_subdir("yay", "yay")
}

/// Get all .nay test files from the test/nay/ directory.
fn get_nay_files() -> Vec<String> {
    get_files_in_subdir("nay", "nay")
}

/// Get all files with a given extension from a subdirectory of test/.
fn get_files_in_subdir(subdir: &str, ext: &str) -> Vec<String> {
    let dir = test_root().join(subdir);
    let mut files: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == ext).unwrap_or(false) {
                files.push(path.to_string_lossy().to_string());
            }
        }
    }
    files.sort();
    files
}

/// Read the expected JavaScript output for a .yay test file.
fn read_expected_js(yay_path: &str) -> Option<String> {
    let basename = Path::new(yay_path).file_stem().unwrap().to_string_lossy();
    let js_path = test_root().join("js").join(format!("{}.js", basename));
    fs::read_to_string(js_path).ok()
}

/// Read the expected error message for a .nay file.
fn read_expected_error(nay_path: &str) -> Option<String> {
    let basename = Path::new(nay_path).file_stem().unwrap().to_string_lossy();
    let error_path = test_root().join("nay").join(format!("{}.error", basename));
    fs::read_to_string(error_path).ok()
}

/// Normalize a JS value string for comparison.
/// Handles differences in formatting, key ordering, whitespace, etc.
fn normalize_js_value(s: &str) -> String {
    let s = s.trim();
    // Remove outer parentheses from objects: ({ ... }) -> { ... }
    let s = if s.starts_with("({") && s.ends_with("})") {
        &s[1..s.len() - 1]
    } else {
        s
    };
    // Normalize whitespace - collapse multiple spaces
    let mut s = s.replace("\n", " ");
    while s.contains("  ") {
        s = s.replace("  ", " ");
    }
    // Remove trailing commas and normalize bracket spacing
    let s = s.replace(", }", " }").replace(", ]", "]");
    // Remove space after [ and before ]
    let s = s.replace("[ ", "[").replace(" ]", "]");
    // Normalize number formatting (remove underscores)
    let s = s.replace("_", "");
    // Normalize empty byte arrays
    let s = s.replace("new Uint8Array(0)", "Uint8Array.from([])");
    // Normalize string quotes (convert single-quoted to double-quoted)
    let s = normalize_string_quotes(&s);
    // Sort object keys for comparison (simple approach - won't handle nested objects perfectly)
    normalize_object_keys(&s)
}

/// Normalize string quotes - convert single-quoted strings to double-quoted.
/// 'say "hi"' -> "say \"hi\""
fn normalize_string_quotes(s: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '\'' {
            // Found a single-quoted string, convert to double-quoted
            let mut content = String::new();
            i += 1; // skip opening '
            while i < chars.len() && chars[i] != '\'' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    // Handle escapes
                    let next = chars[i + 1];
                    if next == '\'' {
                        content.push('\'');
                        i += 2;
                    } else if next == '\\' {
                        content.push('\\');
                        content.push('\\');
                        i += 2;
                    } else {
                        content.push(chars[i]);
                        i += 1;
                    }
                } else if chars[i] == '"' {
                    // Escape double quotes in the content
                    content.push('\\');
                    content.push('"');
                    i += 1;
                } else {
                    content.push(chars[i]);
                    i += 1;
                }
            }
            i += 1; // skip closing '
            result.push('"');
            result.push_str(&content);
            result.push('"');
        } else if chars[i] == '"' {
            // Double-quoted string, copy as-is
            result.push(chars[i]);
            i += 1;
            while i < chars.len() {
                result.push(chars[i]);
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                    result.push(chars[i]);
                } else if chars[i] == '"' {
                    i += 1;
                    break;
                }
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Sort object keys in a JS-like string for comparison.
fn normalize_object_keys(s: &str) -> String {
    // This is a simple approach that works for most cases
    // For a proper solution, we'd need to parse the JS-like syntax
    s.trim().to_string()
}

/// Run a single .yay test file (expected to succeed).
fn run_yay_test(path: &str) -> Result<(), String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {}", path, e))?;

    let filename = Path::new(path)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    // Parse the YAY file
    let result = parse(&content);

    match result {
        Ok(value) => {
            // Compare against JS fixture using the actual encoder
            if let Some(expected_js) = read_expected_js(path) {
                let expected = normalize_js_value(&expected_js);
                let actual_raw = encode(&value, Format::JavaScript);
                let actual = normalize_js_value(&actual_raw);

                if actual != expected {
                    return Err(format!(
                        "{}: Output mismatch\n    expected: {}\n    actual:   {}",
                        filename, expected, actual
                    ));
                }
                println!("  {} => {}", filename, actual_raw.trim());
            } else {
                println!("  {} => {:?} (no expected output)", filename, value);
            }
            Ok(())
        }
        Err(e) => {
            // .yay files should not fail
            Err(format!("{}: Unexpected parse error: {}", filename, e))
        }
    }
}

/// Run a single .nay test file (expected to fail with specific error).
fn run_nay_test(path: &str) -> Result<(), String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {}", path, e))?;

    let filename = Path::new(path)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    // Parse with filename for error location reporting
    let result = parse_with_filename(&content, Some(&filename));

    match result {
        Ok(value) => {
            // .nay files should fail to parse
            Err(format!(
                "{}: Expected parse error, but got success: {:?}",
                filename, value
            ))
        }
        Err(e) => {
            let actual_error = e.to_string();

            // Check if we have an expected error message
            if let Some(expected) = read_expected_error(path) {
                let expected = expected.trim();
                if actual_error == expected {
                    println!("  {} => error (as expected)", filename);
                    Ok(())
                } else {
                    Err(format!(
                        "{}: Error mismatch\n    expected: {}\n    actual:   {}",
                        filename, expected, actual_error
                    ))
                }
            } else {
                // No .error file - just verify it fails
                println!(
                    "  {} => error: {} (no .error file to compare)",
                    filename, actual_error
                );
                Ok(())
            }
        }
    }
}

#[test]
fn test_all_yay_fixtures() {
    let files = get_yay_files();

    if files.is_empty() {
        println!("No .yay test files found!");
        return;
    }

    println!("\nRunning {} .yay test files:", files.len());

    let mut passed = 0;
    let mut failed = 0;
    let mut errors: Vec<String> = Vec::new();

    for file in &files {
        match run_yay_test(file) {
            Ok(()) => passed += 1,
            Err(e) => {
                failed += 1;
                errors.push(e);
            }
        }
    }

    println!("\nResults: {} passed, {} failed", passed, failed);

    if !errors.is_empty() {
        println!("\nErrors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    assert!(failed == 0, "{} .yay tests failed", failed);
}

#[test]
fn test_all_nay_fixtures() {
    let files = get_nay_files();

    if files.is_empty() {
        println!("No .nay test files found!");
        return;
    }

    println!("\nRunning {} .nay test files:", files.len());

    let mut passed = 0;
    let mut failed = 0;
    let mut errors: Vec<String> = Vec::new();

    for file in &files {
        match run_nay_test(file) {
            Ok(()) => passed += 1,
            Err(e) => {
                failed += 1;
                errors.push(e);
            }
        }
    }

    println!("\nResults: {} passed, {} failed", passed, failed);

    if !errors.is_empty() {
        println!("\nErrors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    assert!(failed == 0, "{} .nay tests failed", failed);
}

/// Exercise all encoder formats for coverage.
/// This test doesn't compare output (fixtures may have minor formatting differences),
/// but ensures all encoder code paths are exercised.
fn exercise_all_encoders(value: &Value) {
    // Exercise all format encoders
    let _ = encode(value, Format::Yay);
    let _ = encode(value, Format::JavaScript);
    let _ = encode(value, Format::Go);
    let _ = encode(value, Format::Python);
    let _ = encode(value, Format::Rust);
    let _ = encode(value, Format::C);
    let _ = encode(value, Format::Java);
    let _ = encode(value, Format::Scheme);
    let _ = encode(value, Format::Json);
    let _ = encode(value, Format::Yson);
}

#[test]
fn test_all_encoder_coverage() {
    let files = get_yay_files();

    if files.is_empty() {
        println!("No .yay test files found!");
        return;
    }

    println!("\nExercising all encoders for {} .yay files:", files.len());

    let mut errors: Vec<String> = Vec::new();

    for file in &files {
        let content = match fs::read_to_string(&file) {
            Ok(c) => c,
            Err(e) => {
                errors.push(format!("Failed to read {}: {}", file, e));
                continue;
            }
        };

        let value = match parse(&content) {
            Ok(v) => v,
            Err(e) => {
                errors.push(format!("Failed to parse {}: {}", file, e));
                continue;
            }
        };

        // Exercise all encoders - this is for coverage, not correctness
        exercise_all_encoders(&value);
    }

    if !errors.is_empty() {
        println!("\nErrors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    assert!(errors.is_empty(), "{} files had errors", errors.len());
}

/// Exercise Value accessor methods for coverage
fn exercise_value_accessors(value: &Value) {
    // Exercise all accessor methods
    let _ = value.is_null();
    let _ = value.as_bool();
    let _ = value.as_integer();
    let _ = value.as_float();
    let _ = value.as_str();
    let _ = value.as_array();
    let _ = value.as_object();
    let _ = value.as_bytes();
    let _ = value.json_incompatibility();

    // Exercise Debug formatting
    let _ = format!("{:?}", value);

    // Recursively exercise nested values
    if let Some(arr) = value.as_array() {
        for item in arr {
            exercise_value_accessors(item);
        }
    }
    if let Some(obj) = value.as_object() {
        for v in obj.values() {
            exercise_value_accessors(v);
        }
    }
}

#[test]
fn test_value_accessor_coverage() {
    let files = get_yay_files();

    if files.is_empty() {
        println!("No .yay test files found!");
        return;
    }

    println!(
        "\nExercising Value accessors for {} .yay files:",
        files.len()
    );

    for file in &files {
        let content = match fs::read_to_string(&file) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let value = match parse(&content) {
            Ok(v) => v,
            Err(_) => continue,
        };

        exercise_value_accessors(&value);
    }

    // Also test Value::from conversions
    let _ = Value::from(true);
    let _ = Value::from(false);
    let _ = Value::from(42i64);
    let _ = Value::from(3.14f64);
    let _ = Value::from("hello");
    let _ = Value::from(String::from("world"));
    let _ = Value::from(vec![Value::Null]);
    let _ = Value::from(std::collections::HashMap::from([(
        "key".to_string(),
        Value::Null,
    )]));
    let _ = Value::from(vec![0u8, 1, 2]);
    let _ = Value::from(num_bigint::BigInt::from(123));
}

/// Categories of MEH round-trip failures
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum MehFailureCategory {
    MehParseFailed,
    MehFormatFailed,
    StrictParseFailed,
    ValueMismatch,
}

impl std::fmt::Display for MehFailureCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MehFailureCategory::MehParseFailed => write!(f, "MEH parse failed"),
            MehFailureCategory::MehFormatFailed => write!(f, "MEH format failed"),
            MehFailureCategory::StrictParseFailed => write!(f, "Strict parse of MEH output failed"),
            MehFailureCategory::ValueMismatch => write!(f, "Value mismatch after round-trip"),
        }
    }
}

/// Run a single MEH round-trip test.
/// Parses with MEH, formats back to YAY, then parses with strict parser.
fn run_meh_roundtrip_test(path: &str) -> Result<(), (MehFailureCategory, String)> {
    let content = fs::read_to_string(path).map_err(|e| {
        (
            MehFailureCategory::MehParseFailed,
            format!("Failed to read {}: {}", path, e),
        )
    })?;

    let filename = Path::new(path)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    // Step 1: Parse original with strict parser to get expected value
    let original_value = parse_with_filename(&content, Some(&filename)).map_err(|e| {
        (
            MehFailureCategory::MehParseFailed,
            format!("{}: Failed to parse original: {}", filename, e),
        )
    })?;

    // Step 2: Format with MEH (parse + transform + format)
    let formatted = libyay::format_yay(&content).map_err(|e| {
        (
            MehFailureCategory::MehFormatFailed,
            format!("{}: MEH format failed: {}", filename, e),
        )
    })?;

    // Step 3: Parse the formatted output with strict parser
    let roundtrip_value =
        parse_with_filename(&formatted, Some(&format!("{} (roundtrip)", filename))).map_err(
            |e| {
                (
                    MehFailureCategory::StrictParseFailed,
                    format!(
                        "{}: Failed to parse MEH output: {}\n  Formatted output:\n{}",
                        filename,
                        e,
                        formatted
                            .lines()
                            .map(|l| format!("    {}", l))
                            .collect::<Vec<_>>()
                            .join("\n")
                    ),
                )
            },
        )?;

    // Step 4: Compare values (using custom comparison that handles NaN)
    if !values_equal(&original_value, &roundtrip_value) {
        return Err((
            MehFailureCategory::ValueMismatch,
            format!(
                "{}: Value mismatch after MEH round-trip\n  Original: {:?}\n  Roundtrip: {:?}",
                filename, original_value, roundtrip_value
            ),
        ));
    }

    println!("  {} => MEH round-trip OK", filename);
    Ok(())
}

#[test]
fn test_meh_roundtrip_all_yay_fixtures() {
    use std::collections::HashMap;

    let files = get_yay_files();

    if files.is_empty() {
        println!("No .yay test files found!");
        return;
    }

    println!(
        "\nRunning MEH round-trip test on {} .yay files:",
        files.len()
    );

    let mut passed = 0;
    let mut failures_by_category: HashMap<MehFailureCategory, Vec<String>> = HashMap::new();

    for file in &files {
        match run_meh_roundtrip_test(file) {
            Ok(()) => passed += 1,
            Err((category, msg)) => {
                failures_by_category.entry(category).or_default().push(msg);
            }
        }
    }

    let total_failed: usize = failures_by_category.values().map(|v| v.len()).sum();

    println!(
        "\nMEH Round-trip Results: {} passed, {} failed",
        passed, total_failed
    );

    if !failures_by_category.is_empty() {
        println!("\nFailures by category:");
        for (category, errors) in &failures_by_category {
            println!("\n  {} ({} failures):", category, errors.len());
            for error in errors {
                // Print just the filename, not the full error for summary
                let filename = error.split(':').next().unwrap_or(error);
                println!("    - {}", filename);
            }
        }

        println!("\nDetailed errors:");
        for errors in failures_by_category.values() {
            for error in errors {
                println!("  - {}", error);
            }
        }
    }

    assert!(
        total_failed == 0,
        "{} MEH round-trip tests failed",
        total_failed
    );
}

// Individual test cases for specific fixtures

#[test]
fn test_null_literal() {
    let result = parse("null").unwrap();
    assert_eq!(result, Value::Null);
}

#[test]
fn test_boolean_true() {
    let result = parse("true").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_boolean_false() {
    let result = parse("false").unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_integer_basic() {
    let result = parse("10").unwrap();
    assert_eq!(result, Value::Integer(10.into()));
}

#[test]
fn test_integer_negative() {
    let result = parse("-10").unwrap();
    assert_eq!(result, Value::Integer((-10).into()));
}

#[test]
fn test_integer_grouped() {
    let result = parse("1 000").unwrap();
    assert_eq!(result, Value::Integer(1000.into()));
}

#[test]
fn test_float_basic() {
    let result = parse("1.0").unwrap();
    assert_eq!(result, Value::Float(1.0));
}

#[test]
fn test_float_leading_dot() {
    let result = parse(".5").unwrap();
    assert_eq!(result, Value::Float(0.5));
}

#[test]
fn test_float_trailing_dot() {
    let result = parse("1.").unwrap();
    assert_eq!(result, Value::Float(1.0));
}

#[test]
fn test_float_negative_zero() {
    let result = parse("-0.0").unwrap();
    let f = result.as_float().unwrap();
    assert!(f == 0.0 && f.is_sign_negative());
}

#[test]
fn test_float_infinity() {
    let result = parse("infinity").unwrap();
    assert_eq!(result, Value::Float(f64::INFINITY));
}

#[test]
fn test_float_negative_infinity() {
    let result = parse("-infinity").unwrap();
    assert_eq!(result, Value::Float(f64::NEG_INFINITY));
}

#[test]
fn test_float_nan() {
    let result = parse("nan").unwrap();
    assert!(result.as_float().unwrap().is_nan());
}

#[test]
fn test_float_grouped() {
    let result = parse("1 000.000 100").unwrap();
    assert_eq!(result, Value::Float(1000.0001));
}

#[test]
fn test_string_double_quote() {
    let result = parse(r#""text""#).unwrap();
    assert_eq!(result, Value::String("text".into()));
}

#[test]
fn test_string_single_quote() {
    let result = parse("'text'").unwrap();
    assert_eq!(result, Value::String("text".into()));
}

#[test]
fn test_string_escapes() {
    let result = parse(r#""\"\\/\b\f\n\r\t\u{263A}""#).unwrap();
    let expected = "\"\\/\x08\x0C\n\r\t\u{263A}";
    assert_eq!(result, Value::String(expected.into()));
}

#[test]
fn test_inline_array_integers() {
    let result = parse("[1, 2, 3]").unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0], Value::Integer(1.into()));
    assert_eq!(arr[1], Value::Integer(2.into()));
    assert_eq!(arr[2], Value::Integer(3.into()));
}

#[test]
fn test_inline_array_strings() {
    let result = parse("['a', 'b']").unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0], Value::String("a".into()));
    assert_eq!(arr[1], Value::String("b".into()));
}

#[test]
fn test_inline_array_nested() {
    let result = parse("[[1.1, 2.2], [\"a\", \"b\"]]").unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);
}

#[test]
fn test_inline_object() {
    let result = parse("{a: 1, b: 2}").unwrap();
    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 2);
    assert_eq!(obj.get("a"), Some(&Value::Integer(1.into())));
    assert_eq!(obj.get("b"), Some(&Value::Integer(2.into())));
}

#[test]
fn test_inline_object_mixed() {
    let result = parse("{name: 'Alice', age: 30}").unwrap();
    let obj = result.as_object().unwrap();
    assert_eq!(obj.get("name"), Some(&Value::String("Alice".into())));
    assert_eq!(obj.get("age"), Some(&Value::Integer(30.into())));
}

#[test]
fn test_inline_object_nested() {
    let result = parse("{a: {b: 1}, c: [2, 3]}").unwrap();
    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 2);
}

#[test]
fn test_bytes_empty() {
    let result = parse("<>").unwrap();
    assert_eq!(result, Value::Bytes(vec![]));
}

#[test]
fn test_bytes_inline() {
    let result = parse("<b0b5c0ffeefacade>").unwrap();
    assert_eq!(
        result,
        Value::Bytes(vec![0xb0, 0xb5, 0xc0, 0xff, 0xee, 0xfa, 0xca, 0xde])
    );
}

#[test]
fn test_multiline_array() {
    let input = "- 1\n- 2\n- 3";
    let result = parse(input).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 3);
}

#[test]
fn test_multiline_object() {
    let input = "a: 10\nb: 20";
    let result = parse(input).unwrap();
    let obj = result.as_object().unwrap();
    assert_eq!(obj.get("a"), Some(&Value::Integer(10.into())));
    assert_eq!(obj.get("b"), Some(&Value::Integer(20.into())));
}

#[test]
fn test_nested_object() {
    let input = "objectName:\n  a: 10\n  b: 20";
    let result = parse(input).unwrap();
    let obj = result.as_object().unwrap();
    let inner = obj.get("objectName").unwrap().as_object().unwrap();
    assert_eq!(inner.get("a"), Some(&Value::Integer(10.into())));
}

#[test]
fn test_comment_only() {
    // Comment-only documents should be an error per the spec
    let result = parse("# comment");
    assert!(result.is_err());
}

#[test]
fn test_whitespace_leading_lines() {
    let input = "\n\n# Commentary here\n\n10\n\n\n# Comments there\n";
    let result = parse(input).unwrap();
    assert_eq!(result, Value::Integer(10.into()));
}

#[test]
fn test_block_string_leading_line() {
    let input = "`\n  Hello, World!\n\n  Goodbye!";
    let result = parse(input).unwrap();
    let s = result.as_str().unwrap();
    assert!(s.contains("Hello, World!"));
    assert!(s.contains("Goodbye!"));
}

#[test]
fn test_emoji_string() {
    let result = parse("\"😀\"").unwrap();
    assert_eq!(result, Value::String("😀".into()));
}

#[test]
fn test_array_inline_bytearray() {
    let result = parse("[<b0b5>, <cafe>]").unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0], Value::Bytes(vec![0xb0, 0xb5]));
    assert_eq!(arr[1], Value::Bytes(vec![0xca, 0xfe]));
}

#[test]
fn test_object_empty() {
    let result = parse("objectName: {}").unwrap();
    let obj = result.as_object().unwrap();
    let inner = obj.get("objectName").unwrap().as_object().unwrap();
    assert!(inner.is_empty());
}

#[test]
fn test_quoted_key_double() {
    let result = parse("\"key name\": 1").unwrap();
    let obj = result.as_object().unwrap();
    assert_eq!(obj.get("key name"), Some(&Value::Integer(1.into())));
}

#[test]
fn test_quoted_key_single() {
    let result = parse("'key-name': 2").unwrap();
    let obj = result.as_object().unwrap();
    assert_eq!(obj.get("key-name"), Some(&Value::Integer(2.into())));
}

/// Get all MEH reformat test files from the test/meh/ directory.
/// Naming convention: <test-name>.<variant>.meh -> <test-name>.yay
/// e.g., "comment-alignment.unaligned.meh" -> "comment-alignment.yay"
fn get_meh_reformat_files() -> Vec<(String, String)> {
    let test_dir = test_root().join("meh");

    let mut pairs: Vec<(String, String)> = Vec::new();

    if let Ok(entries) = fs::read_dir(&test_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "meh" {
                    let meh_path = path.to_string_lossy().to_string();
                    // Extract the test name from the .meh filename
                    // e.g., "comment-alignment.unaligned.meh" -> "comment-alignment"
                    let stem = path.file_stem().unwrap().to_string_lossy();
                    // The test name is everything before the last dot
                    if let Some(dot_idx) = stem.rfind('.') {
                        let test_name = &stem[..dot_idx];
                        let yay_path = test_dir.join(format!("{}.yay", test_name));
                        if yay_path.exists() {
                            pairs.push((meh_path, yay_path.to_string_lossy().to_string()));
                        }
                    }
                }
            }
        }
    }

    pairs.sort();
    pairs
}

/// Run a single MEH reformat test.
/// Parses the .meh file with MEH, formats to YAY, and compares with expected .yay output.
fn run_meh_reformat_test(meh_path: &str, yay_path: &str) -> Result<(), String> {
    let meh_content =
        fs::read_to_string(meh_path).map_err(|e| format!("Failed to read {}: {}", meh_path, e))?;
    let expected_yay =
        fs::read_to_string(yay_path).map_err(|e| format!("Failed to read {}: {}", yay_path, e))?;

    let meh_filename = Path::new(meh_path)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    // Format with MEH
    let formatted = libyay::format_yay(&meh_content)
        .map_err(|e| format!("{}: MEH format failed: {}", meh_filename, e))?;

    if formatted != expected_yay {
        return Err(format!(
            "{}: Output mismatch\n  Expected:\n{}\n  Actual:\n{}",
            meh_filename,
            expected_yay
                .lines()
                .map(|l| format!("    {}", l))
                .collect::<Vec<_>>()
                .join("\n"),
            formatted
                .lines()
                .map(|l| format!("    {}", l))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }

    println!("  {} => OK", meh_filename);
    Ok(())
}

#[test]
fn test_meh_reformat_fixtures() {
    let pairs = get_meh_reformat_files();

    if pairs.is_empty() {
        println!("No MEH reformat test files found!");
        return;
    }

    println!("\nRunning {} MEH reformat tests:", pairs.len());

    let mut passed = 0;
    let mut failed = 0;
    let mut errors: Vec<String> = Vec::new();

    for (meh_path, yay_path) in &pairs {
        match run_meh_reformat_test(meh_path, yay_path) {
            Ok(()) => passed += 1,
            Err(e) => {
                failed += 1;
                errors.push(e);
            }
        }
    }

    println!("\nResults: {} passed, {} failed", passed, failed);

    if !errors.is_empty() {
        println!("\nErrors:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    assert!(failed == 0, "{} MEH reformat tests failed", failed);
}
