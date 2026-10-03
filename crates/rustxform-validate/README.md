# rustxform-validate

Structural validations and non-fatal warnings over a parsed
[`Survey`](https://crates.io/crates/rustxform-core), run between parsing and
emission in the [`rustxform`](https://crates.io/crates/rustxform) pipeline.

```text
 parse ─▶ Survey ─┬─▶ validate ─▶ (blocking errors?)
                  └─▶ xform ─▶ XForm XML
```

## API

```rust
/// Blocking problems; empty when the form is valid.
pub fn validate(survey: &Survey) -> Vec<ValidationError>;

/// Non-fatal warnings; needs the raw workbook for header checks.
pub fn warnings(workbook: &Workbook, survey: &Survey) -> Vec<Warning>;
```

### `ValidationError` (blocking)

- `DuplicateName` / `EmptyName` — node naming problems.
- `UnknownReference { owner, reference }` — a `${…}` that names no node.
- `UnknownChoiceList { question, list }` — a `select_*` over a missing list.
- `InvalidRange { question, reason }` — malformed `range` `parameters`.
- `InvalidGeoDefault { question, value }` — a geo `default` that is not
  `;`-separated `lat lon altitude accuracy` points.

### `Warning` (non-fatal)

- `MissingSetting` — e.g. no `form_title`/`form_id` (a default is used).
- `UnknownColumn` — an unrecognized `survey` column header (likely a typo).
- `UnknownLanguageTag` — a language without a recognizable subtag code
  (ISO 639-1 two-letter, or any three-letter code).

Both enums implement `std::error::Error` via `Display`, so they print clean
messages.

## Example

```rust
use rustxform_reader::read_markdown;
use rustxform_parse::workbook_to_survey;
use rustxform_validate::{validate, ValidationError};

let wb = read_markdown("\
| survey |      |      |              |
|        | type | name | relevant     |
|        | text | q    | ${ghost} = 1 |
").unwrap();

let survey = workbook_to_survey(&wb).unwrap();
let errors = validate(&survey);
assert!(errors.contains(&ValidationError::UnknownReference {
    owner: "q".to_owned(),
    reference: "ghost".to_owned(),
}));
```

## Note

Internal building block of the `rustxform` workspace. The top-level
[`rustxform`](https://crates.io/crates/rustxform) crate runs this for you via
its `build_*` / `convert_*_checked` functions.

## License

Licensed under either of [Apache-2.0](../../LICENSE-APACHE) or
[MIT](../../LICENSE-MIT) at your option.
