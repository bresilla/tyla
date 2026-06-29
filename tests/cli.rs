//! End-to-end tests for the `tyla` binary.
//!
//! These drive the compiled CLI the same way a user would: feeding input on
//! stdin and asserting on stdout. `CARGO_BIN_EXE_tyla` is provided by Cargo.

use std::io::Write;
use std::process::{Command, Stdio};

/// Run the `tyla` binary with `args`, feeding `stdin`, and return its stdout.
fn run(args: &[&str], stdin: &str) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tyla"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn tyla");

    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin.as_bytes())
        .expect("write stdin");

    let output = child.wait_with_output().expect("wait for tyla");
    assert!(output.status.success(), "tyla exited with {}", output.status);
    String::from_utf8(output.stdout).expect("utf8 stdout")
}

#[test]
fn latex_math_to_typst() {
    let out = run(&["-d", "l2t"], r"\frac{1}{2} + \sqrt{x^2+y^2}");
    assert!(out.contains("1/2"), "got: {out}");
    assert!(out.contains("sqrt("), "got: {out}");
}

#[test]
fn typst_math_to_latex() {
    let out = run(&["-d", "t2l"], "$frac(1,2) + sqrt(x^2+y^2)$");
    assert!(out.contains(r"\frac{1}{2}"), "got: {out}");
    assert!(out.contains(r"\sqrt{"), "got: {out}");
}

#[test]
fn detect_reports_latex() {
    let out = run(&["--detect"], r"\alpha + \beta");
    assert_eq!(out.trim(), "latex");
}

#[test]
fn info_subcommand_prints_version() {
    let out = run(&["info"], "");
    assert!(out.contains("tyla"), "got: {out}");
    assert!(out.contains(env!("CARGO_PKG_VERSION")), "got: {out}");
}
