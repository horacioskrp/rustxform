//! Conformance tests.
//!
//! The oracle self-tests check the canonicalization/diff machinery. The corpus
//! runner compiles every `fixtures/<name>.md` and compares it to the golden
//! `fixtures/<name>.xml` produced by the reference compiler.

mod common;

use std::fs;
use std::path::PathBuf;

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

/// The fixtures directory (`crates/rustxform/tests/fixtures`).
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Every `<name>` that has both a `.md` input and a `.xml` golden, sorted.
fn golden_names(dir: &PathBuf) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("fixtures dir")
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()? != "md" {
                return None;
            }
            let name = path.file_stem()?.to_str()?.to_owned();
            dir.join(format!("{name}.xml")).exists().then_some(name)
        })
        .collect();
    names.sort();
    names
}

/// Corpus runner: compile each Markdown fixture and compare to its golden.
#[test]
fn golden_fixtures_match_reference() {
    let dir = fixtures_dir();
    let names = golden_names(&dir);
    assert!(
        names.len() >= 50,
        "expected the full corpus, found only {}",
        names.len()
    );

    let mut failures = Vec::new();
    for name in &names {
        let md = fs::read_to_string(dir.join(format!("{name}.md"))).expect("read .md");
        let expected = fs::read_to_string(dir.join(format!("{name}.xml"))).expect("read .xml");
        match convert_markdown(&md) {
            Ok(actual) => {
                if let Some(diff) = xform_diff(&expected, &actual) {
                    failures.push(format!("[{name}]\n{diff}"));
                }
            }
            Err(error) => failures.push(format!("[{name}] conversion error: {error}")),
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} fixtures differ from the golden:\n{}",
        failures.len(),
        names.len(),
        failures.join("\n")
    );
}

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
