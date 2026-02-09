//! YAY command-line tool for parsing, formatting, and transcoding YAY documents.
//!
//! Usage: yay [OPTIONS] [FILE|DIR]
//!
//! Options:
//!       -f, --from <FORMAT>    Input format (meh, yay, json, yson, yaml, toml, cbor)
//!                              [default: meh, or yay when --check]
//!   -t, --to <FORMAT>      Output format (yay, json, yson, js, go, python, rust, c, java, scheme, yaml, toml, cbor, diag)
//!   -w, --write            Write output to file with inferred name
//!   -o, --output <FILE>    Write output to specified file
//!   --check                Check if file is valid (exit 0 if valid, 1 if invalid)
//!                          Defaults to strict YAY input; use --from meh for lenient
//!   -h, --help             Print help
//!   -V, --version          Print version

use libyay::{
    encode, format_yay, parse, parse_shon_bracket, parse_shon_file_bytes, parse_shon_file_string,
    parse_shon_hex, parse_with_filename, parse_yson, Format, Value,
};
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process;

mod transcode;

/// Check whether a string is a recognized format name for -f or -t.
fn is_format_name(s: &str) -> bool {
    matches!(
        s,
        "meh"
            | "yay"
            | "json"
            | "yson"
            | "js"
            | "javascript"
            | "go"
            | "python"
            | "py"
            | "rust"
            | "rs"
            | "c"
            | "java"
            | "scheme"
            | "scm"
            | "yaml"
            | "yml"
            | "toml"
            | "cbor"
            | "diag"
    )
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut from_format: Option<&str> = None;
    let mut to_format: Option<&str> = None;
    let mut write_back = false;
    let mut output_file: Option<&str> = None;
    let mut check_only = false;
    let mut input_path: Option<&str> = None;
    let mut shon_value: Option<Value> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "-V" | "--version" => {
                println!("yay {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "-f" | "--from" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: -f requires a format argument");
                    process::exit(1);
                }
                if !is_format_name(&args[i]) {
                    eprintln!("Error: Unknown format: {}", args[i]);
                    process::exit(1);
                }
                from_format = Some(&args[i]);
            }
            "-t" | "--to" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: -t requires a format argument");
                    process::exit(1);
                }
                if !is_format_name(&args[i]) {
                    eprintln!("Error: Unknown format: {}", args[i]);
                    process::exit(1);
                }
                to_format = Some(&args[i]);
            }
            "-w" | "--write" => {
                write_back = true;
            }
            "-o" | "--output" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --output requires an argument");
                    process::exit(1);
                }
                output_file = Some(&args[i]);
            }
            "--check" => {
                check_only = true;
            }
            "-" => {
                // Explicit stdin
                // input_path stays None, which means stdin
            }
            // SHON triggers
            "[" | "[]" | "[--]" => {
                if shon_value.is_some() {
                    eprintln!("Error: Multiple SHON expressions not supported");
                    process::exit(1);
                }
                if input_path.is_some() {
                    eprintln!("Error: Cannot combine input file with SHON expression");
                    process::exit(1);
                }
                match parse_shon_bracket(&args[i..]) {
                    Ok((value, consumed)) => {
                        shon_value = Some(value);
                        i += consumed;
                        // After consuming SHON, continue parsing remaining args as CLI flags
                        continue;
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        process::exit(1);
                    }
                }
            }
            "-x" => {
                if shon_value.is_some() {
                    eprintln!("Error: Multiple SHON expressions not supported");
                    process::exit(1);
                }
                if input_path.is_some() {
                    eprintln!("Error: Cannot combine input file with SHON expression");
                    process::exit(1);
                }
                match parse_shon_hex(&args[i..]) {
                    Ok((value, consumed)) => {
                        shon_value = Some(value);
                        i += consumed;
                        continue;
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        process::exit(1);
                    }
                }
            }
            "-b" => {
                if shon_value.is_some() {
                    eprintln!("Error: Multiple SHON expressions not supported");
                    process::exit(1);
                }
                if input_path.is_some() {
                    eprintln!("Error: Cannot combine input file with SHON expression");
                    process::exit(1);
                }
                match parse_shon_file_bytes(&args[i..]) {
                    Ok((value, consumed)) => {
                        shon_value = Some(value);
                        i += consumed;
                        continue;
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        process::exit(1);
                    }
                }
            }
            "-s" => {
                if shon_value.is_some() {
                    eprintln!("Error: Multiple SHON expressions not supported");
                    process::exit(1);
                }
                if input_path.is_some() {
                    eprintln!("Error: Cannot combine input file with SHON expression");
                    process::exit(1);
                }
                match parse_shon_file_string(&args[i..]) {
                    Ok((value, consumed)) => {
                        shon_value = Some(value);
                        i += consumed;
                        continue;
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        process::exit(1);
                    }
                }
            }
            arg if arg.starts_with('-') => {
                eprintln!("Error: Unknown option: {}", arg);
                process::exit(1);
            }
            _ => {
                if shon_value.is_some() {
                    eprintln!("Error: Cannot combine input file with SHON expression");
                    process::exit(1);
                }
                if input_path.is_some() {
                    eprintln!("Error: Multiple input paths not supported");
                    process::exit(1);
                }
                input_path = Some(&args[i]);
            }
        }
        i += 1;
    }

    // Cannot have both SHON and input format (SHON is its own input)
    if shon_value.is_some() && from_format.is_some() {
        eprintln!("Error: Cannot use -f/--from with SHON input");
        process::exit(1);
    }

    // Default input format: "yay" (strict) when --check, "meh" (lenient) otherwise.
    // Can always be overridden with --from.
    let from_format = from_format.unwrap_or(if check_only { "yay" } else { "meh" });

    // Validate options
    if write_back && output_file.is_some() {
        eprintln!("Error: --write and --output are mutually exclusive");
        process::exit(1);
    }

    // Determine output format
    // Default output is YAY (canonical form)
    let output_format_str = to_format.unwrap_or("yay");
    let output_format = parse_format(output_format_str);

    // SHON mode: we already have a Value, skip file reading and parsing
    if let Some(value) = shon_value {
        if check_only {
            // SHON is always valid if it parsed
            println!("ok");
            return;
        }
        let exit_code = output_value(
            &value,
            output_format_str,
            output_format,
            output_file,
            write_back,
            None,
        );
        process::exit(exit_code);
    }

    // Check if input is a directory
    if let Some(path) = input_path {
        let path_ref = Path::new(path);
        if path_ref.is_dir() {
            // Directory mode: process all .yay files
            if output_file.is_some() {
                eprintln!("Error: --output cannot be used with directory input");
                process::exit(1);
            }
            process_directory(
                path,
                from_format,
                output_format_str,
                output_format,
                write_back,
                check_only,
            );
            return;
        }
    }

    // Single file mode: always read raw bytes first, then derive string as needed.
    // This avoids the double-read problem for CBOR and supports stdin uniformly.
    let raw_bytes: Vec<u8> = match input_path {
        Some(path) => match fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) => {
                eprintln!("Error reading {}: {}", path, e);
                process::exit(1);
            }
        },
        None => {
            let mut buffer = Vec::new();
            if let Err(e) = io::stdin().read_to_end(&mut buffer) {
                eprintln!("Error reading stdin: {}", e);
                process::exit(1);
            }
            buffer
        }
    };

    let is_binary_input = from_format == "cbor";
    let input: String = if is_binary_input {
        // For CBOR, the string representation is unused by the parser,
        // but process_input still takes &str, so provide an empty string.
        String::new()
    } else {
        match String::from_utf8(raw_bytes.clone()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error: input is not valid UTF-8: {}", e);
                process::exit(1);
            }
        }
    };

    let input_bytes: Option<&[u8]> = if is_binary_input {
        Some(&raw_bytes)
    } else {
        None
    };

    let exit_code = process_input(
        &input,
        input_bytes,
        input_path,
        from_format,
        output_format_str,
        output_format,
        output_file,
        write_back,
        check_only,
    );
    process::exit(exit_code);
}

fn parse_format(s: &str) -> Format {
    match s {
        "yay" | "meh" => Format::Yay,
        "json" => Format::Json,
        "yson" => Format::Yson,
        "js" | "javascript" => Format::JavaScript,
        "go" => Format::Go,
        "python" | "py" => Format::Python,
        "rust" | "rs" => Format::Rust,
        "c" => Format::C,
        "java" => Format::Java,
        "scheme" | "scm" => Format::Scheme,
        "yaml" | "yml" => Format::Yaml,
        "toml" => Format::Toml,
        "cbor" => Format::Cbor,
        "diag" => Format::CborDiag,
        _ => {
            eprintln!("Error: Unknown format: {}", s);
            process::exit(1);
        }
    }
}

fn format_extension(format: Format) -> &'static str {
    match format {
        Format::Yay => "yay",
        Format::Json => "json",
        Format::Yson => "yson",
        Format::JavaScript => "js",
        Format::Go => "go",
        Format::Python => "py",
        Format::Rust => "rs",
        Format::C => "c",
        Format::Java => "java",
        Format::Scheme => "scm",
        Format::Yaml => "yaml",
        Format::Toml => "toml",
        Format::Cbor => "cbor",
        Format::CborDiag => "diag",
    }
}

fn process_directory(
    dir_path: &str,
    from_format: &str,
    output_format_str: &str,
    output_format: Format,
    write_back: bool,
    check_only: bool,
) {
    let entries = match fs::read_dir(dir_path) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error reading directory {}: {}", dir_path, e);
            process::exit(1);
        }
    };

    let mut had_errors = false;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "yay").unwrap_or(false) {
            let path_str = path.to_string_lossy();
            let input = match fs::read_to_string(&path) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("Error reading {}: {}", path_str, e);
                    had_errors = true;
                    continue;
                }
            };

            let exit_code = process_input(
                &input,
                None,
                Some(&path_str),
                from_format,
                output_format_str,
                output_format,
                None,
                write_back,
                check_only,
            );

            if exit_code != 0 {
                had_errors = true;
            }
        }
    }

    process::exit(if had_errors { 1 } else { 0 });
}

#[allow(clippy::too_many_arguments)]
fn process_input(
    input: &str,
    input_bytes: Option<&[u8]>,
    input_file: Option<&str>,
    from_format: &str,
    output_format_str: &str,
    output_format: Format,
    output_file: Option<&str>,
    write_back: bool,
    check_only: bool,
) -> i32 {
    let filename = input_file.map(|p| {
        Path::new(p)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| p.to_string())
    });

    // For strict YAY mode (--from yay), validate with strict parser first
    if from_format == "yay" {
        match parse_with_filename(input, filename.as_deref()) {
            Ok(_) => {
                // Strict parse succeeded, continue to MEH processing
            }
            Err(e) => {
                if let Some(path) = input_file {
                    eprintln!("{}: {}", path, e);
                } else {
                    eprintln!("Parse error: {}", e);
                }
                return 1;
            }
        }
    }

    // For --check mode, just validate
    if check_only {
        // For strict YAY, we already validated above
        if from_format == "yay" {
            if let Some(path) = input_file {
                println!("{}: ok", path);
            }
            return 0;
        }

        // For MEH, validate with MEH parser
        if from_format == "meh" {
            match format_yay(input) {
                Ok(_) => {
                    if let Some(path) = input_file {
                        println!("{}: ok", path);
                    }
                    return 0;
                }
                Err(e) => {
                    if let Some(path) = input_file {
                        eprintln!("{}: {}", path, e);
                    } else {
                        eprintln!("{}", e);
                    }
                    return 1;
                }
            }
        }

        // For JSON/YSON, validate with YSON parser
        if from_format == "json" || from_format == "yson" {
            match parse_yson(input) {
                Ok(_) => {
                    if let Some(path) = input_file {
                        println!("{}: ok", path);
                    }
                    return 0;
                }
                Err(e) => {
                    if let Some(path) = input_file {
                        eprintln!("{}: {}", path, e);
                    } else {
                        eprintln!("{}", e);
                    }
                    return 1;
                }
            }
        }

        // For YAML/TOML/CBOR, validate by parsing
        if from_format == "yaml" || from_format == "yml" {
            match transcode::yaml::decode(input) {
                Ok(_) => {
                    if let Some(path) = input_file {
                        println!("{}: ok", path);
                    }
                    return 0;
                }
                Err(e) => {
                    if let Some(path) = input_file {
                        eprintln!("{}: {}", path, e);
                    } else {
                        eprintln!("{}", e);
                    }
                    return 1;
                }
            }
        }

        if from_format == "toml" {
            match transcode::toml::decode(input) {
                Ok(_) => {
                    if let Some(path) = input_file {
                        println!("{}: ok", path);
                    }
                    return 0;
                }
                Err(e) => {
                    if let Some(path) = input_file {
                        eprintln!("{}: {}", path, e);
                    } else {
                        eprintln!("{}", e);
                    }
                    return 1;
                }
            }
        }

        if from_format == "cbor" {
            let bytes = input_bytes.unwrap_or(input.as_bytes());
            match transcode::cbor::decode(bytes) {
                Ok(_) => {
                    if let Some(path) = input_file {
                        println!("{}: ok", path);
                    }
                    return 0;
                }
                Err(e) => {
                    if let Some(path) = input_file {
                        eprintln!("{}: {}", path, e);
                    } else {
                        eprintln!("{}", e);
                    }
                    return 1;
                }
            }
        }
    }

    // Special case: YAY/MEH to YAY uses MEH formatter to preserve comments/key order
    if (from_format == "yay" || from_format == "meh") && output_format_str == "yay" {
        let output = match format_yay(input) {
            Ok(s) => s,
            Err(e) => {
                if let Some(path) = input_file {
                    eprintln!("{}: {}", path, e);
                } else {
                    eprintln!("Format error: {}", e);
                }
                return 1;
            }
        };

        write_text_output(&output, output_file, write_back, input_file, output_format);
        return 0;
    }

    // Parse input for other conversions
    let value: Value = match from_format {
        "yay" => match parse(input) {
            Ok(v) => v,
            Err(e) => {
                if let Some(path) = input_file {
                    eprintln!("{}: {}", path, e);
                } else {
                    eprintln!("Parse error: {}", e);
                }
                return 1;
            }
        },
        "meh" => {
            // For MEH input, first format to canonical YAY, then parse
            let canonical = match format_yay(input) {
                Ok(s) => s,
                Err(e) => {
                    if let Some(path) = input_file {
                        eprintln!("{}: {}", path, e);
                    } else {
                        eprintln!("Format error: {}", e);
                    }
                    return 1;
                }
            };
            match parse(&canonical) {
                Ok(v) => v,
                Err(e) => {
                    // This shouldn't happen if format_yay succeeded
                    if let Some(path) = input_file {
                        eprintln!("{}: {}", path, e);
                    } else {
                        eprintln!("Parse error: {}", e);
                    }
                    return 1;
                }
            }
        }
        "json" | "yson" => match parse_yson(input) {
            Ok(v) => v,
            Err(e) => {
                if let Some(path) = input_file {
                    eprintln!("{}: {}", path, e);
                } else {
                    eprintln!("Parse error: {}", e);
                }
                return 1;
            }
        },
        "yaml" | "yml" => match transcode::yaml::decode(input) {
            Ok(v) => v,
            Err(e) => {
                if let Some(path) = input_file {
                    eprintln!("{}: {}", path, e);
                } else {
                    eprintln!("Parse error: {}", e);
                }
                return 1;
            }
        },
        "toml" => match transcode::toml::decode(input) {
            Ok(v) => v,
            Err(e) => {
                if let Some(path) = input_file {
                    eprintln!("{}: {}", path, e);
                } else {
                    eprintln!("Parse error: {}", e);
                }
                return 1;
            }
        },
        "cbor" => {
            let bytes = input_bytes.unwrap_or(input.as_bytes());
            match transcode::cbor::decode(bytes) {
                Ok(v) => v,
                Err(e) => {
                    if let Some(path) = input_file {
                        eprintln!("{}: {}", path, e);
                    } else {
                        eprintln!("Parse error: {}", e);
                    }
                    return 1;
                }
            }
        }
        _ => {
            eprintln!("Error: Unknown input format: {}", from_format);
            return 1;
        }
    };

    // Check-only mode
    if check_only {
        if let Some(path) = input_file {
            println!("{}: ok", path);
        }
        return 0;
    }

    // Check for JSON incompatibility
    if output_format == Format::Json {
        if let Some(reason) = value.json_incompatibility() {
            eprintln!(
                "Error: Cannot convert to JSON because the document contains {}.",
                reason
            );
            eprintln!("Hint: Try using YSON format instead (-t yson), which supports these types.");
            return 1;
        }
    }

    // Handle output formats that need special treatment
    match output_format {
        Format::Yaml => match transcode::yaml::encode(&value) {
            Ok(output) => {
                write_text_output(&output, output_file, write_back, input_file, output_format);
            }
            Err(e) => {
                eprintln!("Error: Cannot convert to YAML: {}", e);
                return 1;
            }
        },
        Format::Toml => match transcode::toml::encode(&value) {
            Ok(output) => {
                write_text_output(&output, output_file, write_back, input_file, output_format);
            }
            Err(e) => {
                eprintln!("Error: Cannot convert to TOML: {}", e);
                return 1;
            }
        },
        Format::Cbor => match transcode::cbor::encode(&value) {
            Ok(bytes) => {
                write_binary_output(&bytes, output_file, write_back, input_file, output_format);
            }
            Err(e) => {
                eprintln!("Error: Cannot convert to CBOR: {}", e);
                return 1;
            }
        },
        Format::CborDiag => {
            // Encode to CBOR bytes first, then render as diagnostic notation.
            // This ensures the diagnostic output reflects the actual wire encoding.
            match transcode::cbor::encode(&value) {
                Ok(bytes) => match transcode::cbor::diagnostic(&bytes) {
                    Ok(output) => {
                        write_text_output(
                            &output,
                            output_file,
                            write_back,
                            input_file,
                            output_format,
                        );
                    }
                    Err(e) => {
                        eprintln!("Error: Cannot render CBOR diagnostic notation: {}", e);
                        return 1;
                    }
                },
                Err(e) => {
                    eprintln!("Error: Cannot convert to CBOR: {}", e);
                    return 1;
                }
            }
        }
        _ => {
            // Use libyay's encode for all other formats
            let output = encode(&value, output_format);
            write_text_output(&output, output_file, write_back, input_file, output_format);
        }
    }

    0
}

/// Output a Value that was already parsed (e.g. from SHON).
/// This skips the parse phase and goes straight to encoding/output.
fn output_value(
    value: &Value,
    output_format_str: &str,
    output_format: Format,
    output_file: Option<&str>,
    write_back: bool,
    input_file: Option<&str>,
) -> i32 {
    // For SHON → YAY, encode via the standard encoder
    if output_format_str == "yay" {
        let output = encode(value, Format::Yay);
        write_text_output(&output, output_file, write_back, input_file, output_format);
        return 0;
    }

    // Check for JSON incompatibility
    if output_format == Format::Json {
        if let Some(reason) = value.json_incompatibility() {
            eprintln!(
                "Error: Cannot convert to JSON because the document contains {}.",
                reason
            );
            eprintln!("Hint: Try using YSON format instead (-t yson), which supports these types.");
            return 1;
        }
    }

    // Handle output formats that need special treatment
    match output_format {
        Format::Yaml => match transcode::yaml::encode(value) {
            Ok(output) => {
                write_text_output(&output, output_file, write_back, input_file, output_format);
            }
            Err(e) => {
                eprintln!("Error: Cannot convert to YAML: {}", e);
                return 1;
            }
        },
        Format::Toml => match transcode::toml::encode(value) {
            Ok(output) => {
                write_text_output(&output, output_file, write_back, input_file, output_format);
            }
            Err(e) => {
                eprintln!("Error: Cannot convert to TOML: {}", e);
                return 1;
            }
        },
        Format::Cbor => match transcode::cbor::encode(value) {
            Ok(bytes) => {
                write_binary_output(&bytes, output_file, write_back, input_file, output_format);
            }
            Err(e) => {
                eprintln!("Error: Cannot convert to CBOR: {}", e);
                return 1;
            }
        },
        Format::CborDiag => match transcode::cbor::encode(value) {
            Ok(bytes) => match transcode::cbor::diagnostic(&bytes) {
                Ok(output) => {
                    write_text_output(&output, output_file, write_back, input_file, output_format);
                }
                Err(e) => {
                    eprintln!("Error: Cannot render CBOR diagnostic notation: {}", e);
                    return 1;
                }
            },
            Err(e) => {
                eprintln!("Error: Cannot convert to CBOR: {}", e);
                return 1;
            }
        },
        _ => {
            let output = encode(value, output_format);
            write_text_output(&output, output_file, write_back, input_file, output_format);
        }
    }

    0
}

fn write_text_output(
    output: &str,
    output_file: Option<&str>,
    write_back: bool,
    input_file: Option<&str>,
    format: Format,
) {
    if let Some(path) = output_file {
        if let Err(e) = fs::write(path, output) {
            eprintln!("Error writing {}: {}", path, e);
            process::exit(1);
        }
    } else if write_back {
        if let Some(input_path) = input_file {
            let ext = format_extension(format);
            let output_path = Path::new(input_path).with_extension(ext);
            if let Err(e) = fs::write(&output_path, output) {
                eprintln!("Error writing {}: {}", output_path.display(), e);
                process::exit(1);
            }
        } else {
            eprintln!("Error: --write requires an input file");
            process::exit(1);
        }
    } else {
        print!("{}", output);
        // Ensure output ends with newline
        if !output.ends_with('\n') {
            println!();
        }
    }
}

fn write_binary_output(
    output: &[u8],
    output_file: Option<&str>,
    write_back: bool,
    input_file: Option<&str>,
    format: Format,
) {
    if let Some(path) = output_file {
        if let Err(e) = fs::write(path, output) {
            eprintln!("Error writing {}: {}", path, e);
            process::exit(1);
        }
    } else if write_back {
        if let Some(input_path) = input_file {
            let ext = format_extension(format);
            let output_path = Path::new(input_path).with_extension(ext);
            if let Err(e) = fs::write(&output_path, output) {
                eprintln!("Error writing {}: {}", output_path.display(), e);
                process::exit(1);
            }
        } else {
            eprintln!("Error: --write requires an input file");
            process::exit(1);
        }
    } else {
        // Write raw bytes to stdout
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        if let Err(e) = handle.write_all(output) {
            eprintln!("Error writing to stdout: {}", e);
            process::exit(1);
        }
    }
}

fn print_help() {
    println!(
        "yay - YAY command-line tool

USAGE:
    yay [OPTIONS] [FILE|DIR]

ARGS:
    [FILE|DIR]    Input file or directory (reads from stdin if not provided)
                  When a directory is given, processes all .yay files in it

OPTIONS:
    -f, --from <FORMAT>    Input format [default: meh, or yay when --check]
                           Supported: meh, yay, json, yson, yaml, toml, cbor
                           
                           'meh' (default) accepts loose formatting and reformats
                           to canonical YAY. 'yay' enforces strict YAY syntax
                           before transformation.
                           
                           When --check is used, the default flips to 'yay'
                           (strict). Use --from meh to check lenient syntax.
    
    -t, --to <FORMAT>      Output format
                           Supported: yay, json, yson, js, go, python, rust, c,
                                      java, scheme, yaml, toml, cbor, diag
    
    -w, --write            Write output to file with inferred extension
    
    -o, --output <FILE>    Write output to specified file (not valid with directory input)
    
    --check                Check if input is valid (exit 0 if valid, 1 if invalid)
                           Defaults to strict YAY input; use --from meh for lenient
    
    -h, --help             Print help
    
    -V, --version          Print version

EXAMPLES:
    # Reformat a MEH file to canonical YAY (default behavior)
    yay config.meh
    
    # Strictly validate a YAY file (--check defaults to strict)
    yay --check config.yay
    
    # Validate with lenient parsing (meh)
    yay --check --from meh config.yay
    
    # Validate all YAY files in a directory strictly
    yay --check ./configs/
    
    # Convert YAY to JSON (lenient input)
    yay -t json config.yay
    
    # Convert YAY to JSON (strict input)
    yay -f yay -t json config.yay
    
    # Convert JSON to YAY
    yay -f json -t yay data.json
    
    # Convert YAY to YAML
    yay -t yaml config.yay
    
    # Convert YAML to YAY
    yay -f yaml -t yay config.yaml
    
    # Convert YAY to TOML
    yay -t toml config.yay
    
    # Convert TOML to YAY
    yay -f toml -t yay config.toml
    
    # Convert YAY to CBOR (binary)
    yay -t cbor config.yay -o config.cbor
    
    # Convert CBOR to YAY
    yay -f cbor -t yay config.cbor
    
    # View CBOR in diagnostic notation (RFC 8949 §8)
    yay -f cbor -t diag config.cbor
    
    # Generate Go code from YAY
    yay -t go config.yay > config.go
    
    # Convert all YAY files in a directory to JSON
    yay -t json -w ./configs/
    
    # Convert YAY to YSON (JSON with YAY extensions)
    yay -t yson config.yay -o config.yson
    
    # SHON: construct data from command-line arguments
    yay [ --name hello --count 42 ]
    yay -t json [ --x 1.0 --y 2.0 ]
    yay -t yson -x cafe
    yay -b image.png -o image.yay
    yay -s message.txt
"
    );
}
