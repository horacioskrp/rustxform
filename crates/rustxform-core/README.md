# rustxform-core

Core data model shared across the [`rustxform`](https://crates.io/crates/rustxform)
pipeline — the normalized, backend-agnostic representation of an
[XLSForm](https://xlsform.org/) after reading and parsing, and before XForm
emission.

```text
            ┌────────┐   ┌───────┐   ┌───────┐
 XLSForm ──▶│ reader │──▶│ parse │──▶│ xform │──▶ XForm XML
            └────────┘   └───────┘   └───────┘
               Workbook  ▸▸ Survey ◂◂   (reads the model)
                            core: the types every stage speaks
```

`rustxform-core` has **no dependencies** and defines no behavior beyond a couple
of lookup helpers: it is purely the vocabulary the other crates exchange.

## What it contains

| Type | Role |
| --- | --- |
| `Survey` | A whole parsed form: `settings`, a `children` node tree, `choices`, `languages`, `entity`, `audit`, `external_instances`, `osm_tags`. |
| `Node` | A tree node: `Question`, `Group(Container)` or `Repeat(Container)`. |
| `Question` | A leaf field: `kind`, `name`, `label`/`hint`, logic columns, `parameters`, `media`, `default`, … |
| `Container` | A group or repeat: `name`, `label`, `appearance`, repeat `count`, `children`. |
| `Kind` | Resolved `type` semantics: `Builtin`, `Select`, `Osm`, or `Unknown`. |
| `Builtin` / `Control` / `SelectType` | The XForm mapping of a built-in type, its body control, and the selection flavor (`One`/`Multiple`/`Rank`). |
| `Preload` / `Action` | Metadata preloads (`jr:preload`) and model actions (`odk:recordaudio`, `odk:setgeopoint`). |
| `Choice` / `ChoiceList` | Choice options and named lists. |
| `Localized` / `Media` | Single- or multi-language text, and label media attachments. |
| `Entity` | An Entities declaration (create/update a dataset row). |
| `Settings` | Form-level settings (`form_id`, `title`, `version`, …). |

## Example

```rust
use rustxform_core::{resolve_builtin, Control};

// `resolve_builtin` maps an XLSForm type token to its XForm binding.
let integer = resolve_builtin("integer").unwrap();
assert_eq!(integer.bind_type, "int");
assert_eq!(integer.control, Some(Control::Input));

let note = resolve_builtin("note").unwrap();
assert!(note.readonly);

// `select_*`, `osm` and unrecognized tokens return `None` — they are resolved
// by the parser, which has the choice lists and tag sets in context.
assert!(resolve_builtin("select_one").is_none());
```

Every type derives `Debug`, `Clone` and `PartialEq`, so forms are easy to build
by hand in tests and to compare.

## Note

This is an internal building block of the `rustxform` workspace. To compile
XLSForms, depend on the top-level
[`rustxform`](https://crates.io/crates/rustxform) crate instead; reach for
`rustxform-core` only when you build other tools on the same model.

## License

Licensed under either of [Apache-2.0](../../LICENSE-APACHE) or
[MIT](../../LICENSE-MIT) at your option.
