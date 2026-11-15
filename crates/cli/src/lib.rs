//! The command-line surface.
//!
//! Agents rely on `--json` and the exit codes, so those are fixed before the
//! rest of the commands land. Human output stays readable; machine output is
//! one JSON object per invocation.

use std::fmt;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Component, Path, PathBuf};

use clap::{Parser, Subcommand};
use cursus_core::{EntryFile, EntryKind, Issue, LoadError, Project, YearMonth, json_schema};
use serde::Serialize;
use serde_json::{Value, json};

/// Exit code for invalid input data or a target that cannot be built.
pub const EXIT_INVALID_DATA: u8 = 1;
/// Exit code for file-system failures.
pub const EXIT_IO: u8 = 3;
/// Exit code for a failed LaTeX/PDF build.
pub const EXIT_BUILD: u8 = 4;

#[derive(Debug, Parser)]
#[command(
    name = "cursus",
    version,
    about = "Write your career once. Tailor it for every job.",
    long_about = None
)]
pub struct Cli {
    /// Print machine-readable JSON instead of text.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create data/ and targets/.
    Init,
    /// Check the data files.
    Validate,
    /// List entries, optionally filtered by tag.
    List {
        /// Only show entries with this tag.
        #[arg(long)]
        tag: Option<String>,
    },
    /// Render a target to LaTeX and PDF.
    Build {
        /// Target name, e.g. acme-swe.
        target: String,
        /// Write the `.tex` file and skip the LaTeX/PDF step.
        #[arg(long)]
        tex_only: bool,
        /// Write the PDF here instead of the output/ folder. A directory keeps
        /// the target name; a `.pdf` path sets the exact file.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Print the JSON Schema.
    Schema,
}

impl Command {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Validate => "validate",
            Self::List { .. } => "list",
            Self::Build { .. } => "build",
            Self::Schema => "schema",
        }
    }
}

/// Run a parsed command. Returns an error that the binary turns into an exit
/// code and, with `--json`, a stable envelope.
pub fn run(cli: Cli) -> Result<(), CliError> {
    let Cli { json, command } = cli;

    match command {
        Command::Init => init(json),
        Command::List { tag } => list(json, tag.as_deref()),
        Command::Build {
            target,
            tex_only,
            output,
        } => build(json, &target, tex_only, output.as_deref()),
        Command::Validate => validate(json),
        Command::Schema => {
            let schema = json_schema();
            println!(
                "{}",
                serde_json::to_string_pretty(&schema).expect("the schema is serializable")
            );
            Ok(())
        }
    }
}

/// A minimal entry for a new project. The values are valid on purpose so
/// `cursus validate` succeeds immediately after `init`.
const STARTER_ENTRY: &str = "\
# Replace the placeholder values with your own experience, then run
# `cursus validate`.
id: first-entry
kind: experience
title: Your role
organization: Your organization
start: 2024-01
tags: [example]
variants:
  default:
    summary: Describe what you did and the impact you had.
";

/// A minimal target that includes the starter entry.
const STARTER_TARGET: &str = "\
# Replace the placeholder values and include the entries for this CV.
name: My CV
template: classic
include:
  - id: first-entry
    variant: default
";

fn init(json: bool) -> Result<(), CliError> {
    let root = Path::new(".");
    let data_dir = root.join("data");
    let experience_dir = data_dir.join("experience");
    let targets_dir = root.join("targets");
    let entry_path = experience_dir.join("first-entry.yaml");
    let target_path = targets_dir.join("my-cv.yaml");

    let mut created = Vec::new();
    let mut skipped = Vec::new();

    for directory in [&data_dir, &experience_dir, &targets_dir] {
        if directory.is_dir() {
            skipped.push(format!("{}/", display_path(directory)));
            continue;
        }

        fs::create_dir_all(directory).map_err(|source| CliError::Io {
            action: "create directory",
            path: directory.clone(),
            source,
        })?;
        created.push(format!("{}/", display_path(directory)));
    }

    for (path, contents) in [(&entry_path, STARTER_ENTRY), (&target_path, STARTER_TARGET)] {
        let mut file = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
        {
            Ok(file) => file,
            Err(source) if source.kind() == ErrorKind::AlreadyExists => {
                skipped.push(display_path(path));
                continue;
            }
            Err(source) => {
                return Err(CliError::Io {
                    action: "create",
                    path: path.clone(),
                    source,
                });
            }
        };

        file.write_all(contents.as_bytes())
            .map_err(|source| CliError::Io {
                action: "write",
                path: path.clone(),
                source,
            })?;
        created.push(display_path(path));
    }

    if json {
        print_success(
            "init",
            &json!({
                "root": ".",
                "created": created,
                "skipped": skipped,
            }),
        );
    } else {
        for path in &created {
            println!("created {path}");
        }
        for path in &skipped {
            println!("skipped {path} (already exists)");
        }
        println!(
            "initialized .: {} created, {} skipped",
            created.len(),
            skipped.len()
        );
    }

    Ok(())
}

/// Path relative to the project root, with `/` separators so the output does
/// not depend on the platform.
fn display_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[derive(Debug, Serialize)]
struct EntrySummary<'a> {
    id: &'a str,
    kind: EntryKind,
    title: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    organization: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start: Option<&'a YearMonth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end: Option<&'a YearMonth>,
    tags: Vec<&'a str>,
    variants: Vec<&'a str>,
    file: String,
}

fn list(json: bool, tag: Option<&str>) -> Result<(), CliError> {
    let project = Project::load(Path::new(".")).map_err(CliError::from)?;
    let issues = project.validate();
    if !issues.is_empty() {
        return Err(CliError::InvalidData { issues });
    }

    let mut files: Vec<&EntryFile> = project
        .entries
        .iter()
        .filter(|file| tag.is_none_or(|tag| file.entry.tags.iter().any(|value| value == tag)))
        .collect();
    files.sort_by(|left, right| left.entry.id.cmp(&right.entry.id));

    let entries: Vec<EntrySummary<'_>> =
        files.iter().map(|file| summarize(&project, file)).collect();

    if json {
        print_success("list", &json!({ "entries": entries }));
    } else {
        print_entries(&entries);
    }

    Ok(())
}

fn summarize<'a>(project: &Project, file: &'a EntryFile) -> EntrySummary<'a> {
    let entry = &file.entry;

    let mut tags: Vec<&str> = entry.tags.iter().map(String::as_str).collect();
    tags.sort_unstable();

    let mut variants: Vec<&str> = entry.variants.keys().map(String::as_str).collect();
    variants.sort_unstable();

    EntrySummary {
        id: &entry.id,
        kind: entry.kind,
        title: &entry.title,
        organization: entry.organization.as_deref(),
        location: entry.location.as_deref(),
        start: entry.start.as_ref(),
        end: entry.end.as_ref(),
        tags,
        variants,
        file: display_path(
            file.path
                .strip_prefix(&project.root)
                .unwrap_or(file.path.as_path()),
        ),
    }
}

fn print_entries(entries: &[EntrySummary<'_>]) {
    let id_width = entries
        .iter()
        .map(|entry| entry.id.chars().count())
        .max()
        .unwrap_or(0);
    let kind_width = entries
        .iter()
        .map(|entry| entry.kind.id().chars().count())
        .max()
        .unwrap_or(0);
    let title_width = entries
        .iter()
        .map(|entry| entry.title.chars().count())
        .max()
        .unwrap_or(0);
    let organization_width = entries
        .iter()
        .map(|entry| entry.organization.unwrap_or("").chars().count())
        .max()
        .unwrap_or(0);

    for entry in entries {
        let tags = format!("[{}]", entry.tags.join(", "));
        let organization = entry.organization.unwrap_or("");
        let line = format!(
            "{:<id_width$}  {:<kind_width$}  {:<title_width$}  {:<organization_width$}  {tags}",
            entry.id,
            entry.kind.id(),
            entry.title,
            organization,
        );
        println!("{}", line.trim_end());
    }
}

fn validate(json: bool) -> Result<(), CliError> {
    let project = Project::load(Path::new(".")).map_err(CliError::from)?;
    let issues = project.validate();

    if !issues.is_empty() {
        return Err(CliError::InvalidData { issues });
    }

    if json {
        print_success(
            "validate",
            &json!({
                "entries": project.entries.len(),
                "targets": project.targets.len(),
            }),
        );
    } else {
        println!(
            "valid: {} entries, {} targets",
            project.entries.len(),
            project.targets.len()
        );
    }

    Ok(())
}

fn build(json: bool, target: &str, tex_only: bool, output: Option<&Path>) -> Result<(), CliError> {
    let project = Project::load(Path::new(".")).map_err(CliError::from)?;
    let issues = project.validate();
    if !issues.is_empty() {
        return Err(CliError::InvalidData { issues });
    }

    let file = project
        .find_target(target)
        .ok_or_else(|| CliError::TargetNotFound {
            target: target.to_owned(),
        })?;
    let resolved = project
        .resolve(file)
        .map_err(|issues| CliError::InvalidData { issues })?;
    let tex = cursus_render::render(&resolved).map_err(|error| CliError::Build {
        message: error.to_string(),
    })?;

    let pdf = output_path(output, &file.id);
    let output_dir = parent_directory(&pdf);
    fs::create_dir_all(output_dir).map_err(|source| CliError::Io {
        action: "create",
        path: output_dir.to_path_buf(),
        source,
    })?;

    let tex_path = pdf.with_extension("tex");
    fs::write(&tex_path, tex).map_err(|source| CliError::Io {
        action: "write",
        path: tex_path.clone(),
        source,
    })?;

    if tex_only {
        if json {
            print_success(
                "build",
                &json!({
                    "target": file.id,
                    "tex": tex_path.display().to_string(),
                }),
            );
        } else {
            println!("rendered {} -> {}", file.id, tex_path.display());
        }

        return Ok(());
    }

    let pdf =
        cursus_render::compile_pdf(&tex_path, output_dir).map_err(|error| CliError::Build {
            message: error.to_string(),
        })?;

    if json {
        print_success(
            "build",
            &json!({
                "target": file.id,
                "tex": tex_path.display().to_string(),
                "pdf": pdf.display().to_string(),
            }),
        );
    } else {
        println!("built {} -> {}", file.id, pdf.display());
    }

    Ok(())
}

fn output_path(output: Option<&Path>, target: &str) -> PathBuf {
    match output {
        Some(path)
            if path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf")) =>
        {
            path.to_path_buf()
        }
        Some(directory) => directory.join(format!("{target}.pdf")),
        None => Path::new("output").join(format!("{target}.pdf")),
    }
}

fn parent_directory(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn print_success(command: &'static str, data: &Value) {
    let envelope = SuccessEnvelope {
        ok: true,
        command,
        data,
    };
    println!(
        "{}",
        serde_json::to_string(&envelope).expect("the success envelope is serializable")
    );
}

#[derive(Debug)]
pub enum CliError {
    InvalidData {
        issues: Vec<Issue>,
    },
    TargetNotFound {
        target: String,
    },
    Io {
        action: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
    Build {
        message: String,
    },
}

impl CliError {
    /// Stable machine-readable code. Agents should match on this, not on the
    /// human message.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidData { .. } => "invalid_data",
            Self::TargetNotFound { .. } => "target_not_found",
            Self::Io { .. } => "io_error",
            Self::Build { .. } => "build_failed",
        }
    }

    pub fn exit_code(&self) -> u8 {
        match self {
            Self::InvalidData { .. } | Self::TargetNotFound { .. } => EXIT_INVALID_DATA,
            Self::Io { .. } => EXIT_IO,
            Self::Build { .. } => EXIT_BUILD,
        }
    }

    pub fn envelope(&self, command: &'static str) -> ErrorEnvelope<'_> {
        ErrorEnvelope {
            ok: false,
            command,
            error: ErrorBody {
                code: self.code(),
                message: self.to_string(),
                details: match self {
                    Self::InvalidData { issues } => Some(json!({ "issues": issues })),
                    _ => None,
                },
            },
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidData { issues } => {
                writeln!(f, "invalid data ({} issue(s)):", issues.len())?;
                for issue in issues {
                    writeln!(f, "  {issue}")?;
                }
                Ok(())
            }
            Self::TargetNotFound { target } => {
                write!(f, "target `{target}` not found in targets/")
            }
            Self::Io {
                action,
                path,
                source,
            } => {
                write!(f, "failed to {action} {}: {source}", path.display())
            }
            Self::Build { message } => f.write_str(message),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<LoadError> for CliError {
    fn from(error: LoadError) -> Self {
        match error {
            LoadError::NotAProject { root } => Self::InvalidData {
                issues: vec![Issue::new(
                    root,
                    None,
                    "no data/ or targets/ directory found; run cursus from a project root",
                )],
            },
            LoadError::Io { path, source } => Self::Io {
                action: "read",
                path,
                source,
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SuccessEnvelope<'a, T> {
    pub ok: bool,
    pub command: &'a str,
    pub data: T,
}

#[derive(Debug, Serialize)]
pub struct ErrorEnvelope<'a> {
    pub ok: bool,
    pub command: &'a str,
    pub error: ErrorBody<'a>,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody<'a> {
    pub code: &'a str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}
