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
    let output = cursus()
        .args(["list", "--json"])
        .output()
        .expect("run cursus");

    assert_eq!(output.status.code(), Some(5));
    let value: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(value["ok"], false);
    assert_eq!(value["command"], "list");
    assert_eq!(value["error"]["code"], "not_implemented");
    assert!(value["error"]["message"].is_string());
}

#[test]
fn json_flag_works_before_the_subcommand() {
    let directory = tempfile::tempdir().expect("create temp directory");
    let output = cursus()
        .current_dir(directory.path())
        .args(["--json", "init"])
        .output()
        .expect("run cursus");

    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(value["ok"], true);
}

#[test]
fn init_scaffolds_a_valid_project() {
    let directory = tempfile::tempdir().expect("create temp directory");
    let output = cursus()
        .current_dir(directory.path())
        .arg("init")
        .output()
        .expect("run cursus");

    assert!(
        output.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    for path in [
        "data/",
        "data/experience/",
        "targets/",
        "data/experience/first-entry.yaml",
        "targets/my-cv.yaml",
    ] {
        assert!(
            stdout.contains(path),
            "init did not report {path}: {stdout}"
        );
    }

    assert!(
        directory
            .path()
            .join("data/experience/first-entry.yaml")
            .is_file()
    );
    assert!(directory.path().join("targets/my-cv.yaml").is_file());

    let validate = cursus()
        .current_dir(directory.path())
        .arg("validate")
        .output()
        .expect("run cursus");
    assert!(
        validate.status.success(),
        "validate failed after init: {}",
        String::from_utf8_lossy(&validate.stderr)
    );
    let stdout = String::from_utf8(validate.stdout).expect("utf-8 stdout");
    assert!(stdout.contains("valid: 1 entries, 1 target"));
}

#[test]
fn init_is_idempotent_and_reports_skipped_paths() {
    let directory = tempfile::tempdir().expect("create temp directory");
    let first = cursus()
        .current_dir(directory.path())
        .arg("init")
        .output()
        .expect("run cursus");
    assert!(first.status.success());

    let entry = directory.path().join("data/experience/first-entry.yaml");
    let target = directory.path().join("targets/my-cv.yaml");
    let entry_before = fs::read(&entry).expect("read starter entry");
    let target_before = fs::read(&target).expect("read starter target");

    let second = cursus()
        .current_dir(directory.path())
        .arg("init")
        .output()
        .expect("run cursus");

    assert!(second.status.success());
    let stdout = String::from_utf8(second.stdout).expect("utf-8 stdout");
    assert!(
        stdout.contains("0 created, 5 skipped"),
        "unexpected output: {stdout}"
    );
    for path in [
        "data/",
        "data/experience/",
        "targets/",
        "data/experience/first-entry.yaml",
        "targets/my-cv.yaml",
    ] {
        assert!(
            stdout.contains(&format!("skipped {path}")),
            "init did not report skipping {path}: {stdout}"
        );
    }

    assert_eq!(fs::read(&entry).expect("read entry"), entry_before);
    assert_eq!(fs::read(&target).expect("read target"), target_before);
}

#[test]
fn init_never_touches_existing_files() {
    let directory = tempfile::tempdir().expect("create temp directory");
    let data = directory.path().join("data/experience");
    let targets = directory.path().join("targets");
    fs::create_dir_all(&data).expect("create data directory");
    fs::create_dir_all(&targets).expect("create targets directory");

    let entry = data.join("first-entry.yaml");
    let target = targets.join("my-cv.yaml");
    let keep = directory.path().join("data/keep.txt");
    let entry_contents = "\
id: first-entry
kind: project
title: My own entry
variants:
  default:
    summary: Custom placeholder.
";
    let target_contents = "\
name: My own CV
template: classic
include:
  - id: first-entry
";
    fs::write(&entry, entry_contents).expect("write existing entry");
    fs::write(&target, target_contents).expect("write existing target");
    fs::write(&keep, "keep me").expect("write extra file");

    let output = cursus()
        .current_dir(directory.path())
        .arg("init")
        .output()
        .expect("run cursus");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    assert!(
        stdout.contains("0 created, 5 skipped"),
        "unexpected output: {stdout}"
    );

    assert_eq!(
        fs::read_to_string(&entry).expect("read existing entry"),
        entry_contents
    );
    assert_eq!(
        fs::read_to_string(&target).expect("read existing target"),
        target_contents
    );
    assert_eq!(
        fs::read_to_string(&keep).expect("read extra file"),
        "keep me"
    );

    let validate = cursus()
        .current_dir(directory.path())
        .arg("validate")
        .output()
        .expect("run cursus");
    assert!(
        validate.status.success(),
        "validate failed with existing files: {}",
        String::from_utf8_lossy(&validate.stderr)
    );
}

#[test]
fn init_json_reports_created_and_skipped_paths() {
    let directory = tempfile::tempdir().expect("create temp directory");
    let first = cursus()
        .current_dir(directory.path())
        .args(["init", "--json"])
        .output()
        .expect("run cursus");

    assert!(first.status.success());
    let value: Value = serde_json::from_slice(&first.stdout).expect("valid json");
    assert_eq!(value["ok"], true);
    assert_eq!(value["command"], "init");
    assert_eq!(value["data"]["root"], ".");
    let created = value["data"]["created"].as_array().expect("created array");
    for path in [
        "data/",
        "data/experience/",
        "targets/",
        "data/experience/first-entry.yaml",
        "targets/my-cv.yaml",
    ] {
        assert!(
            created.iter().any(|item| item.as_str() == Some(path)),
            "created is missing {path}: {value}"
        );
    }
    assert!(
        value["data"]["skipped"]
            .as_array()
            .expect("skipped array")
            .is_empty()
    );

    let second = cursus()
        .current_dir(directory.path())
        .args(["init", "--json"])
        .output()
        .expect("run cursus");

    assert!(second.status.success());
    let value: Value = serde_json::from_slice(&second.stdout).expect("valid json");
    assert!(
        value["data"]["created"]
            .as_array()
            .expect("created array")
            .is_empty()
    );
    assert_eq!(
        value["data"]["skipped"]
            .as_array()
            .expect("skipped array")
            .len(),
        5
    );
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

#[test]
fn build_tex_only_writes_tex_without_a_latex_compiler() {
    let directory = tempfile::tempdir().expect("create temp directory");
    let empty_path = directory.path().join("empty-path");
    fs::create_dir(&empty_path).expect("create empty PATH directory");

    let output = cursus()
        .current_dir(fixture("valid"))
        .env("PATH", &empty_path)
        .args(["build", "acme-swe", "--tex-only", "--output"])
        .arg(directory.path())
        .arg("--json")
        .output()
        .expect("run cursus");

    assert!(
        output.status.success(),
        "tex-only build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let value: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(value["ok"], true);
    assert_eq!(value["command"], "build");
    assert_eq!(value["data"]["target"], "acme-swe");
    assert!(
        value["data"].get("pdf").is_none(),
        "tex-only JSON must not have a pdf field: {value}"
    );

    let tex = PathBuf::from(value["data"]["tex"].as_str().expect("tex path"));
    assert_eq!(tex, directory.path().join("acme-swe.tex"));
    assert!(tex.is_file(), "TeX was not written");
    assert!(
        !directory.path().join("acme-swe.pdf").exists(),
        "tex-only must not write a PDF"
    );

    let tex = fs::read_to_string(&tex).expect("read TeX");
    let first_content_line = tex
        .lines()
        .find(|line| !line.starts_with('%'))
        .expect("TeX is not empty");
    assert!(
        first_content_line.starts_with("\\documentclass"),
        "unexpected first line: {first_content_line}"
    );
    assert!(tex.contains("\\cvsection{Experience}"));
}

#[test]
fn build_tex_only_human_output_names_the_tex_file() {
    let directory = tempfile::tempdir().expect("create temp directory");

    let output = cursus()
        .current_dir(fixture("valid"))
        .args(["build", "acme-swe", "--tex-only", "--output"])
        .arg(directory.path())
        .output()
        .expect("run cursus");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    assert!(stdout.contains("rendered acme-swe -> "));
    assert!(stdout.contains("acme-swe.tex"));
    assert!(!stdout.contains(".pdf"));
}

#[test]
fn build_tex_only_accepts_a_pdf_output_path() {
    let directory = tempfile::tempdir().expect("create temp directory");
    let pdf = directory.path().join("custom.pdf");

    let output = cursus()
        .current_dir(fixture("valid"))
        .args(["build", "acme-swe", "--tex-only", "--output"])
        .arg(&pdf)
        .output()
        .expect("run cursus");

    assert!(
        output.status.success(),
        "tex-only build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(directory.path().join("custom.tex").is_file());
    assert!(!pdf.exists(), "tex-only must not write the PDF");
}

#[test]
fn build_tex_only_skips_compiler_detection() {
    let directory = tempfile::tempdir().expect("create temp directory");
    let empty_path = directory.path().join("empty-path");
    fs::create_dir(&empty_path).expect("create empty PATH directory");

    let output = cursus()
        .current_dir(fixture("valid"))
        .env("PATH", &empty_path)
        .env("CURSUS_LATEX", "definitely-not-installed")
        .args(["build", "acme-swe", "--tex-only", "--output"])
        .arg(directory.path())
        .output()
        .expect("run cursus");

    assert!(
        output.status.success(),
        "tex-only build should not read CURSUS_LATEX: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn build_tex_only_matches_the_tex_from_a_normal_build() {
    if !cursus_render::latex_available() {
        eprintln!("skipping comparison test: no LaTeX compiler on PATH");
        return;
    }

    let tex_only_dir = tempfile::tempdir().expect("create temp directory");
    let full_dir = tempfile::tempdir().expect("create temp directory");

    let tex_only = cursus()
        .current_dir(fixture("valid"))
        .args(["build", "acme-swe", "--tex-only", "--output"])
        .arg(tex_only_dir.path())
        .output()
        .expect("run cursus");
    assert!(
        tex_only.status.success(),
        "tex-only build failed: {}",
        String::from_utf8_lossy(&tex_only.stderr)
    );

    let full = cursus()
        .current_dir(fixture("valid"))
        .args(["build", "acme-swe", "--output"])
        .arg(full_dir.path())
        .output()
        .expect("run cursus");
    assert!(
        full.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&full.stderr)
    );

    let tex_only = fs::read(tex_only_dir.path().join("acme-swe.tex")).expect("read tex-only TeX");
    let full = fs::read(full_dir.path().join("acme-swe.tex")).expect("read normal TeX");
    assert_eq!(tex_only, full, "the .tex files differ between modes");
}

#[test]
fn build_fails_when_cursus_latex_is_invalid() {
    let directory = tempfile::tempdir().expect("create temp directory");

    let output = cursus()
        .current_dir(fixture("valid"))
        .env("CURSUS_LATEX", "foo")
        .args(["build", "acme-swe", "--output"])
        .arg(directory.path())
        .arg("--json")
        .output()
        .expect("run cursus");

    assert_eq!(output.status.code(), Some(4));
    let value: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(value["ok"], false);
    assert_eq!(value["command"], "build");
    assert_eq!(value["error"]["code"], "build_failed");
    assert_eq!(
        value["error"]["message"],
        "CURSUS_LATEX=foo is invalid; use tectonic, latexmk or pdflatex"
    );
    assert!(!directory.path().join("acme-swe.pdf").exists());
}

#[test]
fn build_fails_when_the_pinned_compiler_is_missing() {
    let directory = tempfile::tempdir().expect("create temp directory");
    let empty_path = directory.path().join("empty-path");
    fs::create_dir(&empty_path).expect("create empty PATH directory");

    let output = cursus()
        .current_dir(fixture("valid"))
        .env("PATH", &empty_path)
        .env("CURSUS_LATEX", "pdflatex")
        .args(["build", "acme-swe", "--output"])
        .arg(directory.path())
        .arg("--json")
        .output()
        .expect("run cursus");

    assert_eq!(output.status.code(), Some(4));
    let value: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(value["ok"], false);
    assert_eq!(value["command"], "build");
    assert_eq!(value["error"]["code"], "build_failed");
    assert_eq!(
        value["error"]["message"],
        "CURSUS_LATEX=pdflatex is set but pdflatex was not found on PATH"
    );
    assert!(!directory.path().join("acme-swe.pdf").exists());
}
