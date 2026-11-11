# Templates

A template turns a resolved target into a complete LaTeX document. The first
one is `classic`; the type lives in `crates/core/src/model.rs` and the renderer
in `crates/render/src/classic.rs`.

## Classic

`classic` is intentionally plain: one column, black text, standard fonts, no
tables, icons or graphics. Applicant tracking systems parse it reliably and it
still prints well.

File: `crates/render/templates/classic.tex`

```tex
\documentclass[11pt]{article}
\usepackage[margin=1in]{geometry}
\usepackage[hidelinks]{hyperref}
% ...
\begin{document}
\begin{center}
{\LARGE\bfseries %%CURSUS_TITLE%%}
\end{center}
%%CURSUS_BODY%%
\end{document}
```

The renderer replaces two tokens:

| Token | Value |
| --- | --- |
| `%%CURSUS_TITLE%%` | Escaped target `name`. |
| `%%CURSUS_BODY%%` | Generated sections, entries, bullets and links. |

Everything outside the tokens is trusted template code. Everything inside the
tokens comes from user data and is escaped first.

## Document structure

Entries are grouped by `kind`, in the order the kind first appears in the
target. The order inside a section follows the target. The section heading is
`EntryKind::label()`; entries are independent of the kind otherwise.

Each entry renders:

1. Title, with a date range on the right.
2. Organization and location, separated by a bullet, when present.
3. The selected variant's `summary`, when present.
4. The selected variant's `bullets`, in order.
5. Links as hyperlinked labels, when present.

Dates are month names: `2025-06` becomes `June 2025`. A missing `end` renders as
`Present` for experience, education and projects, and as a single date for
awards, certifications, skills and other kinds.

## Escaping

`escape()` in `crates/render/src/classic.rs` handles the ten LaTeX special
characters, plus newlines:

| Input | Output |
| --- | --- |
| `&` | `\&` |
| `%` | `\%` |
| `$` | `\$` |
| `#` | `\#` |
| `_` | `\_` |
| `{` | `\{` |
| `}` | `\}` |
| `~` | `\textasciitilde{}` |
| `^` | `\textasciicircum{}` |
| `\` | `\textbackslash{}` |
| newline | space |

User data never becomes LaTeX syntax. `examples/valid/data/projects/cursus.yaml`
contains all ten characters and the golden test keeps them covered.

## Adding a template

1. Add a variant to `Template` in `crates/core/src/model.rs` and its `id()`.
2. Add `crates/render/src/<name>.rs` with a `render_<name>` function.
3. Add `crates/render/templates/<name>.tex` and include it with `include_str!`.
4. Match the new variant in `render()` in `crates/render/src/classic.rs` (or
   move the dispatch into `lib.rs` when there is more than one).
5. Add a resolved target and a golden file under `crates/render/tests/golden/`.
6. Document the theme in this file and mention any extra LaTeX packages.

Templates are plain functions over `ResolvedTarget`, not a general-purpose
templating language. That keeps output deterministic, avoids a template engine
dependency and makes escaping auditable. If a theme needs conditional layout,
write it in Rust and keep the `.tex` file to tokens and boilerplate.
