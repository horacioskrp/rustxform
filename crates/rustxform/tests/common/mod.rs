//! Conformance oracle: canonicalize XForm XML and diff two documents.
//!
//! XForm equivalence must ignore insignificant XML differences (attribute
//! order, inter-element whitespace). We canonicalize both sides and compare.
//! XPath-level assertions are added in a later phase.

use quick_xml::Writer;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;

/// Canonicalize an XML string: trim inter-element whitespace and sort the
/// attributes of every element by key.
///
/// Returns a human-readable error string on malformed XML.
pub fn canonicalize(xml: &str) -> Result<String, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut writer = Writer::new(Vec::new());

    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Eof => break,
            Event::Start(e) => writer
                .write_event(Event::Start(sort_attributes(&e)))
                .map_err(|e| e.to_string())?,
            Event::Empty(e) => writer
                .write_event(Event::Empty(sort_attributes(&e)))
                .map_err(|e| e.to_string())?,
            other => writer.write_event(other).map_err(|e| e.to_string())?,
        }
    }

    String::from_utf8(writer.into_inner()).map_err(|e| e.to_string())
}

fn sort_attributes(start: &BytesStart<'_>) -> BytesStart<'static> {
    let name = String::from_utf8_lossy(start.name().as_ref()).into_owned();
    let mut out = BytesStart::new(name);
    let mut attrs: Vec<_> = start.attributes().filter_map(Result::ok).collect();
    attrs.sort_by(|a, b| a.key.as_ref().cmp(b.key.as_ref()));
    for attr in attrs {
        // Decode then re-encode so escaping is normalized (e.g. `'` vs `&apos;`,
        // `>` vs `&gt;`): equivalent values compare equal regardless of source.
        let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
        let value = attr
            .unescape_value()
            .map(|v| v.into_owned())
            .unwrap_or_default();
        out.push_attribute((key.as_str(), value.as_str()));
    }
    out
}

/// Return a unified diff of two XForm documents after canonicalization, or
/// `None` when they are equivalent.
pub fn xform_diff(expected: &str, actual: &str) -> Option<String> {
    let expected_c = canonicalize(expected).expect("expected XForm is well-formed XML");
    let actual_c = canonicalize(actual).expect("actual XForm is well-formed XML");
    if expected_c == actual_c {
        return None;
    }
    let diff = similar::TextDiff::from_lines(&expected_c, &actual_c);
    Some(diff.unified_diff().header("expected", "actual").to_string())
}

/// Assert that two XForm documents are equivalent, panicking with a diff
/// otherwise.
#[track_caller]
pub fn assert_xform_eq(expected: &str, actual: &str) {
    if let Some(diff) = xform_diff(expected, actual) {
        panic!("XForm mismatch:\n{diff}");
    }
}
