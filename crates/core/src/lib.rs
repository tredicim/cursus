//! The data model, validation and target resolution.
//!
//! Both the CLI and the desktop app go through this crate, so it should not
//! depend on either of them. Nothing is here yet, the skeleton only exists so
//! the workspace matches the plan.

/// Version of the core crate, used by `cursus --version` later on.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
