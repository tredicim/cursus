# Cursus: plan

Working document. Everything here is a draft and can change.

## Goals

1. Keep all career data in one structured place.
2. Build a tailored CV for a specific job in minutes, reproducibly.
3. Produce clean LaTeX and a PDF that applicant tracking systems can parse.
4. Work well with coding agents (Claude Code, opencode) without depending on them.
5. Ship as a downloadable desktop app for people who prefer clicking to editing YAML.

## Non-goals (for now)

- A hosted service. Everything runs locally and your data stays in your files.
- Built-in LLM calls or API keys. The agent lives outside the tool.

## Core concepts

| Concept | Meaning |
| --- | --- |
| **Entry** | One item: an experience, project, education, skill group, award... |
| **Variant** | A way of writing an entry (e.g. `data-science`, `software-eng`). |
| **Target** | One CV to build: selected entries, chosen variants, order, template. |
| **Template** | A LaTeX theme that turns a resolved target into a `.tex` document. |

## Data model (sketch)

```yaml
# data/experiences/acme-internship.yaml
id: acme-internship
kind: experience
company: Acme Corp
role: Software Engineering Intern
start: 2025-06
end: 2025-09
tags: [backend, python]
variants:
  software-eng:
    bullets:
      - Built a REST service handling 2k req/s ...
  data-science:
    bullets:
      - Trained and evaluated a ranking model ...
```

```yaml
# targets/acme-swe.yaml
name: Acme SWE internship
template: classic
include:
  - id: acme-internship
    variant: software-eng
  - id: thesis-project
    variant: default
```

## CLI (sketch)

```
cursus init                      # scaffold data/ and targets/
cursus validate                  # check data against the schema
cursus list [--tag backend]      # list entries
cursus build <target>            # render LaTeX and compile to PDF
cursus schema                    # print JSON Schema (for agents)
cursus ats check <pdf> [--job job.txt]   # later
```

All commands support `--json` so agents can read results reliably.

## Architecture

Cargo workspace:

- `core`: data model, validation, target resolution.
- `render`: templating and LaTeX generation (including escaping of special characters).
- `cli`: the `cursus` binary.
- `app`: Tauri desktop app (Rust side: commands, file watching, PDF build).
- `ui`: frontend (TypeScript) running inside the app.
- `ats` (later): text extraction from PDF and checks.

## Agent integration

- The tool does not call any LLM. The agent reads the job posting, edits
  `targets/*.yaml` (and adds variants if needed), then runs `cursus build`.
- Ship an `AGENTS.md` / `CLAUDE.md` that explains the data layout and commands.
- Publish the JSON Schema so edits can be validated before building.

## UI

The main way to use Cursus is a downloadable desktop app built with Tauri: a
Rust core with a TypeScript frontend, nothing to set up in a browser. The CLI
stays for scripting and agents.

Planned screens:

- **Library:** browse and edit entries and their variants.
- **Target builder:** tick which entries to include, pick a variant for each,
  reorder, choose a template.
- **Preview:** the real compiled PDF next to the editor, refreshed on change.
- **Job posting:** a box to paste a posting (used by agents and, later, the ATS panel).

Design rule: the UI never owns the data. It reads and writes the same YAML files
the CLI and agents use, and watches the files so edits from any of them show up
live. This keeps everything git-friendly and avoids a separate database.

## Milestones

- **M0: Repo.** README, PLAN, license, CI skeleton.
- **M1: Data.** Data model, YAML loading, `validate`, JSON Schema.
- **M2: Render.** One LaTeX template, `build` to `.tex`, then PDF.
- **M3: Targets.** Selection and variants, ordering, `list`.
- **M4: App.** Tauri shell, library and target builder screens, PDF preview.
- **M5: Agents.** `AGENTS.md`, `--json` everywhere, example workflow.
- **M6: ATS.** PDF text extraction, structure checks, keyword coverage.
- **M7: Release.** Installers for macOS, Windows and Linux, signing, auto-update.
- **Later.** More templates, language support (EN/IT), import from existing LaTeX.

## Open questions

- YAML or TOML for data files?
- Bundle an engine such as Tectonic so the app works without a TeX install, or require a system LaTeX?
- Templating engine: Tera, minijinja, or Handlebars?
- How to handle a profile photo and personal data safely (keep it out of git)?
- How should "same entry, different framing" be diffed so it stays easy to review?

- Frontend stack inside Tauri: Svelte or React?
- Name: `cursus` is already taken on crates.io. Irrelevant while it ships as an app; revisit if the core is published as a crate.
- Preview: compile the real PDF each time (accurate, 1 to 3 s) or a faster approximate HTML preview?

## Risks

- **Distribution:** installers for three OSes, code signing and notarization, OS security warnings.
- **LaTeX escaping and templates** are fiddly; budget time for them.
- **Scope creep** (ATS, UI polish, AI features). Ship M1 to M3 first, then the UI.
- **Two interfaces over one dataset** (UI and agents editing the same files) needs careful file watching and validation.
