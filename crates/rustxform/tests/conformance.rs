//! Conformance tests.
//!
//! Phase 0 verifies the oracle machinery itself (canonicalization and diff).
//! Fixture-driven tests are `#[ignore]`d until Phase 1 implements real
//! conversion and the golden XForm is verified.

mod common;

use common::{assert_xform_eq, canonicalize, xform_diff};
use rustxform::{convert_csv, convert_markdown, convert_xlsx};

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

/// Compile `fixtures/<name>.md` and compare to `fixtures/<name>.xml`.
macro_rules! golden {
    ($test:ident, $name:literal) => {
        #[test]
        fn $test() {
            let md = include_str!(concat!("fixtures/", $name, ".md"));
            let expected = include_str!(concat!("fixtures/", $name, ".xml"));
            let actual = convert_markdown(md).expect("conversion succeeds");
            assert_xform_eq(expected, &actual);
        }
    };
}

golden!(conformance_simple_text_form, "simple");
golden!(conformance_choices_form, "choices");
golden!(conformance_types_and_metadata_form, "types");
golden!(conformance_groups_and_repeats_form, "groups");
golden!(conformance_logic_and_references_form, "logic");
golden!(conformance_multilingual_form, "multi");

#[test]
fn xlsx_reader_matches_markdown_golden() {
    let xform = convert_xlsx(include_bytes!("fixtures/simple.xlsx")).expect("conversion succeeds");
    assert_xform_eq(include_str!("fixtures/simple.xml"), &xform);
}

#[test]
fn xlsx_reader_handles_multilingual_form() {
    let xform = convert_xlsx(include_bytes!("fixtures/multi.xlsx")).expect("conversion succeeds");
    assert_xform_eq(include_str!("fixtures/multi.xml"), &xform);
}

#[test]
fn csv_reader_matches_markdown_golden() {
    let xform = convert_csv(include_str!("fixtures/simple.csv")).expect("conversion succeeds");
    assert_xform_eq(include_str!("fixtures/simple.xml"), &xform);
}
