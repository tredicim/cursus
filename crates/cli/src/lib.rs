//! The command-line surface.
//!
//! Agents rely on `--json` and the exit codes, so those are fixed before the
//! rest of the commands land. Human output stays readable; machine output is
//! one JSON object per invocation.

use std::fmt;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use cursus_core::{Issue, LoadError, Project, json_schema};
use serde::Serialize;
use serde_json::{Value, json};

/// Exit code for invalid input data or a target that cannot be built.
pub const EXIT_INVALID_DATA: u8 = 1;
/// Exit code for file-system failures.
pub const EXIT_IO: u8 = 3;
/// Exit code for a failed LaTeX/PDF build.
pub const EXIT_BUILD: u8 = 4;
/// Exit code for a command that exists but has no implementation yet.
pub const EXIT_NOT_IMPLEMENTED: u8 = 5;

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
        Command::Init => Err(CliError::NotImplemented { command: "init" }),
        Command::List { .. } => Err(CliError::NotImplemented { command: "list" }),
        Command::Build { .. } => Err(CliError::NotImplemented { command: "build" }),
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
    NotImplemented {
        command: &'static str,
    },
    InvalidData {
        issues: Vec<Issue>,
    },
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl CliError {
    /// Stable machine-readable code. Agents should match on this, not on the
    /// human message.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotImplemented { .. } => "not_implemented",
            Self::InvalidData { .. } => "invalid_data",
            Self::Io { .. } => "io_error",
        }
    }

    pub fn exit_code(&self) -> u8 {
        match self {
            Self::NotImplemented { .. } => EXIT_NOT_IMPLEMENTED,
            Self::InvalidData { .. } => EXIT_INVALID_DATA,
            Self::Io { .. } => EXIT_IO,
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
            Self::NotImplemented { command } => {
                write!(f, "{command} is not implemented yet")
            }
            Self::InvalidData { issues } => {
                writeln!(f, "invalid data ({} issue(s)):", issues.len())?;
                for issue in issues {
                    writeln!(f, "  {issue}")?;
                }
                Ok(())
            }
            Self::Io { path, source } => {
                write!(f, "failed to read {}: {source}", path.display())
            }
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
            LoadError::Io { path, source } => Self::Io { path, source },
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
