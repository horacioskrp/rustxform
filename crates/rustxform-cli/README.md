# rustxform-cli

Command-line interface for [`rustxform`](https://crates.io/crates/rustxform):
compile an [XLSForm](https://xlsform.org/) into an
[XForm](https://getodk.github.io/xforms-spec/). It validates the form, picks the
reader by file extension, and writes the XML.

## Install

```sh
cargo install rustxform-cli
```

This installs a binary named `rustxform`.

## Usage

```sh
rustxform <input> [output] [--json] [--odk-validate <JAR>]
```

- `<input>` — `.md`, `.csv`, `.xlsx` or `.xls`. The reader is chosen by extension.
- `[output]` — defaults to the input path with a `.xml` suffix.

```sh
rustxform form.xlsx                 # writes form.xml
rustxform form.md out.xml           # explicit output path
rustxform form.csv out.xml --json   # machine-readable report on stdout
rustxform form.xlsx out.xml --odk-validate ODK-Validate.jar
```

### Flags

| Flag | Effect |
| --- | --- |
| `--json` | Emit a JSON report (`status`, `output`, `warnings`, `errors`) on stdout instead of plain text. |
| `--odk-validate <JAR>` | After writing, run `java -jar <JAR> <xform>` to validate the output (needs Java on `PATH`). |

### Exit codes

- `0` — the form compiled (any warnings are printed but non-fatal).
- non-zero — the form is invalid (validation errors), the output was rejected by
  ODK Validate, or an I/O error occurred.

Validation errors are reported with their messages; a valid form with issues
such as a missing `form_title` still compiles and prints the warning.

## Example

Given `age.md`:

```text
| survey |         |      |       |
|        | type    | name | label |
|        | integer | age  | Age   |
| settings |            |         |
|          | form_title | form_id |
|          | Age form   | age     |
```

```sh
$ rustxform age.md
wrote age.xml
```

## License

Licensed under either of [Apache-2.0](../../LICENSE-APACHE) or
[MIT](../../LICENSE-MIT) at your option.
