# rustxform-xform2json

The **reverse** direction of the
[`rustxform`](https://crates.io/crates/rustxform) pipeline: parse an existing
XForm back into the [`Survey`](https://crates.io/crates/rustxform-core) model,
to import or inspect a compiled form.

```text
 XLSForm ─▶ reader ─▶ parse ─▶ xform ─▶ XForm XML
                                           │
                        xform2json ◀────────┘   (XForm ─▶ Survey)
```

## API

```rust
pub fn xform_to_survey(xml: &str) -> Result<Survey, Xform2JsonError>;
```

It reads the `<bind>`s to infer each field's type, the primary-instance root for
the `form_id`, `<h:title>` for the title, and the body controls
(`input`/`select1`/`select`/`rank`/`trigger`/`upload`/`range`) for the question
list and inline labels.

## Scope

This recovers the common forward output as a **flat** list of questions.
Not yet reconstructed:

- groups and repeats (nested fields are flattened),
- `itext` translations (only inline labels),
- choice lists (a select keeps its list id, not its options),
- Entities.

It is a useful inspector and a basis for round-tripping, not a full inverse of
the emitter.

## Example

```rust
use rustxform_xform2json::xform_to_survey;

let xml = r#"<?xml version="1.0"?>
<h:html xmlns:h="http://www.w3.org/1999/xhtml">
  <h:head>
    <h:title>Demo</h:title>
    <model>
      <instance><data id="demo"><q/></data></instance>
      <bind nodeset="/data/q" type="string"/>
    </model>
  </h:head>
  <h:body><input ref="/data/q"><label>Q</label></input></h:body>
</h:html>"#;

let survey = xform_to_survey(xml).unwrap();
assert_eq!(survey.settings.title.as_deref(), Some("Demo"));
assert_eq!(survey.children.len(), 1);
```

## Note

Internal building block of the `rustxform` workspace. For end-to-end XLSForm →
XForm compilation (the forward direction), use the top-level
[`rustxform`](https://crates.io/crates/rustxform) crate.

## License

Licensed under either of [Apache-2.0](../../LICENSE-APACHE) or
[MIT](../../LICENSE-MIT) at your option.
