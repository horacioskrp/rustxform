//! Conformance tests.
//!
//! Phase 0 verifies the oracle machinery itself (canonicalization and diff).
//! Fixture-driven tests are `#[ignore]`d until Phase 1 implements real
//! conversion and the golden XForm is verified.

mod common;

use common::{assert_xform_eq, canonicalize, xform_diff};
use rustxform::convert_markdown;

#[test]
fn oracle_ignores_attribute_order() {
    let a = r#"<n x="1" y="2"><c/></n>"#;
    let b = r#"<n y="2" x="1"><c/></n>"#;
    assert_eq!(
        canonicalize(a).unwrap(),
        canonicalize(b).unwrap(),
        "canonicalization must ignore attribute order"
    );
    assert!(xform_diff(a, b).is_none());
}

#[test]
fn oracle_ignores_insignificant_whitespace() {
    let a = "<n>\n  <c/>\n</n>";
    let b = "<n><c/></n>";
    assert_eq!(canonicalize(a).unwrap(), canonicalize(b).unwrap());
}

#[test]
fn oracle_detects_real_difference() {
    let diff = xform_diff("<a/>", "<b/>").expect("different documents must diff");
    assert!(diff.contains("expected") && diff.contains("actual"));
}

#[test]
fn skeleton_is_well_formed_xform() {
    let xform = convert_markdown("| survey |").expect("conversion succeeds");
    // The stub still emits a well-formed XForm shell the oracle can parse.
    assert!(canonicalize(&xform).is_ok());
    assert!(xform.contains("h:html"));
    assert!(xform.contains("<model"));
}

#[test]
fn conformance_simple_text_form() {
    let md = include_str!("fixtures/simple.md");
    let expected = include_str!("fixtures/simple.xml");
    let actual = convert_markdown(md).expect("conversion succeeds");
    assert_xform_eq(expected, &actual);
}
