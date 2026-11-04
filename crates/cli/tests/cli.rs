use std::process::Command;

use serde_json::Value;

fn cursus() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cursus"))
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
fn json_errors_are_a_stable_envelope() {
    for (command, expected) in [("validate", "validate"), ("schema", "schema")] {
        let output = cursus()
            .args([command, "--json"])
            .output()
            .expect("run cursus");

        assert_eq!(output.status.code(), Some(5));
        let value: Value = serde_json::from_slice(&output.stdout).expect("valid json");
        assert_eq!(value["ok"], false);
        assert_eq!(value["command"], expected);
        assert_eq!(value["error"]["code"], "not_implemented");
        assert!(value["error"]["message"].is_string());
    }
}

#[test]
fn json_flag_works_before_the_subcommand() {
    let output = cursus()
        .args(["--json", "validate"])
        .output()
        .expect("run cursus");

    assert_eq!(output.status.code(), Some(5));
    let value: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(value["ok"], false);
}

#[test]
fn human_errors_go_to_stderr() {
    let output = cursus().arg("validate").output().expect("run cursus");

    assert_eq!(output.status.code(), Some(5));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    assert!(stderr.contains("validate is not implemented yet"));
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
