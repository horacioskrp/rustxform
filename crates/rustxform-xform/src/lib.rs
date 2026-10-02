//! Emit XForm XML from the [`Survey`] model.
//!
//! Produces the `<model>` with its primary instance and binds, and the
//! `<body>` controls. Phase 0 emits a well-formed XForm skeleton (title +
//! empty instance/body); questions, binds, itext and body controls are
//! filled in from Phase 1.

use quick_xml::Writer;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use rustxform_core::Survey;

/// An error produced while emitting XForm XML.
#[derive(Debug, thiserror::Error)]
pub enum XformError {
    /// Underlying XML writer I/O failure.
    #[error("xml write error: {0}")]
    Io(#[from] std::io::Error),
    /// The writer produced bytes that are not valid UTF-8.
    #[error("xform output was not valid utf-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

/// Serialize a [`Survey`] to an XForm XML string.
///
/// # Errors
///
/// Returns [`XformError`] if the XML writer fails or produces invalid UTF-8.
pub fn survey_to_xform(survey: &Survey) -> Result<String, XformError> {
    let title = survey.settings.title.clone().unwrap_or_default();
    let form_id = survey
        .settings
        .form_id
        .clone()
        .unwrap_or_else(|| "data".to_owned());

    let mut w = Writer::new(Vec::new());

    let mut html = BytesStart::new("h:html");
    html.push_attribute(("xmlns", "http://www.w3.org/2002/xforms"));
    html.push_attribute(("xmlns:h", "http://www.w3.org/1999/xhtml"));
    html.push_attribute(("xmlns:jr", "http://openrosa.org/javarosa"));
    html.push_attribute(("xmlns:orx", "http://openrosa.org/xforms"));
    w.write_event(Event::Start(html))?;

    w.write_event(Event::Start(BytesStart::new("h:head")))?;

    w.write_event(Event::Start(BytesStart::new("h:title")))?;
    w.write_event(Event::Text(BytesText::new(&title)))?;
    w.write_event(Event::End(BytesEnd::new("h:title")))?;

    w.write_event(Event::Start(BytesStart::new("model")))?;
    w.write_event(Event::Start(BytesStart::new("instance")))?;
    let mut root = BytesStart::new("data");
    root.push_attribute(("id", form_id.as_str()));
    w.write_event(Event::Empty(root))?;
    w.write_event(Event::End(BytesEnd::new("instance")))?;
    w.write_event(Event::End(BytesEnd::new("model")))?;

    w.write_event(Event::End(BytesEnd::new("h:head")))?;

    w.write_event(Event::Empty(BytesStart::new("h:body")))?;

    w.write_event(Event::End(BytesEnd::new("h:html")))?;

    Ok(String::from_utf8(w.into_inner())?)
}
