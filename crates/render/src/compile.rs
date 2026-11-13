//! Compile a rendered `.tex` file to PDF.
//!
//! Cursus does not bundle a TeX distribution. It uses, in order, a `tectonic`,
//! `latexmk` or `pdflatex` executable found on `PATH`. Tectonic is preferred
//! because it fetches a reproducible bundle on demand; `latexmk` and
//! `pdflatex` cover the TeX Live/MiKTeX installs most people already have.
//! `CURSUS_LATEX=tectonic|latexmk|pdflatex` pins one explicitly.

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Tectonic,
    Latexmk,
    PdfLatex,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::Tectonic => "tectonic",
            Self::Latexmk => "latexmk",
            Self::PdfLatex => "pdflatex",
        }
    }

    fn program(self) -> &'static str {
        self.name()
    }

    fn from_name(name: &str) -> Option<Self> {
        match name {
            "tectonic" => Some(Self::Tectonic),
            "latexmk" => Some(Self::Latexmk),
            "pdflatex" => Some(Self::PdfLatex),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct Compiler {
    kind: Kind,
    program: PathBuf,
}

#[derive(Debug, Error)]
pub enum CompileError {
    #[error(
        "no LaTeX compiler found; install Tectonic or TeX Live/MiKTeX (pdflatex or latexmk) and put it on PATH"
    )]
    NoCompiler,

    #[error("CURSUS_LATEX={value} is invalid; use tectonic, latexmk or pdflatex")]
    InvalidConfiguredCompiler { value: String },

    #[error("CURSUS_LATEX={name} is set but {name} was not found on PATH")]
    ConfiguredCompilerMissing { name: &'static str },

    #[error("failed to run {compiler}: {source}")]
    Spawn {
        compiler: &'static str,
        #[source]
        source: io::Error,
    },

    #[error("{compiler} failed ({status}):\n{output}")]
    Failed {
        compiler: &'static str,
        status: String,
        output: String,
    },

    #[error("compilation succeeded but {} was not created", .path.display())]
    MissingOutput { path: PathBuf },

    #[error("failed to prepare the build directory: {0}")]
    Io(#[from] io::Error),
}

/// Name of the compiler Cursus would use, if one is installed and configured
/// correctly.
pub fn compiler_name() -> Option<&'static str> {
    detect().ok().flatten().map(|compiler| compiler.kind.name())
}

/// Whether a PDF can be compiled on this machine.
pub fn latex_available() -> bool {
    matches!(detect(), Ok(Some(_)))
}

/// Compile `tex` and return the path of the PDF next to it.
///
/// `output_dir` is created if needed. Auxiliary files are removed afterwards;
/// the `.tex` and `.pdf` files stay.
pub fn compile_pdf(tex: &Path, output_dir: &Path) -> Result<PathBuf, CompileError> {
    let compiler = detect()?.ok_or(CompileError::NoCompiler)?;

    fs::create_dir_all(output_dir)?;
    let output_dir = fs::canonicalize(output_dir)?;
    let tex = fs::canonicalize(tex)?;
    let stem = tex
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "cursus".to_owned());
    let pdf = output_dir.join(format!("{stem}.pdf"));

    let mut command = Command::new(&compiler.program);
    match compiler.kind {
        Kind::Tectonic => {
            command.arg("--outdir").arg(&output_dir).arg(&tex);
        }
        Kind::Latexmk => {
            command
                .arg("-pdf")
                .arg("-interaction=nonstopmode")
                .arg("-halt-on-error")
                .arg("-file-line-error")
                .arg(format!("-outdir={}", output_dir.display()))
                .arg(&tex);
        }
        Kind::PdfLatex => {
            command
                .arg("-interaction=nonstopmode")
                .arg("-halt-on-error")
                .arg("-file-line-error")
                .arg("-output-directory")
                .arg(&output_dir)
                .arg(&tex);
        }
    }

    if let Some(parent) = tex.parent() {
        command.current_dir(parent);
    }

    let output = command.output().map_err(|source| CompileError::Spawn {
        compiler: compiler.kind.name(),
        source,
    })?;

    if !output.status.success() {
        let mut log = String::from_utf8_lossy(&output.stdout).into_owned();
        log.push_str(&String::from_utf8_lossy(&output.stderr));
        return Err(CompileError::Failed {
            compiler: compiler.kind.name(),
            status: output.status.to_string(),
            output: tail(&log, 4_000),
        });
    }

    if !pdf.is_file() {
        return Err(CompileError::MissingOutput { path: pdf });
    }

    cleanup(&output_dir, &stem);
    Ok(pdf)
}

fn detect() -> Result<Option<Compiler>, CompileError> {
    detect_with(
        env::var_os("CURSUS_LATEX").as_deref(),
        env::var_os("PATH").as_deref(),
    )
}

/// Detect a compiler without reading the process environment, so tests can
/// pass a configured value and a `PATH` without racing on global state.
fn detect_with(
    configured: Option<&OsStr>,
    path: Option<&OsStr>,
) -> Result<Option<Compiler>, CompileError> {
    if let Some(value) = configured {
        let value = value
            .to_str()
            .ok_or_else(|| CompileError::InvalidConfiguredCompiler {
                value: value.to_string_lossy().into_owned(),
            })?;
        let kind =
            Kind::from_name(value).ok_or_else(|| CompileError::InvalidConfiguredCompiler {
                value: value.to_owned(),
            })?;
        let program = find_in_path(kind.program(), path)
            .ok_or(CompileError::ConfiguredCompilerMissing { name: kind.name() })?;

        return Ok(Some(Compiler { kind, program }));
    }

    for kind in [Kind::Tectonic, Kind::Latexmk, Kind::PdfLatex] {
        if let Some(program) = find_in_path(kind.program(), path) {
            return Ok(Some(Compiler { kind, program }));
        }
    }

    Ok(None)
}

fn find_in_path(name: &str, path: Option<&OsStr>) -> Option<PathBuf> {
    for directory in env::split_paths(path?) {
        let candidate = directory.join(name);
        if is_executable(&candidate) {
            return Some(candidate);
        }

        #[cfg(windows)]
        for extension in ["exe", "cmd", "bat"] {
            let candidate = directory.join(format!("{name}.{extension}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        path.is_file()
            && path
                .metadata()
                .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

fn cleanup(output_dir: &Path, stem: &str) {
    for extension in [
        "aux",
        "log",
        "out",
        "fls",
        "fdb_latexmk",
        "synctex.gz",
        "xdv",
    ] {
        let _ = fs::remove_file(output_dir.join(format!("{stem}.{extension}")));
    }
}

fn tail(value: &str, limit: usize) -> String {
    if value.len() <= limit {
        return value.to_owned();
    }

    let mut start = value.len() - limit;
    while !value.is_char_boundary(start) {
        start += 1;
    }
    format!("...{}", &value[start..])
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY_PATH: &str = "/cursus-test-path-that-does-not-exist";

    #[test]
    fn unset_configured_compiler_uses_detection() {
        let compiler = detect_with(None, Some(OsStr::new(EMPTY_PATH))).expect("detection succeeds");

        assert!(compiler.is_none());
    }

    #[test]
    fn invalid_configured_value_is_an_error() {
        let error = detect_with(Some(OsStr::new("foo")), Some(OsStr::new(EMPTY_PATH)))
            .expect_err("an invalid value must fail");

        assert_eq!(
            error.to_string(),
            "CURSUS_LATEX=foo is invalid; use tectonic, latexmk or pdflatex"
        );
    }

    #[test]
    fn configured_compiler_must_be_on_path() {
        let error = detect_with(Some(OsStr::new("pdflatex")), Some(OsStr::new(EMPTY_PATH)))
            .expect_err("a missing pinned compiler must fail");

        assert_eq!(
            error.to_string(),
            "CURSUS_LATEX=pdflatex is set but pdflatex was not found on PATH"
        );
    }
}
