//! Loading a project from disk and resolving a target to concrete entries.
//!
//! A project is a directory with `data/` (one entry per YAML file) and
//! `targets/` (one target per YAML file). File names are the stable IDs used on
//! the command line; the `id` inside an entry is the reference used by
//! targets.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

use crate::model::{Entry, Target, Template, Variant};
use crate::validate::{Issue, validate_project};

/// A parsed entry and the file it came from.
#[derive(Debug, Clone)]
pub struct EntryFile {
    pub path: PathBuf,
    pub entry: Entry,
}

/// A parsed target and the file it came from.
#[derive(Debug, Clone)]
pub struct TargetFile {
    pub path: PathBuf,
    pub id: String,
    pub target: Target,
}

/// A loaded project, including documents that failed to parse.
#[derive(Debug, Clone)]
pub struct Project {
    pub root: PathBuf,
    pub entries: Vec<EntryFile>,
    pub targets: Vec<TargetFile>,
    parse_issues: Vec<Issue>,
}

/// A target after its includes have been resolved to entries and variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTarget {
    pub name: String,
    pub template: Template,
    pub entries: Vec<ResolvedEntry>,
}

/// One selected entry with the chosen variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedEntry {
    pub entry: Entry,
    pub variant_name: String,
    pub variant: Variant,
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("no data/ or targets/ directory found in {}", .root.display())]
    NotAProject { root: PathBuf },

    #[error("failed to read {}: {source}", .path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl Project {
    /// Load every YAML document below `root/data` and `root/targets`.
    ///
    /// Unreadable files are I/O errors; malformed documents are collected as
    /// issues so one bad file does not hide the rest.
    pub fn load(root: impl AsRef<Path>) -> Result<Self, LoadError> {
        let root = root.as_ref().to_path_buf();
        let data_dir = root.join("data");
        let targets_dir = root.join("targets");

        if !data_dir.is_dir() && !targets_dir.is_dir() {
            return Err(LoadError::NotAProject { root });
        }

        let mut parse_issues = Vec::new();
        let mut entries = Vec::new();
        for path in yaml_files(&data_dir)? {
            let text = read(&path)?;
            match parse_yaml::<Entry>(&path, &text) {
                Ok(entry) => entries.push(EntryFile { path, entry }),
                Err(issue) => parse_issues.push(issue),
            }
        }

        let mut targets = Vec::new();
        for path in yaml_files(&targets_dir)? {
            let id = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default();
            let text = read(&path)?;
            match parse_yaml::<Target>(&path, &text) {
                Ok(target) => targets.push(TargetFile { path, id, target }),
                Err(issue) => parse_issues.push(issue),
            }
        }

        Ok(Self {
            root,
            entries,
            targets,
            parse_issues,
        })
    }

    /// Every parse and semantic problem, ordered by file and field.
    pub fn validate(&self) -> Vec<Issue> {
        let mut issues = self.parse_issues.clone();
        issues.extend(validate_project(&self.entries, &self.targets));
        issues.sort_by(|left, right| {
            left.file
                .cmp(&right.file)
                .then_with(|| left.field.cmp(&right.field))
                .then_with(|| left.message.cmp(&right.message))
        });
        issues
    }

    /// Find a target by file name. A `.yaml` suffix and a directory prefix are
    /// accepted so shell completion and editor paths both work.
    pub fn find_target(&self, name: &str) -> Option<&TargetFile> {
        let id = Path::new(name)
            .file_stem()
            .map(|stem| stem.to_string_lossy())
            .unwrap_or_else(|| name.into());
        self.targets.iter().find(|file| file.id == id)
    }

    /// Resolve the includes of a target, reporting missing entries or variants
    /// with the same field paths as validation.
    pub fn resolve(&self, file: &TargetFile) -> Result<ResolvedTarget, Vec<Issue>> {
        let mut available = BTreeMap::new();
        for entry in &self.entries {
            available
                .entry(entry.entry.id.as_str())
                .or_insert(&entry.entry);
        }

        let mut issues = Vec::new();
        let mut entries = Vec::new();
        for (index, include) in file.target.include.iter().enumerate() {
            let Some(entry) = available.get(include.id.as_str()) else {
                issues.push(Issue::new(
                    &file.path,
                    Some(format!("include[{index}].id")),
                    format!("entry `{}` does not exist", include.id),
                ));
                continue;
            };

            let Some(variant) = entry.variants.get(&include.variant) else {
                issues.push(Issue::new(
                    &file.path,
                    Some(format!("include[{index}].variant")),
                    format!(
                        "entry `{}` has no variant `{}`",
                        include.id, include.variant
                    ),
                ));
                continue;
            };

            entries.push(ResolvedEntry {
                entry: (*entry).clone(),
                variant_name: include.variant.clone(),
                variant: variant.clone(),
            });
        }

        if issues.is_empty() {
            Ok(ResolvedTarget {
                name: file.target.name.clone(),
                template: file.target.template,
                entries,
            })
        } else {
            Err(issues)
        }
    }
}

fn yaml_files(dir: &Path) -> Result<Vec<PathBuf>, LoadError> {
    let mut files = Vec::new();
    collect_yaml_files(dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_yaml_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), LoadError> {
    if !dir.exists() {
        return Ok(());
    }

    let entries = fs::read_dir(dir).map_err(|source| LoadError::Io {
        path: dir.to_path_buf(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| LoadError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|source| LoadError::Io {
            path: path.clone(),
            source,
        })?;

        if file_type.is_dir() {
            collect_yaml_files(&path, files)?;
        } else if file_type.is_file() && has_yaml_extension(&path) {
            files.push(path);
        }
    }

    Ok(())
}

fn has_yaml_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("yaml" | "yml")
    )
}

fn read(path: &Path) -> Result<String, LoadError> {
    fs::read_to_string(path).map_err(|source| LoadError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn parse_yaml<T: DeserializeOwned>(path: &Path, text: &str) -> Result<T, Issue> {
    let deserializer = yaml_serde::Deserializer::from_str(text);
    match serde_path_to_error::deserialize::<_, T>(deserializer) {
        Ok(value) => Ok(value),
        Err(error) => {
            let field = error.path().to_string();
            let field = if field.is_empty() || field == "." {
                None
            } else {
                Some(field)
            };
            Err(Issue::new(
                path,
                field,
                format!("invalid YAML: {}", error.inner()),
            ))
        }
    }
}
