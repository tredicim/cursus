# Building a PDF

`cursus build <target>` renders a target to LaTeX, writes the `.tex` file, then
compiles it to PDF.

```sh
cd examples/valid
cargo run -p cursus-cli -- build acme-swe
```

```
built acme-swe -> output/acme-swe.pdf
```

Both files stay in the output directory: `acme-swe.tex` for inspection and
`acme-swe.pdf` for the application. Auxiliary files (`.aux`, `.log`, `.fls`,
`latexmk` state) are removed after a successful build.

## Output path

- Default: `output/<target>.pdf` next to the project root.
- `--output <directory>`: writes `<directory>/<target>.pdf`.
- `--output <file>.pdf`: writes exactly that file.

The `.tex` file uses the same path with a `.tex` extension. Parent directories
are created. `--json` reports both paths:

```json
{
  "ok": true,
  "command": "build",
  "data": {
    "target": "acme-swe",
    "tex": "output/acme-swe.tex",
    "pdf": "output/acme-swe.pdf"
  }
}
```

## Compiler choice

Cursus does not bundle a TeX distribution. It finds, in order:

1. `tectonic`
2. `latexmk -pdf`
3. `pdflatex`

`CURSUS_LATEX=tectonic`, `CURSUS_LATEX=latexmk` or `CURSUS_LATEX=pdflatex`
pins one when several are installed.

This is a deliberate trade-off:

- **Tectonic first.** It is a single executable that downloads a pinned bundle,
  so the toolchain is easy to install and does not depend on a system TeX
  layout. A user who already has it gets reproducible builds without extra
  configuration.
- **TeX Live and MiKTeX next.** Most LaTeX users already have `pdflatex` or
  `latexmk`; requiring Tectonic anyway would duplicate hundreds of megabytes.
- **No library dependency.** Linking a TeX engine into Cursus would make the
  binary and the dependency tree much larger, and the desktop app can reuse a
  system compiler just as easily.
- **The `.tex` is the contract.** Rendering is independent of the compiler, so
  golden tests cover the deterministic document and the PDF step is a thin
  adapter.

Install one of:

```sh
# macOS
brew install tectonic
# or a full TeX distribution
brew install --cask mactex-no-gui

# Debian/Ubuntu
sudo apt install texlive-latex-recommended latexmk
# or
cargo install tectonic

# Windows
winget install tectonic  # or install MiKTeX/TeX Live
```

The classic template only needs `geometry` and `hyperref`, both part of every
standard distribution and the default Tectonic bundle.

## Failures

- Exit code `4` (`build_failed`) means the compiler was missing or LaTeX failed.
  The error includes the tail of the compiler output.
- Exit code `1` (`invalid_data` or `target_not_found`) means the project did not
  validate, or the target name does not exist. Nothing is written in that case.
- Exit code `3` (`io_error`) means a file or directory could not be read or
  written.

The `.tex` file is written before compilation, so if LaTeX fails you can inspect
the exact source and run the compiler by hand:

```sh
pdflatex -interaction=nonstopmode -halt-on-error output/acme-swe.tex
```

## Determinism and tests

Rendering contains no timestamps, absolute paths or environment-dependent
ordering. `crates/render/tests/classic.rs` checks:

- the ten LaTeX special characters are escaped;
- rendering the same target twice is byte-identical;
- the output matches `crates/render/tests/golden/acme-swe.tex`;
- a PDF starts with `%PDF` when a compiler is available.

CI does not install LaTeX. The golden `.tex` test always runs; the PDF test
skips with a message when no compiler is present. Run it locally with any engine
installed to cover the whole pipeline.

To update the golden file after an intentional template change:

```sh
UPDATE_GOLDEN=1 cargo test -p cursus-render classic_template_matches_the_golden_file
```
