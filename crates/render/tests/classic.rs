use std::fs;
use std::path::{Path, PathBuf};

use cursus_core::{Project, ResolvedTarget};
use cursus_render::{compile_pdf, escape, latex_available, render};

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn resolved() -> ResolvedTarget {
    let project = Project::load(examples().join("valid")).expect("load valid project");
    let target = project.find_target("acme-swe").expect("find target");

    project.resolve(target).expect("resolve target")
}

#[test]
fn escapes_every_latex_special_character() {
    assert_eq!(
        escape(r"& % $ # _ { } ~ ^ \"),
        r"\& \% \$ \# \_ \{ \} \textasciitilde{} \textasciicircum{} \textbackslash{}"
    );
    assert_eq!(escape("line one\nline two\r\n"), "line one line two  ");
    assert_eq!(
        escape("plain text, Unicode: àèì"),
        "plain text, Unicode: àèì"
    );
}

#[test]
fn rendering_is_deterministic() {
    let target = resolved();

    assert_eq!(render(&target).unwrap(), render(&target).unwrap());
}

#[test]
fn classic_template_matches_the_golden_file() {
    let rendered = render(&resolved()).unwrap();
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/acme-swe.tex");

    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        fs::write(&golden, &rendered).expect("update golden file");
        return;
    }

    let expected = fs::read_to_string(&golden).expect("read golden file");
    assert_eq!(
        rendered, expected,
        "rendered LaTeX changed; run `UPDATE_GOLDEN=1 cargo test -p cursus-render` to update the golden file"
    );
}

#[test]
fn rendered_document_contains_escaped_user_data() {
    let rendered = render(&resolved()).unwrap();

    assert!(rendered.contains("Cursus: a CV builder for \\{LaTeX\\}"));
    assert!(rendered.contains("100\\% deterministic LaTeX \\& PDFs"));
    assert!(rendered.contains("\\textbackslash{} are escaped"));
    assert!(rendered.contains("\\$\\#\\_"));
    assert!(rendered.contains("\\cvsection{Experience}"));
    assert!(rendered.contains("\\cvsection{Projects}"));
}

#[test]
fn compiles_a_pdf_when_a_latex_compiler_is_available() {
    if !latex_available() {
        eprintln!("skipping PDF compilation: no LaTeX compiler on PATH");
        return;
    }

    let directory = tempfile::tempdir().expect("create temp directory");
    let tex = directory.path().join("acme-swe.tex");
    fs::write(&tex, render(&resolved()).unwrap()).expect("write tex");

    let pdf = compile_pdf(&tex, directory.path()).expect("compile PDF");
    let bytes = fs::read(&pdf).expect("read PDF");

    assert!(bytes.starts_with(b"%PDF"), "output is not a PDF");
    assert!(
        !directory.path().join("acme-swe.aux").exists(),
        "auxiliary files should be removed"
    );
}
