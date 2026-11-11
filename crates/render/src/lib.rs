//! LaTeX rendering, escaping and the PDF build pipeline.
//!
//! [`render`] turns a resolved target into a deterministic `.tex` document.
//! [`compile_pdf`] runs a LaTeX compiler on it. Neither step talks to the CLI,
//! so the desktop app can reuse both.

mod classic;
mod compile;

pub use classic::{RenderError, escape, render};
pub use compile::{CompileError, compile_pdf, compiler_name, latex_available};
