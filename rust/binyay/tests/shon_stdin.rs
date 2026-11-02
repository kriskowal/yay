//! `-s -` and `-b -` read stdin, which can be read only once.

use std::io::Write;
use std::process::{Command, Output, Stdio};

fn yay(args: &[&str], stdin: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_yay"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("yay runs");
    // The child may exit without reading stdin, so a write error is fine.
    let _ = child.stdin.take().unwrap().write_all(stdin);
    child.wait_with_output().expect("yay exits")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

#[test]
fn string_from_stdin() {
    let output = yay(&["-s", "-"], b"hi\n");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "\"hi\\n\"\n");
}

#[test]
fn string_from_stdin_in_brackets() {
    let output = yay(&["[", "--content", "-s", "-", "]", "-t", "md"], b"hi\n");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "hi\n");
}

#[test]
fn bytes_from_stdin() {
    let output = yay(&["-b", "-", "-t", "yson"], &[0xff, 0x00, 0x80]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "\"*ff0080\"\n");
}

#[test]
fn bytes_from_stdin_in_brackets() {
    let output = yay(&["[", "--data", "-b", "-", "]", "-t", "yson"], &[0xfe]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "{\n  \"data\": \"*fe\"\n}\n");
}

#[test]
fn string_from_stdin_not_utf8() {
    let output = yay(&["-s", "-"], &[0xff, 0x0a]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("Cannot read stdin: stream did not contain valid UTF-8"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn stdin_named_twice() {
    let output = yay(&["[", "-s", "-", "-b", "-", "]"], b"hi\n");
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("stdin can be read only once"),
        "{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty());
}

#[test]
fn stdin_named_twice_before_a_missing_file() {
    // Refused before any file is read, so the missing file is not reported.
    let output = yay(
        &["[", "-s", "/nonexistent/abc123", "-s", "-", "-s", "-", "]"],
        b"",
    );
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("stdin can be read only once"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn stdin_named_by_shon_and_as_input() {
    for args in [
        &["-", "-s", "-"][..],
        &["-s", "-", "-"][..],
        &["[", "-b", "-", "]", "-"][..],
    ] {
        let output = yay(args, b"hi\n");
        assert!(!output.status.success(), "{:?}", args);
        assert!(
            stderr(&output).contains("both -s/-b - and the input - read it"),
            "{:?}: {}",
            args,
            stderr(&output)
        );
    }
}

#[test]
fn dash_input_beside_shon_without_stdin() {
    // A SHON expression is the whole input; `-` beside one that does not read
    // stdin is accepted as before.
    let output = yay(&["-", "[", "a", "]"], b"");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "[\"a\"]\n");
}

#[test]
fn file_named_dash() {
    let dir = std::env::temp_dir().join(format!("binyay-shon-dash-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("-"), "from file\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_yay"))
        .args(["-s", "./-"])
        .current_dir(&dir)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "\"from file\\n\"\n");
}
