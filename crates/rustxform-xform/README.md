# rustxform-xform

The **emission** stage of the [`rustxform`](https://crates.io/crates/rustxform)
pipeline: serialize a [`Survey`](https://crates.io/crates/rustxform-core) into
an XForm — the [JavaRosa-flavored XML](https://getodk.github.io/xforms-spec/)
form definition.

```text
            ┌────────┐   ┌───────┐   ┌───────┐
 XLSForm ──▶│ reader │──▶│ parse │──▶│ xform │──▶ XForm XML
            └────────┘   └───────┘   └───────┘
             Workbook      Survey  ▸▸ XForm ◂◂
```

## What it emits

- The `<h:head>`/`<model>`: optional `<itext>` translations, the primary
  instance, choice secondary instances, `<bind>`s, metadata preloads,
  Entities binds, and model actions.
- The `<h:body>`: `<input>`/`<select1>`/`<select>`/`<rank>`/`<range>`/
  `<trigger>`/`<upload>` controls, groups and repeats, labels and hints.
- A single-language form uses inline labels; a multi-language form switches to
  `<itext>` with `jr:itext(...)` references.

`${…}` references in logic columns are rewritten to XPath via
[`rustxform-expr`](https://crates.io/crates/rustxform-expr).

## API

```rust
pub fn survey_to_xform(survey: &Survey) -> Result<String, XformError>;
```

`XformError` wraps the underlying XML-writer/IO failures.

## Example

```rust
use rustxform_reader::read_markdown;
use rustxform_parse::workbook_to_survey;
use rustxform_xform::survey_to_xform;

let wb = read_markdown("\
| survey |         |      |       |
|        | type    | name | label |
|        | text    | q    | Q     |
").unwrap();

let survey = workbook_to_survey(&wb).unwrap();
let xml = survey_to_xform(&survey).unwrap();
assert!(xml.contains("<h:html"));
assert!(xml.contains("<input"));
```

## Note

Internal building block of the `rustxform` workspace. For end-to-end XLSForm →
XForm compilation, use the top-level
[`rustxform`](https://crates.io/crates/rustxform) crate.

## License

Licensed under either of [Apache-2.0](../../LICENSE-APACHE) or
[MIT](../../LICENSE-MIT) at your option.
