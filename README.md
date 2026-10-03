# rustxform

A fast, dependency-light **Rust** library and CLI that compiles an **XLSForm**
(a spreadsheet-based form definition) into an **XForm** — the XML form format
consumed by mobile and web data-collection tools.

> **Status:** 🟢 v1 feature-complete. Reads Markdown, CSV and XLSX/XLS and
> compiles question types, choices, groups, repeats, `${…}` logic,
> multilingual `itext` and validations — each verified against golden output
> (see [ROADMAP.md](ROADMAP.md)). Some advanced columns remain (roadmap).

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
| `rustxform`          | End-to-end facade (`convert_markdown`) + tests        |
| `rustxform-cli`      | Command-line interface                                 |

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
`crates/rustxform/tests/fixtures/`.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Unless you explicitly state otherwise, any contribution
intentionally submitted for inclusion in this project, as defined in the
Apache-2.0 license, shall be dual-licensed as above, without any additional
terms or conditions.
