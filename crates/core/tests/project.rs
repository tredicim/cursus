use std::fs;
use std::path::{Path, PathBuf};

use cursus_core::{EntryKind, Project, YearMonth, json_schema};

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn yaml_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read fixture directory") {
        let path = entry.expect("read fixture entry").path();
        if path.is_dir() {
            yaml_files(&path, files);
        } else if matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("yaml" | "yml")
        ) {
            files.push(path);
        }
    }
}

#[test]
fn valid_project_has_no_issues() {
    let project = Project::load(examples().join("valid")).expect("load valid project");

    let issues = project.validate();
    assert!(issues.is_empty(), "unexpected issues: {issues:#?}");
    assert_eq!(project.entries.len(), 5);
    assert_eq!(project.targets.len(), 1);
}

#[test]
fn invalid_fixtures_report_file_field_and_reason() {
    let root = examples().join("invalid");
    let mut fixtures: Vec<_> = fs::read_dir(&root)
        .expect("read invalid fixtures")
        .map(|entry| entry.expect("read fixture").path())
        .collect();
    fixtures.sort();

    assert!(!fixtures.is_empty(), "no invalid fixtures found");

    for fixture in fixtures {
        let project = Project::load(&fixture)
            .unwrap_or_else(|error| panic!("load {}: {error}", fixture.display()));
        let issues = project.validate();

        assert!(
            !issues.is_empty(),
            "fixture {} should be invalid",
            fixture.display()
        );
        for issue in issues {
            assert!(
                issue.file.starts_with(&fixture),
                "issue file {} is outside {}",
                issue.file.display(),
                fixture.display()
            );
            assert!(
                issue.field.is_some(),
                "issue in {} has no field: {}",
                fixture.display(),
                issue.message
            );
            assert!(
                !issue.message.trim().is_empty(),
                "issue in {} has no reason",
                fixture.display()
            );
        }
    }
}

#[test]
fn invalid_fixtures_explain_the_actual_problem() {
    let cases = [
        ("duplicate-id", "id", "duplicate entry id"),
        ("bad-date", "start", "month must be between"),
        ("unknown-variant", "include[0].variant", "has no variant"),
        ("missing-entry", "include[0].id", "does not exist"),
        ("bad-link", "links[0].url", "absolute http"),
        (
            "tag-whitespace",
            "tags[0]",
            "tag must not have leading or trailing whitespace",
        ),
        ("unknown-field", "variants.default", "unknown field"),
    ];

    for (fixture, field, reason) in cases {
        let project =
            Project::load(examples().join("invalid").join(fixture)).expect("load fixture");
        let issues = project.validate();

        assert!(
            issues.iter().any(|issue| {
                issue
                    .field
                    .as_deref()
                    .is_some_and(|value| value.contains(field))
                    && issue.message.contains(reason)
            }),
            "fixture {fixture} did not report `{reason}` at `{field}`: {issues:#?}"
        );
    }
}

#[test]
fn committed_schema_matches_the_rust_types() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schema.json");
    let committed: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("read schema.json"))
            .expect("schema.json is JSON");

    assert_eq!(
        committed,
        json_schema(),
        "schema.json is stale; run `cargo run -p cursus-cli -- schema > schema.json`"
    );
}

#[test]
fn schema_validates_every_valid_fixture() {
    let schema = json_schema();
    assert_eq!(schema["title"], "Cursus document");
    assert!(schema["$defs"]["Entry"].is_object());
    assert!(schema["$defs"]["Target"].is_object());

    let validator = jsonschema::validator_for(&schema).expect("compile JSON Schema");
    let mut files = Vec::new();
    yaml_files(&examples().join("valid"), &mut files);
    assert_eq!(files.len(), 6);

    for file in files {
        let text = fs::read_to_string(&file).expect("read fixture");
        let value: serde_json::Value = yaml_serde::from_str(&text).expect("fixture is YAML");

        if let Err(error) = validator.validate(&value) {
            panic!("schema rejected {}: {error}", file.display());
        }
    }
}

#[test]
fn resolution_keeps_target_order_and_selected_variants() {
    let project = Project::load(examples().join("valid")).expect("load valid project");
    let target = project.find_target("acme-swe").expect("find target");
    let resolved = project.resolve(target).expect("resolve target");

    assert_eq!(resolved.name, "Acme Software Engineering Internship");
    assert_eq!(resolved.entries.len(), 5);
    assert_eq!(resolved.entries[0].entry.kind, EntryKind::Experience);
    assert_eq!(resolved.entries[0].variant_name, "software-eng");
    assert_eq!(resolved.entries[4].entry.kind, EntryKind::Award);

    let project_entry = &resolved.entries[1];
    assert_eq!(project_entry.variant_name, "default");
    assert!(project_entry.entry.title.contains("{LaTeX}"));
    assert!(
        project_entry
            .variant
            .summary
            .as_deref()
            .is_some_and(|summary| summary.contains('&'))
    );
}

#[test]
fn year_month_rejects_impossible_dates() {
    assert_eq!(YearMonth::parse("2025-06").unwrap().as_str(), "2025-06");
    assert!(YearMonth::parse("2025-13").is_err());
    assert!(YearMonth::parse("2025-6").is_err());
    assert!(YearMonth::parse("June 2025").is_err());
}
