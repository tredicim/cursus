# Contributing to Cursus

Thanks for taking the time to contribute. Cursus is early, so there is plenty
of room to shape the data model, the LaTeX output and the desktop app.

This guide covers the local setup, the conventions we use and the pull request
flow. It intentionally stays short: [PLAN.md](PLAN.md) explains the product and
the roadmap, [README.md](README.md) explains what Cursus is.

By participating you agree to follow our
[Code of Conduct](CODE_OF_CONDUCT.md).

## Before you start

- Search the [open issues](https://github.com/tredicim/cursus/issues) before
  opening a new one.
- For anything larger than a typo, open or claim an issue first and say what you
  plan to do. This avoids two people working on the same thing.
- Issues labelled `good first issue` are a good entry point. Issues labelled
  `help wanted` are ready for a contributor. The `area:` labels show which part
  of the project they touch.
- Security problems do **not** go through public issues. See
  [SECURITY.md](SECURITY.md).

## Project layout

Cursus is a Cargo workspace. The crates are split so the CLI and the desktop
app can share the same logic.

| Path | Purpose |
| --- | --- |
| `crates/core` | Data model, YAML loading, validation, target resolution. |
| `crates/render` | LaTeX rendering, escaping and the PDF build pipeline. |
| `crates/cli` | The `cursus` binary; integration tests live in `crates/cli/tests/`. |
| `data/` | Your entries. Created by `cursus init` (still planned). |
| `targets/` | One file per CV target. |
| `examples/` | A complete example project and one fixture per validation rule. |
| `docs/` | Data model, build pipeline and template notes. |
| `schema.json` | Generated JSON Schema for entries and targets. |
| `.github/` | CI workflows, issue forms and the pull request template. |
| `PLAN.md` | Goals, milestones, open questions and risks. |

`validate`, `build` and `schema` work. `init` and `list` are still stubs; keep
changes aligned with the milestones in `PLAN.md`.

## Prerequisites

- [Git](https://git-scm.com/).
- [Rust](https://rustup.rs/). The repository pins the toolchain in
  `rust-toolchain.toml`; `rustup` installs and selects it automatically.
- Optional: Tectonic, `latexmk` or `pdflatex` to compile PDFs locally. The
  golden LaTeX tests run without one; the PDF test skips. See
  [docs/building.md](docs/building.md).
- Optional: [`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny) for the
  dependency check that CI runs.

Check the toolchain:

```sh
rustc --version
cargo --version
```

## Set up the workspace

```sh
git clone https://github.com/tredicim/cursus.git
cd cursus
cargo build
cargo run -p cursus-cli -- --help
```

## Run the checks

Run the same checks as CI before you push:

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

`cargo fmt --all` applies formatting. `cargo-deny` is optional locally but not
in CI. If it is not installed:

```sh
cargo install cargo-deny --locked
```

CI runs the first three commands on Ubuntu, macOS and Windows, plus
`cargo-deny` on Ubuntu and a Conventional Commits check on the pull request
title. All of them must be green before a pull request can merge.

## Branches

Branch from an up-to-date `main` and use one of these prefixes:

| Prefix | Use for |
| --- | --- |
| `docs/...` | Documentation, README, guides. |
| `feat/...` | A new capability. |
| `fix/...` | A bug fix. |
| `chore/...` | Tooling, dependencies, CI and other maintenance. |

Examples: `feat/validate-yaml`, `fix/latex-escaping`, `docs/contributing`,
`chore/bump-clap`.

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/) with the same
types as the pull request title check: `feat`, `fix`, `docs`, `refactor`,
`perf`, `test`, `ci`, `build` and `chore`.

```text
feat: add YAML entry loading
fix: escape underscores in entry titles
docs: explain target resolution
refactor: split validation from parsing
test: cover duplicate entry ids
ci: cache cargo artifacts
chore: bump clap
```

Guidelines:

- Write the subject in the imperative mood, lower case, without a trailing
  period.
- Keep commits small and focused. One logical change per commit.
- The body explains **why**, not just what.
- Do not commit generated output (`target/`), PDFs or personal CV data.

## Pull requests

1. Open a pull request from your branch to `main`.
2. Use a Conventional Commits title, exactly like the commit subject.
3. Link the issue in the description (`Closes #12`) or say `Part of #2` for a
   sub-issue. The pull request template asks for this.
4. Keep the pull request small. If it grows beyond one issue, split it.
5. Fill in the test plan with the exact commands you ran and what you checked
   manually.
6. Wait for all checks and one approving review. Resolve every conversation
   before merging.
7. We squash-merge, so the pull request title becomes the commit on `main`.
   Delete the branch afterwards.

`main` is protected: do not push to it directly. Every change goes through a
pull request, the CI checks and review. Force pushes and branch deletion are
blocked.

## Reviews

- Be kind and specific. Review the change, not the person.
- Maintainers may ask you to split a pull request or add tests; that is normal.
- If a review comment is unclear, ask for an example.
- Once you address a comment, reply and resolve the thread.

## Style

- Rust: `rustfmt` defaults and no Clippy warnings. Prefer small modules and
  explicit error types with `thiserror`.
- Tests: unit tests next to the code, CLI behaviour in
  `crates/cli/tests/cli.rs`. New behaviour needs a test.
- Documentation: short sentences, explain the reason behind a decision, and
  update `PLAN.md` when a milestone or open question changes.
- User-facing output: keep the human text readable and the `--json` envelope
  stable. Agents depend on the error codes.
- Fixtures: keep them synthetic and free of personal data.

## First contribution walkthrough

1. Pick a `good first issue` and comment that you are working on it.
2. Create a branch:

   ```sh
   git switch main
   git pull --ff-only
   git switch -c docs/fix-typo
   ```

3. Make the change, then run the checks:

   ```sh
   cargo fmt --all
   cargo fmt --all --check
   cargo clippy --all-targets -- -D warnings
   cargo test --workspace
   ```

4. Commit and push:

   ```sh
   git add README.md
   git commit -m "docs: fix typo in README"
   git push -u origin docs/fix-typo
   ```

5. Open the pull request, fill in the template, and wait for CI.
6. Address review comments, then a maintainer squash-merges it.

## Questions

Open an issue with the `question` label, or email
[info@saajansaini.com](mailto:info@saajansaini.com) for anything that should not
be public.
