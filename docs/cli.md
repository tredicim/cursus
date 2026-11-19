# CLI contract

This page is the stable reference for Cursus command output. The data format
and build pipeline are documented separately in [Data model](data-model.md) and
[Building](building.md).

The examples below were run from `examples/valid/` with
`cargo run -p cursus-cli --`. Paths in output use the platform's native
separator.

## Global JSON output

`--json` is a global flag. It can appear before or after the subcommand:

```sh
cargo run -p cursus-cli -- --json validate
cargo run -p cursus-cli -- validate --json
```

For commands with an envelope, a successful invocation has this shape:

```json
{
  "ok": true,
  "command": "<subcommand>",
  "data": {}
}
```

The `command` field is the parsed subcommand name, including when a command
fails. An error has this shape:

```json
{
  "ok": false,
  "command": "<subcommand>",
  "error": {
    "code": "<stable error code>",
    "message": "<human-readable message>"
  }
}
```

`error.details` is present only for `invalid_data`. Its `issues` array contains
the file, field path (when available), and reason for each validation issue.
Other errors omit `details`.

For example, an unknown target produces this actual error envelope:

```json
{"ok":false,"command":"build","error":{"code":"target_not_found","message":"target `nope` not found in targets/"}}
```

A validation failure includes the `issues` details:

```json
{"ok":false,"command":"validate","error":{"code":"invalid_data","message":"invalid data (1 issue(s)):\n  .\\targets\\target.yaml: include[0].variant: entry `only-default` has no variant `software-eng` (available: default)\n","details":{"issues":[{"field":"include[0].variant","file":".\\targets\\target.yaml","message":"entry `only-default` has no variant `software-eng` (available: default)"}]}}}
```

The `schema` command is an exception: it prints the JSON Schema itself, not an
envelope. `schema` prints the same schema with or without `--json`.

## Exit codes and error codes

The stable error codes and exit codes are defined by `CliError::code()` and
`CliError::exit_code()`:

| Exit code | Meaning | Error code |
| ---: | --- | --- |
| `0` | Success | — |
| `1` | Invalid project data or target not found | `invalid_data`, `target_not_found` |
| `2` | Command-line usage error reported by Clap | — |
| `3` | File-system I/O failure | `io_error` |
| `4` | LaTeX/PDF build failure | `build_failed` |

Usage errors are reported by Clap before a subcommand is run, so they do not
have a `CliError` code or JSON envelope.

## `validate`

### Synopsis

```text
cursus validate [--json]
```

Checks every entry and target in the current project. See the [validation
rules](data-model.md#validation) for the data-specific checks.

Human output:

```text
valid: 5 entries, 1 targets
```

JSON output:

```json
{"ok":true,"command":"validate","data":{"entries":5,"targets":1}}
```

## `build`

### Synopsis

```text
cursus build <target> [--tex-only] [--output <path>] [--json]
```

Renders a target to LaTeX and, unless `--tex-only` is set, compiles it to PDF.
See [Building](building.md) for output paths, compiler selection and
compilation behavior.

Human output (`--tex-only`):

```text
rendered acme-swe -> output\acme-swe.tex
```

JSON output (`--tex-only`):

```json
{"ok":true,"command":"build","data":{"target":"acme-swe","tex":"output\\acme-swe.tex"}}
```

## `schema`

### Synopsis

```text
cursus schema [--json]
```

Prints the draft 2020-12 JSON Schema for entry and target documents. The
complete schema is also committed as [`schema.json`](../schema.json).

Human output (beginning of the actual schema output):

```json
{
  "$defs": {
    "Entry": {
      "additionalProperties": false,
      "description": "One item in the career library.",
      "properties": {
        "end": {
          "anyOf": [
```

JSON output with `schema --json` is the same raw schema output; it is not
wrapped in the success envelope:

```json
{
  "$defs": {
    "Entry": {
      "additionalProperties": false,
```

`init` and `list` are planned additions to this reference. Their command
specific examples will be added when their follow-up CLI documentation lands.
