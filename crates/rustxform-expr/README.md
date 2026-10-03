# rustxform-expr

Rewrite `${name}` references inside XLSForm expressions into XPath. A small,
dependency-free helper used by the emission and validation stages of the
[`rustxform`](https://crates.io/crates/rustxform) pipeline.

```text
 parse ─▶ Survey ─▶ xform ─▶ XForm XML
                     ▲
                     │  expr: ${name}  ──▶  /data/… XPath
                     └── (also used by validate to find references)
```

Full XPath is **never evaluated**. Expressions are only scanned for `${name}`
tokens; everything else is passed through untouched. This keeps the crate tiny
and the behavior predictable.

## API

```rust
/// Replace every `${name}` using `resolve`; resolved refs become ` <xpath> `
/// (space-padded, matching the reference compiler).
pub fn rewrite_references(expr: &str, resolve: impl Fn(&str) -> Option<String>) -> String;

/// List the names referenced as `${name}`, in order.
pub fn reference_names(expr: &str) -> Vec<String>;
```

`resolve` returns the XPath for a referenced node, or `None` to leave the
`${name}` token verbatim (the caller decides what counts as resolvable).

## Example

```rust
use rustxform_expr::{reference_names, rewrite_references};

// Discover references:
assert_eq!(reference_names("${a} + ${b} - 1"), ["a", "b"]);

// Rewrite known references; unknown ones are left as-is:
let out = rewrite_references("${age} >= 18", |name| {
    (name == "age").then(|| "/data/age".to_owned())
});
assert_eq!(out, " /data/age  >= 18");
```

## Note

Internal building block of the `rustxform` workspace. For end-to-end XLSForm →
XForm compilation, use the top-level
[`rustxform`](https://crates.io/crates/rustxform) crate.

## License

Licensed under either of [Apache-2.0](../../LICENSE-APACHE) or
[MIT](../../LICENSE-MIT) at your option.
