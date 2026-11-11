# Data model

Cursus keeps one career library in YAML and builds each CV from a *target* that
selects entries and variants. The Rust types in `crates/core/src/model.rs` are
the source of truth; the JSON Schema, validation and renderer all derive from
them.

## Why YAML

YAML wins over TOML for this workload:

- Entries contain nested maps and lists of maps (`variants`, `links`). TOML
  makes those verbose with `[[entries.variants]]` tables and repeated keys.
- Comments are useful in a career library ("why this framing?"); JSON has none.
- YAML is the format behind the examples in the original plan, and most editors,
  including the future desktop app, parse it without extra tooling.
- Git diffs stay readable, which matters because agents and humans edit the
  same files.

The trade-offs are real: YAML has sharp edges around indentation and unquoted
scalars, and the format has more types than the model needs. `serde` plus
`deny_unknown_fields` catches typos, and the schema lets editors catch them
before `cursus validate` runs. Dates use one explicit string format instead of
YAML timestamps so they behave the same in every parser.

## Layout

```
data/
  experience/acme-internship.yaml
  projects/cursus.yaml
targets/
  acme-swe.yaml
```

- Every file below `data/` contains exactly one entry.
- Every file below `targets/` contains exactly one target.
- Subdirectories are allowed and only used for organisation.
- The file name is the target's command-line ID (`acme-swe.yaml` becomes
  `acme-swe`). An entry's `id` field is the reference targets use.

`examples/valid/` is a complete project; `examples/invalid/` contains one broken
project per validation rule.

## Entry

```yaml
id: acme-internship
kind: experience
title: Software Engineering Intern
organization: Acme Corp
location: Remote
start: 2025-06
end: 2025-09
tags: [backend, python]
links:
  - label: Acme engineering blog
    url: https://example.com/acme-engineering
variants:
  software-eng:
    summary: Built and shipped backend services for the internal data platform.
    bullets:
      - Built a REST service handling 2k requests per second.
  data-science:
    summary: Analysed product usage data to prioritise platform work.
```

| Field | Required | Rules |
| --- | --- | --- |
| `id` | yes | Unique across the project; lowercase letters, digits and hyphens; starts with a letter or digit. |
| `kind` | yes | `experience`, `project`, `education`, `skill`, `award`, `certification` or `other`. |
| `title` | yes | Non-empty. |
| `organization` | no | Free text, e.g. company, university or client. |
| `location` | no | Free text, e.g. `Remote` or `Bologna, Italy`. |
| `start` | no | `YYYY-MM`, month `01`–`12`. |
| `end` | no | `YYYY-MM`, not before `start`. Omit for ongoing entries. |
| `tags` | no | Non-empty, unique strings. |
| `links` | no | `label` plus an absolute `http://` or `https://` URL. |
| `variants` | yes | At least one named framing. |

A variant contains an optional `summary` and an optional list of `bullets`; at
least one of the two must be present. Empty strings are rejected. A target that
omits `variant` uses the variant named `default`.

`variants` is a map, so the order in the file follows YAML order but the loader
sorts variant names for deterministic errors. Target order, not map order,
decides which entries appear in a CV.

## Target

```yaml
name: Acme Software Engineering Internship
template: classic
include:
  - id: acme-internship
    variant: software-eng
  - id: cursus
    variant: default
```

| Field | Required | Rules |
| --- | --- | --- |
| `name` | yes | Non-empty display name and document title. |
| `template` | yes | `classic` for now. |
| `include` | yes | At least one entry, no duplicate entry IDs, every reference must exist. |
| `include[].id` | yes | Must match an entry `id`. |
| `include[].variant` | no | Must exist on the referenced entry; defaults to `default`. |

The target file name must follow the same naming rules as entry IDs.

## Validation

`cursus validate` loads every YAML document below `data/` and `targets/`, then
checks:

1. The YAML parses into the model with no unknown fields.
2. IDs and target file names are valid and unique.
3. Titles, names, tags, links and variant content are non-empty and well formed.
4. Date ranges are ordered.
5. Targets only reference existing entries and variants, and include each entry
   once.

Every issue carries the file, the field path (`include[0].variant`,
`variants.default.bullets[2]`) and a reason. Human output goes to stderr with
exit code `1`; `--json` puts the same issues under
`error.details.issues`:

```json
{
  "ok": false,
  "command": "validate",
  "error": {
    "code": "invalid_data",
    "message": "invalid data (1 issue(s)):\n  ...",
    "details": {
      "issues": [
        {
          "file": "targets/acme-swe.yaml",
          "field": "include[0].variant",
          "message": "entry `acme-internship` has no variant `backend` (available: software-eng)"
        }
      ]
    }
  }
}
```

On success the exit code is `0` and `--json` reports counts:

```json
{"ok":true,"command":"validate","data":{"entries":5,"targets":1}}
```

## JSON Schema

`schema.json` is generated from the Rust types and committed at the repository
root. `cursus schema` prints the same document to stdout, with or without
`--json`, so an editor or agent can save it directly:

```sh
cargo run -p cursus-cli -- schema > schema.json
```

The schema is draft 2020-12 and describes both entry and target documents.
`crates/core/tests/project.rs` validates every fixture in `examples/valid/`
against it with an external validator (the `jsonschema` crate) and fails if the
committed schema is stale.

## Adding an entry kind

Add the variant to `EntryKind` in `crates/core/src/model.rs` and give it a
label. Validation is kind-agnostic and templates render sections from the
label, so no other match needs to change. Renderers that want a special layout
for a kind can opt in later without touching the data format.
