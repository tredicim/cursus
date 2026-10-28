# Cursus

> Write your career once. Tailor it for every job.

A modular, local-first CV builder that renders to LaTeX. You keep all your
experiences, projects, skills and education in one place, then pick exactly what
goes into each CV, and which *framing* of it, for every application.

**Status:** planning. Nothing to install yet. See [PLAN.md](PLAN.md).

## The problem

Every application needs a slightly different CV. Today that usually means
copying old LaTeX files, pasting a job posting into a chatbot, and hoping the
result is consistent. It works, but it is manual, error-prone, and nothing is
reusable.

## The idea

- **One source of truth.** Experiences, projects, skills and education live in
  structured data files, not inside a `.tex` file.
- **Variants.** The same experience can be written several ways, e.g. a
  *data scientist* framing and a *software engineer* framing.
- **Targets.** A target describes one CV: which entries to include, which
  variant of each, in what order, with which template.
- **A desktop app.** Edit entries, build targets and preview the PDF in a
  downloadable app, working on the same files the CLI uses.
- **LaTeX out.** A target is rendered through a template into a LaTeX document
  and compiled to PDF.
- **Agent-friendly.** A small CLI with machine-readable output and a published
  schema, so tools like Claude Code or opencode can tailor a CV to a job posting
  by editing data and running commands, instead of rewriting LaTeX by hand.

## Planned features

- [ ] Data model and validation
- [ ] Targets (selection of entries and variants)
- [ ] LaTeX templates and PDF build
- [ ] Desktop app (library, target builder, live PDF preview)
- [ ] Agent integration (schema, `AGENTS.md`, JSON output)
- [ ] ATS-style checker (text extraction, structure, keyword coverage)

## Tech

A Rust core shared by a desktop app (Tauri, TypeScript UI) and a CLI for scripting and agents. Details and open questions in [PLAN.md](PLAN.md).

## License

To be decided.
