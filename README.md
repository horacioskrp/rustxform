# rustxform

A fast, dependency-light **Rust** library and CLI that compiles an **XLSForm**
(a spreadsheet-based form definition) into an **XForm** — the XML form format
consumed by mobile and web data-collection tools.

> **Status:** 🟢 **v0.1.0 released.** Reads Markdown, CSV and XLSX/XLS and
> compiles question types, selects, groups, repeats, `${…}` logic, multilingual
> `itext`, media, metadata/actions, Entities and validations — verified
> byte-identical to the reference compiler on a 53-form golden corpus (see
> [CHANGELOG.md](CHANGELOG.md) and [ROADMAP.md](ROADMAP.md)).

## Why

XLSForm is a convenient, human-friendly way to author complex forms — nested
groups, repeats, skip logic, constraints, calculations, multilingual labels — in
an ordinary spreadsheet. Those spreadsheets are compiled into XForms before they
can run on devices. `rustxform` performs that compilation as a small native
binary with no runtime dependencies, suitable for embedding in services and
build pipelines.

## What it does

```
.xlsx / .xls / .csv / .md   →   XForm XML
      (XLSForm)                  (W3C-derived form definition)
```

The compiler runs in three stages:

1. **Read** the workbook into rows (Markdown tables first; XLSX/XLS/CSV next).
2. **Parse** the rows into a normalized survey model (types, choices, groups,
   repeats, settings).
3. **Emit** the XForm XML: the model, bindings and body controls.

## Install

```bash
cargo install --path crates/rustxform-cli
```

## Usage

```bash
rustxform form.md form.xml      # compile an XLSForm to an XForm
rustxform form.md               # writes form.xml next to the input
```

As a library:

```rust
let xform = rustxform::convert_markdown(markdown_source)?;
```

## Workspace layout

| Crate                | Responsibility                                        |
| -------------------- | ----------------------------------------------------- |
| `rustxform-core`     | Data model: `Survey`, `Question`, `Settings`          |
| `rustxform-reader`   | Read a workbook into rows (Markdown; XLSX/XLS/CSV)    |
| `rustxform-parse`    | Normalize rows into the survey model                  |
| `rustxform-expr`     | Tokenize expressions, rewrite `${ref}` into XPath     |
| `rustxform-xform`    | Emit XForm XML                                         |
| `rustxform-validate` | Form validations                                      |
| `rustxform-xform2json` | Reverse: parse an XForm back into the survey model  |
| `rustxform`          | End-to-end facade (`convert_markdown`) + tests        |
| `rustxform-cli`      | Command-line interface (`--json`, `--odk-validate`)   |

## Build & test

```bash
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
```

> **Note for some Windows hosts.** If Smart App Control / WDAC blocks the native
> build (`os error 4551`) or no linker is available, build inside the Rust Docker
> image — `scripts/docker-dev.ps1` wraps `cargo` with cached volumes:
>
> ```powershell
> ./scripts/docker-dev.ps1 test
> ```

## Conformance

Correctness is driven by golden tests: an input form is compiled and the result
is compared to an expected XForm after XML canonicalization (attribute order and
insignificant whitespace are ignored). Fixtures live under
`crates/rustxform/tests/fixtures/` as `<name>.md` (input) + `<name>.xml`
(golden) pairs, and `cargo test` compiles every pair and checks it — no Python
needed to run the tests.

### Regenerating the corpus

The golden `.xml` files are produced by the reference compiler
([pyxform](https://github.com/XLSForm/pyxform)) and committed; a pair is only
written if, after C14N canonicalization, it is byte-identical to rustxform's own
output. rustxform cannot be its own oracle, so regeneration depends on pyxform +
lxml (Python). The committed `scripts/gen_corpus.py` drives it; see its module
docstring for the exact two-step Docker commands (build the CLI binary, then run
the script with `RUSTXFORM_BIN` pointing at it). Day to day you never need this —
only when adding or updating corpus forms.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Unless you explicitly state otherwise, any contribution
intentionally submitted for inclusion in this project, as defined in the
Apache-2.0 license, shall be dual-licensed as above, without any additional
terms or conditions.
