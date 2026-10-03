# rustxform-reader

The **reading** stage of the [`rustxform`](https://crates.io/crates/rustxform)
pipeline: turn an [XLSForm](https://xlsform.org/) source into a raw `Workbook`
of string rows, before any semantic interpretation.

```text
            ┌────────┐   ┌───────┐   ┌───────┐
 XLSForm ──▶│ reader │──▶│ parse │──▶│ xform │──▶ XForm XML
            └────────┘   └───────┘   └───────┘
            ▸▸ Workbook ◂◂  Survey      XForm
```

## Inputs

| Function | Input | Notes |
| --- | --- | --- |
| `read_markdown(&str)` | Markdown tables | First column selects the sheet (see below). Infallible. |
| `read_csv(&str)` | CSV text | Same sheet convention as Markdown. |
| `read_xlsx(&[u8])` | XLSX/XLS bytes | Each worksheet maps directly to a `Sheet` (via `calamine`). |

All three return `Result<Workbook, ReadError>` for a uniform API.

A `Workbook` is just `{ sheets: Vec<Sheet> }`, and a `Sheet` is
`{ name: String, rows: Vec<Vec<String>> }`. Use `Workbook::sheet(name)` to look
one up.

### The "first column selects the sheet" convention

For Markdown and CSV, a row whose **first cell is non-empty** opens a new sheet;
rows whose first cell is empty append their remaining cells to the current
sheet. Markdown separator rows (`|---|`) are skipped.

```text
| survey  |         |      |           |
|         | type    | name | label     |
|         | text    | q1   | Question  |
| settings |            |         |
|          | form_title | form_id |
|          | Demo       | demo    |
```

## Example

```rust
use rustxform_reader::read_markdown;

let wb = read_markdown("\
| survey |      |      |          |
|        | type | name | label    |
|        | text | q1   | Question |
").unwrap();

let survey = wb.sheet("survey").unwrap();
assert_eq!(survey.rows[0], ["type", "name", "label"]);
assert_eq!(survey.rows[1], ["text", "q1", "Question"]);
```

## Note

Internal building block of the `rustxform` workspace. For end-to-end XLSForm →
XForm compilation, use the top-level
[`rustxform`](https://crates.io/crates/rustxform) crate.

## License

Licensed under either of [Apache-2.0](../../LICENSE-APACHE) or
[MIT](../../LICENSE-MIT) at your option.
