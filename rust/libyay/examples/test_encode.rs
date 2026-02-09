//! Test the encode module against fixture files.

use libyay::{encode, parse, Format};
use std::fs;
use std::path::Path;

fn main() {
    let test_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("test");

    let formats = [
        (Format::JavaScript, "js"),
        (Format::Go, "go"),
        (Format::Python, "py"),
        (Format::C, "c"),
        (Format::Java, "java"),
        (Format::Scheme, "scm"),
    ];

    let mut passed = 0;
    let mut failed = 0;

    for entry in fs::read_dir(&test_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension().map(|e| e == "yay").unwrap_or(false) {
            let yay_content = fs::read_to_string(&path).unwrap();
            let basename = path.file_stem().unwrap().to_str().unwrap();

            match parse(&yay_content) {
                Ok(value) => {
                    for (format, ext) in &formats {
                        let fixture_path = test_dir.join(format!("{}.{}", basename, ext));
                        if fixture_path.exists() {
                            let expected = fs::read_to_string(&fixture_path).unwrap();
                            let expected = expected.trim();
                            let actual = encode(&value, *format);

                            if actual == expected {
                                passed += 1;
                            } else {
                                failed += 1;
                                println!("FAIL: {}.{}", basename, ext);
                                println!(
                                    "  Expected: {}",
                                    expected.chars().take(100).collect::<String>()
                                );
                                println!(
                                    "  Actual:   {}",
                                    actual.chars().take(100).collect::<String>()
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("Parse error for {}: {}", basename, e);
                }
            }
        }
    }

    println!("\nResults: {} passed, {} failed", passed, failed);
}
