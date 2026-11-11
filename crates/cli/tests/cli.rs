use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn cursus() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cursus"))
}

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(path)
}

#[test]
fn help_lists_every_command() {
    let output = cursus().arg("--help").output().expect("run cursus");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf-8 help");
    for command in ["init", "validate", "list", "build", "schema"] {
        assert!(stdout.contains(command), "help is missing {command}");
    }
}

#[test]
fn version_comes_from_cargo() {
    let output = cursus().arg("--version").output().expect("run cursus");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf-8 version");
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn unimplemented_commands_use_the_stable_envelope() {
    for command in ["init", "list"] {
        let output = cursus()
            .args([command, "--json"])
            .output()
            .expect("run cursus");

        assert_eq!(output.status.code(), Some(5), "{command}");
        let value: Value = serde_json::from_slice(&output.stdout).expect("valid json");
        assert_eq!(value["ok"], false);
        assert_eq!(value["command"], command);
        assert_eq!(value["error"]["code"], "not_implemented");
        assert!(value["error"]["message"].is_string());
    }
}

#[test]
fn json_flag_works_before_the_subcommand() {
    let output = cursus()
        .args(["--json", "init"])
        .output()
        .expect("run cursus");

    assert_eq!(output.status.code(), Some(5));
    let value: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(value["ok"], false);
}

#[test]
fn human_errors_go_to_stderr() {
    let output = cursus()
        .current_dir(fixture("invalid/duplicate-id"))
        .arg("validate")
        .output()
        .expect("run cursus");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    assert!(stderr.contains("duplicate entry id"));
    assert!(stderr.contains("id"));
}

#[test]
fn validate_accepts_the_valid_example() {
    let output = cursus()
        .current_dir(fixture("valid"))
        .arg("validate")
        .output()
        .expect("run cursus");

    assert!(
        output.status.success(),
        "validate failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    assert!(stdout.contains("valid: 5 entries, 1 target"));
}

#[test]
fn validate_json_reports_counts() {
    let output = cursus()
        .current_dir(fixture("valid"))
        .args(["validate", "--json"])
        .output()
        .expect("run cursus");

    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(value["ok"], true);
    assert_eq!(value["command"], "validate");
    assert_eq!(value["data"]["entries"], 5);
    assert_eq!(value["data"]["targets"], 1);
}

#[test]
fn validate_json_reports_issue_details() {
    let output = cursus()
        .current_dir(fixture("invalid/unknown-variant"))
        .args(["validate", "--json"])
        .output()
        .expect("run cursus");

    assert_eq!(output.status.code(), Some(1));
    let value: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(value["ok"], false);
    assert_eq!(value["command"], "validate");
    assert_eq!(value["error"]["code"], "invalid_data");

    let issues = value["error"]["details"]["issues"]
        .as_array()
        .expect("issues array");
    assert!(!issues.is_empty());
    assert!(issues[0]["file"].is_string());
    assert_eq!(issues[0]["field"], "include[0].variant");
    assert!(
        issues[0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("has no variant"))
    );
}

#[test]
fn schema_is_printed_as_json() {
    for args in [vec!["schema"], vec!["schema", "--json"]] {
        let output = cursus().args(args).output().expect("run cursus");

        assert!(output.status.success());
        let value: Value = serde_json::from_slice(&output.stdout).expect("valid json");
        assert!(value["$schema"].is_string());
        assert!(value["$defs"]["Entry"].is_object());
        assert!(value["$defs"]["Target"].is_object());
        assert!(value["$defs"]["YearMonth"].is_object());
    }
}

#[test]
fn unknown_commands_are_a_usage_error() {
    let output = cursus().arg("nope").output().expect("run cursus");

    assert_eq!(output.status.code(), Some(2));
    assert!(!output.stderr.is_empty());
}

#[test]
fn build_requires_a_target() {
    let output = cursus().arg("build").output().expect("run cursus");

    assert_eq!(output.status.code(), Some(2));
    assert!(!output.stderr.is_empty());
}

#[test]
fn build_reports_a_missing_target() {
    let output = cursus()
        .current_dir(fixture("valid"))
        .args(["build", "does-not-exist", "--json"])
        .output()
        .expect("run cursus");

    assert_eq!(output.status.code(), Some(1));
    let value: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(value["error"]["code"], "target_not_found");
}

#[test]
fn build_refuses_invalid_data_before_touching_the_disk() {
    let output = cursus()
        .current_dir(fixture("invalid/duplicate-id"))
        .args(["build", "acme-swe"])
        .output()
        .expect("run cursus");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    assert!(stderr.contains("invalid data"));
    assert!(!fixture("invalid/duplicate-id/output").exists());
}

#[test]
fn build_writes_tex_and_pdf_when_a_latex_compiler_is_available() {
    if !cursus_render::latex_available() {
        eprintln!("skipping build test: no LaTeX compiler on PATH");
        return;
    }

    let directory = tempfile::tempdir().expect("create temp directory");
    let output = cursus()
        .current_dir(fixture("valid"))
        .args(["build", "acme-swe", "--output"])
        .arg(directory.path())
        .arg("--json")
        .output()
        .expect("run cursus");

    assert!(
        output.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let value: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(value["data"]["target"], "acme-swe");

    let pdf = PathBuf::from(value["data"]["pdf"].as_str().expect("pdf path"));
    let tex = PathBuf::from(value["data"]["tex"].as_str().expect("tex path"));
    assert!(pdf.is_file(), "PDF was not written");
    assert!(tex.is_file(), "TeX was not written");
    assert!(fs::read(&pdf).expect("read PDF").starts_with(b"%PDF"));

    let tex = fs::read_to_string(&tex).expect("read TeX");
    assert!(tex.contains("\\documentclass"));
    assert!(tex.contains("\\cvsection{Experience}"));
}
