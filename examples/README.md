# Examples

Fixtures used by the tests and by anyone trying Cursus.

## `valid/`

A complete project:

```
valid/
  data/experience/acme-internship.yaml
  data/projects/cursus.yaml
  data/education/university.yaml
  data/skills/languages.yaml
  data/awards/hackathon.yaml
  targets/acme-swe.yaml
```

The data is synthetic. The `cursus` project deliberately contains every
character that is special in LaTeX so the escaping and golden tests stay
honest.

Build it from this directory:

```sh
cd examples/valid
cargo run -p cursus-cli -- validate
cargo run -p cursus-cli -- build acme-swe
```

The PDF and `.tex` land in `examples/valid/output/`, which is ignored by Git.

## `invalid/`

One directory per broken rule. Each is a project root, so the CLI can validate
it in place:

```sh
cd examples/invalid/duplicate-id
cargo run -p cursus-cli -- validate
```

| Fixture | Rule |
| --- | --- |
| `bad-date` | Months must be `01`–`12`. |
| `bad-link` | Links need an absolute `http://` or `https://` URL. |
| `duplicate-id` | Entry IDs are unique. |
| `missing-entry` | Targets may only include existing entries. |
| `tag-whitespace` | Tags must not have leading or trailing whitespace. |
| `unknown-field` | Unknown YAML fields are rejected. |
| `unknown-variant` | Targets may only select variants the entry defines. |

`crates/core/tests/project.rs` checks that every issue names the file, the field
and the reason.
