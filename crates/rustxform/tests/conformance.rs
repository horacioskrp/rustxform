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
golden!(conformance_advanced_columns_form, "advanced");
golden!(conformance_media_label_form, "media");
golden!(conformance_default_values_form, "defaults");
golden!(conformance_deferred_features_form, "deferred");
golden!(conformance_select_from_file_form, "ext");
golden!(conformance_last_saved_form, "lastsaved");
golden!(conformance_multilingual_messages_form, "mlmsg");
golden!(conformance_cascading_select_form, "cascade");
golden!(conformance_multilingual_media_form, "mmedia");
golden!(conformance_entities_form, "entity");
golden!(conformance_rank_form, "rank");
golden!(conformance_background_audio_form, "bgaudio");
golden!(conformance_start_geopoint_form, "startgeo");
golden!(conformance_audit_form, "audit");
golden!(conformance_entity_update_form, "eupd");
golden!(conformance_xml_external_form, "xmlexternal");
golden!(conformance_osm_form, "osm");
golden!(conformance_entity_create_if_form, "eci");
golden!(conformance_entity_update_if_form, "eui");
golden!(conformance_range_default_form, "rangedefault");
golden!(conformance_file_form, "file");
golden!(conformance_phone_number_form, "phonenumber");

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
