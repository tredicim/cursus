//! Semantic validation.
//!
//! Parsing catches malformed YAML and wrong types. This module catches the
//! rules that depend on more than one value: unique IDs, valid ranges, links
//! and references from targets to entries and variants.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::project::{EntryFile, TargetFile};

/// One validation or parse problem, tied to the file it came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Issue {
    /// File that contains the problem.
    pub file: PathBuf,
    /// Field path inside that file, e.g. `variants.default.bullets[0]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// Human-readable explanation.
    pub message: String,
}

impl Issue {
    pub fn new(
        file: impl Into<PathBuf>,
        field: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            file: file.into(),
            field,
            message: message.into(),
        }
    }
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.field {
            Some(field) => write!(f, "{}: {field}: {}", self.file.display(), self.message),
            None => write!(f, "{}: {}", self.file.display(), self.message),
        }
    }
}

pub(crate) fn validate_project(entries: &[EntryFile], targets: &[TargetFile]) -> Vec<Issue> {
    let mut issues = Vec::new();

    let mut entry_ids: BTreeMap<&str, &EntryFile> = BTreeMap::new();
    for file in entries {
        validate_entry(file, &mut issues);

        if let Some(first) = entry_ids.insert(file.entry.id.as_str(), file) {
            issues.push(Issue::new(
                &file.path,
                Some("id".to_owned()),
                format!(
                    "duplicate entry id `{}` (already defined in {})",
                    file.entry.id,
                    first.path.display()
                ),
            ));
        }
    }

    let mut target_ids: BTreeMap<String, &PathBuf> = BTreeMap::new();
    for file in targets {
        let id = file.id.clone();
        if let Some(first) = target_ids.insert(id.clone(), &file.path) {
            issues.push(Issue::new(
                &file.path,
                None,
                format!(
                    "duplicate target id `{id}` (already defined in {})",
                    first.display()
                ),
            ));
        }

        validate_target(file, &entry_ids, &mut issues);
    }

    issues
}

fn validate_entry(file: &EntryFile, issues: &mut Vec<Issue>) {
    let entry = &file.entry;
    let path = &file.path;

    validate_id(&entry.id, path.as_path(), "id", issues);
    if entry.title.trim().is_empty() {
        issues.push(Issue::new(
            path,
            Some("title".to_owned()),
            "title must not be empty",
        ));
    }

    if entry.end.is_some() && entry.start.is_none() {
        issues.push(Issue::new(
            path,
            Some("end".to_owned()),
            "end requires start to be set",
        ));
    }
    match (&entry.start, &entry.end) {
        (Some(start), Some(end)) if end < start => issues.push(Issue::new(
            path,
            Some("end".to_owned()),
            format!("end `{end}` is before start `{start}`"),
        )),
        _ => {}
    }

    let mut seen_tags = BTreeSet::new();
    for (index, tag) in entry.tags.iter().enumerate() {
        if tag.trim().is_empty() {
            issues.push(Issue::new(
                path,
                Some(format!("tags[{index}]")),
                "tag must not be empty",
            ));
        } else if !seen_tags.insert(tag.as_str()) {
            issues.push(Issue::new(
                path,
                Some(format!("tags[{index}]")),
                format!("duplicate tag `{tag}`"),
            ));
        }
    }

    for (index, link) in entry.links.iter().enumerate() {
        if link.label.trim().is_empty() {
            issues.push(Issue::new(
                path,
                Some(format!("links[{index}].label")),
                "link label must not be empty",
            ));
        }
        if !is_http_url(&link.url) {
            issues.push(Issue::new(
                path,
                Some(format!("links[{index}].url")),
                format!(
                    "link URL `{}` must be an absolute http:// or https:// URL",
                    link.url
                ),
            ));
        }
    }

    if entry.variants.is_empty() {
        issues.push(Issue::new(
            path,
            Some("variants".to_owned()),
            "entry must define at least one variant",
        ));
    }

    for (name, variant) in &entry.variants {
        if !is_valid_id(name) {
            issues.push(Issue::new(
                path,
                Some(format!("variants.{name}")),
                "variant names must contain only lowercase letters, digits and hyphens, and start with a letter or digit",
            ));
        }

        let has_summary = variant
            .summary
            .as_ref()
            .is_some_and(|summary| !summary.trim().is_empty());
        if variant.summary.is_some() && !has_summary {
            issues.push(Issue::new(
                path,
                Some(format!("variants.{name}.summary")),
                "summary must not be empty",
            ));
        }

        if !has_summary && variant.bullets.is_empty() {
            issues.push(Issue::new(
                path,
                Some(format!("variants.{name}.bullets")),
                "variant needs a summary or at least one bullet",
            ));
        }

        for (index, bullet) in variant.bullets.iter().enumerate() {
            if bullet.trim().is_empty() {
                issues.push(Issue::new(
                    path,
                    Some(format!("variants.{name}.bullets[{index}]")),
                    "bullet must not be empty",
                ));
            }
        }
    }
}

fn validate_target(
    file: &TargetFile,
    entry_ids: &BTreeMap<&str, &EntryFile>,
    issues: &mut Vec<Issue>,
) {
    let target = &file.target;
    let path = &file.path;

    let id = &file.id;
    if !is_valid_id(id) {
        issues.push(Issue::new(
            path,
            None,
            format!(
                "target file name `{id}` must contain only lowercase letters, digits and hyphens, and start with a letter or digit"
            ),
        ));
    }

    if target.name.trim().is_empty() {
        issues.push(Issue::new(
            path,
            Some("name".to_owned()),
            "name must not be empty",
        ));
    }

    if target.include.is_empty() {
        issues.push(Issue::new(
            path,
            Some("include".to_owned()),
            "target must include at least one entry",
        ));
    }

    let mut seen = BTreeSet::new();
    for (index, include) in target.include.iter().enumerate() {
        if !seen.insert(include.id.as_str()) {
            issues.push(Issue::new(
                path,
                Some(format!("include[{index}].id")),
                format!("entry `{}` is included more than once", include.id),
            ));
        }

        let Some(entry) = entry_ids.get(include.id.as_str()).copied() else {
            issues.push(Issue::new(
                path,
                Some(format!("include[{index}].id")),
                format!("entry `{}` does not exist", include.id),
            ));
            continue;
        };

        if !entry_has_variant(entry, &include.variant) {
            let available = variant_names(entry);
            issues.push(Issue::new(
                path,
                Some(format!("include[{index}].variant")),
                format!(
                    "entry `{}` has no variant `{}` (available: {})",
                    include.id,
                    include.variant,
                    if available.is_empty() {
                        "none".to_owned()
                    } else {
                        available.join(", ")
                    }
                ),
            ));
        }
    }
}

fn entry_has_variant(entry: &EntryFile, variant: &str) -> bool {
    entry.entry.variants.contains_key(variant)
}

fn variant_names(entry: &EntryFile) -> Vec<String> {
    entry.entry.variants.keys().cloned().collect()
}

pub(crate) fn is_valid_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && (bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit())
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

fn is_http_url(value: &str) -> bool {
    let value = value.trim();
    (value.starts_with("http://") || value.starts_with("https://"))
        && value.len() > "https://".len()
        && !value.chars().any(char::is_whitespace)
}

fn validate_id(value: &str, file: &Path, field: &str, issues: &mut Vec<Issue>) {
    if !is_valid_id(value) {
        issues.push(Issue::new(
            file,
            Some(field.to_owned()),
            "id must contain only lowercase letters, digits and hyphens, and start with a letter or digit",
        ));
    }
}
