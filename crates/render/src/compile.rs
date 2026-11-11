//! Compile a rendered `.tex` file to PDF.
//!
//! Cursus does not bundle a TeX distribution. It uses, in order, a `tectonic`,
//! `latexmk` or `pdflatex` executable found on `PATH`. Tectonic is preferred
//! because it fetches a reproducible bundle on demand; `latexmk` and
//! `pdflatex` cover the TeX Live/MiKTeX installs most people already have.
//! `CURSUS_LATEX=tectonic|latexmk|pdflatex` pins one explicitly.

use std::env;
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

/// Name of the compiler Cursus would use, if one is installed.
pub fn compiler_name() -> Option<&'static str> {
    detect().map(|compiler| compiler.kind.name())
}

/// Whether a PDF can be compiled on this machine.
pub fn latex_available() -> bool {
    detect().is_some()
}

/// Compile `tex` and return the path of the PDF next to it.
///
/// `output_dir` is created if needed. Auxiliary files are removed afterwards;
/// the `.tex` and `.pdf` files stay.
pub fn compile_pdf(tex: &Path, output_dir: &Path) -> Result<PathBuf, CompileError> {
    let compiler = detect().ok_or(CompileError::NoCompiler)?;

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

fn detect() -> Option<Compiler> {
    if let Some(configured) = configured_compiler() {
        return Some(configured);
    }

    for kind in [Kind::Tectonic, Kind::Latexmk, Kind::PdfLatex] {
        if let Some(program) = find_in_path(kind.program()) {
            return Some(Compiler { kind, program });
        }
    }

    None
}

fn configured_compiler() -> Option<Compiler> {
    let value = env::var("CURSUS_LATEX").ok()?;
    let kind = match value.as_str() {
        "tectonic" => Kind::Tectonic,
        "latexmk" => Kind::Latexmk,
        "pdflatex" => Kind::PdfLatex,
        _ => return None,
    };
    let program = find_in_path(kind.program())?;

    Some(Compiler { kind, program })
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    for directory in env::split_paths(&path) {
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
