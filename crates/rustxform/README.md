# rustxform

Compile an [XLSForm](https://xlsform.org/) into an
[XForm](https://getodk.github.io/xforms-spec/) — the end-to-end facade of the
`rustxform` workspace, and the crate most users want. It wires the pipeline
together so you don't have to touch the individual stages.

```text
            ┌────────┐   ┌───────┐   ┌──────────┐   ┌───────┐
 XLSForm ──▶│ reader │──▶│ parse │──▶│ validate │──▶│ xform │──▶ XForm XML
            └────────┘   └───────┘   └──────────┘   └───────┘
```

## Install

```toml
[dependencies]
rustxform = "0.1"
```

For the command-line tool, see
[`rustxform-cli`](https://crates.io/crates/rustxform-cli)
(`cargo install rustxform-cli`).

## API

Two families, chosen by whether you want validation and warnings:

| Want | Functions |
| --- | --- |
| Just the XForm | `convert_markdown` · `convert_csv` · `convert_xlsx` → `Result<String, ConvertError>` |
| Validate, then the XForm | `convert_markdown_checked` · `convert_csv_checked` · `convert_xlsx_checked` → `Result<String, BuildError>` |
| Validate, with warnings | `build_markdown` · `build_csv` · `build_xlsx` → `Result<Built, BuildError>` |

`Built { xform: String, warnings: Vec<Warning> }`. `BuildError::Invalid(Vec<ValidationError>)`
carries every blocking problem at once. `ValidationError` and `Warning` are
re-exported from
[`rustxform-validate`](https://crates.io/crates/rustxform-validate).

## Examples

Quick conversion:

```rust
let xform = rustxform::convert_markdown("\
| survey |         |      |       |
|        | type    | name | label |
|        | text    | q    | Q     |
").unwrap();
assert!(xform.contains("<h:html"));
```

Validated build with warnings:

```rust
use rustxform::{build_markdown, BuildError};

match build_markdown(source) {
    Ok(built) => {
        for warning in &built.warnings {
            eprintln!("warning: {warning}");
        }
        println!("{}", built.xform);
    }
    Err(BuildError::Invalid(errors)) => {
        for error in &errors {
            eprintln!("error: {error}");
        }
    }
    Err(other) => eprintln!("error: {other}"),
}
```

XLSX bytes:

```rust
let bytes = std::fs::read("form.xlsx").unwrap();
let xform = rustxform::convert_xlsx(&bytes).unwrap();
```

## How it compares

`rustxform` is verified **byte-identical** (after XML C14N canonicalization) to
the reference XLSForm compiler on a golden corpus of 53 forms. See the
[repository](https://github.com/horacioskrp/rustxform) for the conformance
harness, roadmap and changelog.

## Workspace crates

The stages are published independently if you need finer control:
[`-reader`](https://crates.io/crates/rustxform-reader) ·
[`-parse`](https://crates.io/crates/rustxform-parse) ·
[`-expr`](https://crates.io/crates/rustxform-expr) ·
[`-validate`](https://crates.io/crates/rustxform-validate) ·
[`-xform`](https://crates.io/crates/rustxform-xform) ·
[`-core`](https://crates.io/crates/rustxform-core) ·
[`-xform2json`](https://crates.io/crates/rustxform-xform2json) (reverse).

## License

Licensed under either of [Apache-2.0](../../LICENSE-APACHE) or
[MIT](../../LICENSE-MIT) at your option.
