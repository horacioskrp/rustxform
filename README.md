# rustxform

[![crates.io](https://img.shields.io/crates/v/rustxform.svg)](https://crates.io/crates/rustxform)
[![docs.rs](https://img.shields.io/docsrs/rustxform)](https://docs.rs/rustxform)
[![CI](https://github.com/horacioskrp/rustxform/actions/workflows/ci.yml/badge.svg)](https://github.com/horacioskrp/rustxform/actions/workflows/ci.yml)
[![license](https://img.shields.io/crates/l/rustxform.svg)](#license)

`rustxform` is a Rust library and command-line tool that converts spreadsheets
following the [XLSForm standard](https://xlsform.org/) into
[XForms](https://getodk.github.io/xforms-spec/) — the XML form definition
consumed by mobile and web data-collection tools. It is a small native binary
with no runtime dependencies, suitable for embedding in services and build
pipelines.

## Project status

🟢 **v0.2.0 released.** `rustxform` compiles a broad XLSForm surface, verified
byte-identical to the reference compiler on a 34-form golden corpus, and reverses
XForms back to the model (`xform2json`) with a byte-identical round-trip property
test (see [Conformance](#conformance) and [CHANGELOG.md](CHANGELOG.md)).

Current goals:

- Broaden coverage of advanced widgets and validation messages.
- Keep strict parity with the reference compiler as it evolves.
- Grow the reverse direction (`xform2json`) toward full round-tripping.

It is part of the DCOLLECT effort to build data-collection tooling in Rust.

## Using `rustxform`

There are three main ways to use `rustxform`:

- The command-line tool `rustxform`, helpful for troubleshooting or as part of a
  form-creation pipeline.
- As a library, imported by another Rust project.
- Embedded in a service (the compiler is pure Rust, no external runtime).

### Running the latest release

Install the CLI with Cargo:

    cargo install --git https://github.com/horacioskrp/rustxform rustxform-cli

Then compile a form (the reader is chosen by extension — `.md`, `.csv`,
`.xlsx`, `.xls`):

    rustxform path_to_form.xlsx [output_path.xml]
    rustxform form.md            # writes form.xml next to the input
    rustxform form.xlsx out.xml --json          # machine-readable report
    rustxform form.xlsx out.xml --odk-validate ODK-Validate.jar

The minimum supported Rust version is 1.85 (edition 2024).

### Running from local source

    # Get a copy of the repository.
    git clone https://github.com/horacioskrp/rustxform.git
    cd rustxform

    # Build and run the CLI.
    cargo run -p rustxform-cli -- form.md form.xml

### As a library

Add the facade crate and call one of the `convert_*` functions:

```rust
let xform = rustxform::convert_markdown(markdown_source)?;
let xform = rustxform::convert_xlsx(&bytes)?;

// Validate before emitting, and surface warnings:
let built = rustxform::build_markdown(markdown_source)?;
println!("{}", built.xform);
for warning in &built.warnings {
    eprintln!("warning: {warning}");
}
```

## Development

Build, test, lint and format with the standard Cargo commands:

    cargo build
    cargo test
    cargo clippy --all-targets -- -D warnings
    cargo fmt --all --check

> **Note for some Windows hosts.** If Smart App Control / WDAC blocks the native
> build (`os error 4551`) or no linker is available, build inside the Rust
> Docker image — `scripts/docker-dev.ps1` wraps `cargo` with cached volumes:
>
> ```powershell
> ./scripts/docker-dev.ps1 test
> ```

### Writing tests

Correctness is driven by a golden corpus. Each fixture is a pair under
`crates/rustxform/tests/fixtures/`: `<name>.md` (the XLSForm, written as a
Markdown table) and `<name>.xml` (the expected XForm). The test runner
`golden_fixtures_match_reference` compiles every `.md` and compares it to its
golden after XML C14N canonicalization (attribute order and insignificant
whitespace are ignored). Adding a case is just dropping the two files in — no
test code to write.

The golden `.xml` files are produced by the reference compiler
([pyxform](https://github.com/XLSForm/pyxform)) and a pair is committed only
when it is byte-identical to `rustxform`'s own output, so the compiler cannot be
its own oracle. Running the tests needs no Python; regenerating the corpus does
— see `scripts/gen_corpus.py` and [Conformance](#conformance).

## Documentation

For developers, `rustxform` uses Rust doc comments and type signatures; browse
them with `cargo doc --open`. Contributors should also be familiar with the
XForms specification (<https://getodk.github.io/xforms-spec/>).

For form authors, the input format is documented at:

- [XLSForm docs](https://xlsform.org/)
- the [ROADMAP](ROADMAP.md) (what is and isn't supported yet)
- the [CHANGELOG](CHANGELOG.md)

## Workspace layout

| Crate                  | Responsibility                                      |
| ---------------------- | --------------------------------------------------- |
| `rustxform-core`       | Data model: `Survey`, `Question`, `Settings`        |
| `rustxform-reader`     | Read a workbook into rows (Markdown, CSV, XLSX/XLS) |
| `rustxform-parse`      | Normalize rows into the survey model                |
| `rustxform-expr`       | Rewrite `${ref}` references into XPath               |
| `rustxform-xform`      | Emit XForm XML                                       |
| `rustxform-validate`   | Form validations and warnings                       |
| `rustxform-xform2json` | Reverse: parse an XForm back into the survey model  |
| `rustxform`            | End-to-end facade (`convert_markdown`, `build_*`)   |
| `rustxform-cli`        | Command-line interface (`--json`, `--odk-validate`) |

## Conformance

`rustxform` is checked against the reference compiler on a 53-form golden corpus
(all byte-identical after C14N) plus a 32/32 type cross-check. To regenerate the
corpus (needs Docker, pyxform and lxml):

    # 1. Build the CLI binary (Linux) into a shared path:
    docker run --rm -v "${PWD}:/work" -w /work \
      -v rustxform-target:/tmp/target -e CARGO_TARGET_DIR=/tmp/target \
      rust:latest bash -c "cargo build -q -p rustxform-cli && cp /tmp/target/debug/rustxform /work/.rustxform-bin"

    # 2. Generate and verify the fixtures:
    docker run --rm -v "${PWD}:/work" -w /work -e RUSTXFORM_BIN=/work/.rustxform-bin \
      python:3.12-slim bash -c "pip install --quiet pyxform openpyxl lxml && python scripts/gen_corpus.py"

## Releasing

1. Ensure `cargo test`, `cargo clippy --all-targets -- -D warnings` and
   `cargo fmt --all --check` are green.
2. Update `CHANGELOG.md` with the new version's notes.
3. Bump `version` in the workspace `Cargo.toml`.
4. Tag and push: `git tag -a vX.Y.Z -m "rustxform vX.Y.Z" && git push origin vX.Y.Z`.
5. Draft a GitHub release from the tag using the changelog notes.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Unless you explicitly state otherwise, any contribution
intentionally submitted for inclusion in this project, as defined in the
Apache-2.0 license, shall be dual-licensed as above, without any additional
terms or conditions.
