//! The command-line surface.
//!
//! Every command is a stub for now. The shape matters more than the behaviour:
//! agents rely on `--json` and the exit codes, so those are fixed before the
//! real work lands.

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use serde::Serialize;

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
        /// Write the PDF here instead of the default output folder.
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
    Err(CliError::NotImplemented {
        command: cli.command.name(),
    })
}

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("{command} is not implemented yet")]
    NotImplemented { command: &'static str },
}

impl CliError {
    /// Stable machine-readable code. Agents should match on this, not on the
    /// human message.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotImplemented { .. } => "not_implemented",
        }
    }

    pub fn exit_code(&self) -> u8 {
        match self {
            Self::NotImplemented { .. } => EXIT_NOT_IMPLEMENTED,
        }
    }

    pub fn envelope(&self, command: &'static str) -> ErrorEnvelope<'_> {
        ErrorEnvelope {
            ok: false,
            command,
            error: ErrorBody {
                code: self.code(),
                message: self.to_string(),
            },
        }
    }
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
}
