# rustxform-parse

The **parsing** stage of the [`rustxform`](https://crates.io/crates/rustxform)
pipeline: normalize a raw `Workbook` into the
[`Survey`](https://crates.io/crates/rustxform-core) model. This is the semantic
core — type resolution, group/repeat nesting, choices, settings, localization,
media, entities and OSM tag sets.

```text
            ┌────────┐   ┌───────┐   ┌───────┐
 XLSForm ──▶│ reader │──▶│ parse │──▶│ xform │──▶ XForm XML
            └────────┘   └───────┘   └───────┘
             Workbook  ▸▸ Survey ◂◂    XForm
```

## API

```rust
pub fn workbook_to_survey(workbook: &Workbook) -> Result<Survey, ParseError>;
```

One function, one output. `ParseError` currently has no variants — parsing of a
well-formed workbook is total; structural problems (duplicate names, broken
references, …) are surfaced later by
[`rustxform-validate`](https://crates.io/crates/rustxform-validate), not here.

What it resolves, among others:

- `type` tokens → `Kind` (`text`/`integer`/… built-ins, `select_one|multiple|rank`,
  `select_*_from_file`, `osm`, the `phone number` preset, unknown → `Unknown`).
- `begin_group`/`end_group` and `begin_repeat`/`end_repeat` → a nested `Node` tree.
- `label`/`hint`/`constraint_message`/`required_message` with `::Lang` qualifiers
  → `Localized`, and the form's `languages` list.
- `media::image|audio|video[::Lang]` → `Media`.
- The `choices`, `settings`, `entities` and `osm` sheets.

## Example

```rust
use rustxform_reader::read_markdown;
use rustxform_parse::workbook_to_survey;

let wb = read_markdown("\
| survey |         |      |       |
|        | type    | name | label |
|        | integer | age  | Age   |
").unwrap();

let survey = workbook_to_survey(&wb).unwrap();
assert_eq!(survey.children.len(), 1);
```

## Note

Internal building block of the `rustxform` workspace. For end-to-end XLSForm →
XForm compilation, use the top-level
[`rustxform`](https://crates.io/crates/rustxform) crate.

## License

Licensed under either of [Apache-2.0](../../LICENSE-APACHE) or
[MIT](../../LICENSE-MIT) at your option.
