//! The data model, validation, schema and target resolution.
//!
//! Both the CLI and the desktop app go through this crate, so it does not
//! depend on either of them.

mod model;
mod project;
mod schema;
mod validate;

pub use model::{
    Document, Entry, EntryKind, Link, Target, TargetInclude, Template, Variant, YearMonth,
};
pub use project::{EntryFile, LoadError, Project, ResolvedEntry, ResolvedTarget, TargetFile};
pub use schema::json_schema;
pub use validate::Issue;

/// Version of the core crate, used by `cursus --version` later on.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
