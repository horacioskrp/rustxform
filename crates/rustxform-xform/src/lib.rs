//! Emit XForm XML from the [`Survey`] model.
//!
//! Produces the `<model>` with its primary instance and binds, and the
//! `<body>` controls. Phase 1 supports flat forms of input-style questions
//! with inline (single-language) labels; groups, repeats, choices and
//! translations arrive in later phases.

use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use rustxform_core::{Question, Survey, resolve_type};

/// Name of the primary instance root element.
const ROOT: &str = "data";

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
    let title = survey.settings.title.as_deref().unwrap_or_default();
    let form_id = survey.settings.form_id.as_deref().unwrap_or(ROOT);

    let mut w = Writer::new(Vec::new());
    w.write_event(Event::Decl(BytesDecl::new("1.0", None, None)))?;

    let mut html = BytesStart::new("h:html");
    for (prefix, uri) in NAMESPACES {
        html.push_attribute((*prefix, *uri));
    }
    w.write_event(Event::Start(html))?;

    // <h:head>
    w.write_event(Event::Start(BytesStart::new("h:head")))?;
    write_text_element(&mut w, "h:title", title)?;

    let mut model = BytesStart::new("model");
    model.push_attribute(("odk:xforms-version", "1.0.0"));
    w.write_event(Event::Start(model))?;

    // <instance>
    w.write_event(Event::Start(BytesStart::new("instance")))?;
    let mut root = BytesStart::new(ROOT);
    root.push_attribute(("id", form_id));
    w.write_event(Event::Start(root))?;
    for question in &survey.children {
        w.write_event(Event::Empty(BytesStart::new(question.name.as_str())))?;
    }
    w.write_event(Event::Start(BytesStart::new("meta")))?;
    w.write_event(Event::Empty(BytesStart::new("instanceID")))?;
    w.write_event(Event::End(BytesEnd::new("meta")))?;
    w.write_event(Event::End(BytesEnd::new(ROOT)))?;
    w.write_event(Event::End(BytesEnd::new("instance")))?;

    // <bind> per question, then the instanceID bind
    for question in &survey.children {
        write_bind(&mut w, question)?;
    }
    let mut id_bind = BytesStart::new("bind");
    id_bind.push_attribute(("nodeset", format!("/{ROOT}/meta/instanceID").as_str()));
    id_bind.push_attribute(("type", "string"));
    id_bind.push_attribute(("readonly", "true()"));
    id_bind.push_attribute(("jr:preload", "uid"));
    w.write_event(Event::Empty(id_bind))?;

    w.write_event(Event::End(BytesEnd::new("model")))?;
    w.write_event(Event::End(BytesEnd::new("h:head")))?;

    // <h:body>
    w.write_event(Event::Start(BytesStart::new("h:body")))?;
    for question in &survey.children {
        write_control(&mut w, question)?;
    }
    w.write_event(Event::End(BytesEnd::new("h:body")))?;

    w.write_event(Event::End(BytesEnd::new("h:html")))?;

    Ok(String::from_utf8(w.into_inner())?)
}

/// XForm namespace declarations, emitted on the root `<h:html>` element.
const NAMESPACES: &[(&str, &str)] = &[
    ("xmlns", "http://www.w3.org/2002/xforms"),
    ("xmlns:h", "http://www.w3.org/1999/xhtml"),
    ("xmlns:ev", "http://www.w3.org/2001/xml-events"),
    ("xmlns:xsd", "http://www.w3.org/2001/XMLSchema"),
    ("xmlns:jr", "http://openrosa.org/javarosa"),
    ("xmlns:orx", "http://openrosa.org/xforms"),
    ("xmlns:odk", "http://www.opendatakit.org/xforms"),
];

/// Write `<bind nodeset="/data/{name}" type="..." [readonly="true()"]/>`.
fn write_bind(w: &mut Writer<Vec<u8>>, question: &Question) -> Result<(), XformError> {
    let spec = resolve_type(&question.kind);
    let bind_type = spec.map_or("string", |s| s.bind_type);

    let mut bind = BytesStart::new("bind");
    bind.push_attribute(("nodeset", format!("/{ROOT}/{}", question.name).as_str()));
    bind.push_attribute(("type", bind_type));
    if spec.is_some_and(|s| s.readonly) {
        bind.push_attribute(("readonly", "true()"));
    }
    w.write_event(Event::Empty(bind))?;
    Ok(())
}

/// Write the body control for a question, e.g. `<input ref="/data/{name}">`.
fn write_control(w: &mut Writer<Vec<u8>>, question: &Question) -> Result<(), XformError> {
    let Some(tag) = resolve_type(&question.kind).and_then(|s| s.control_tag) else {
        return Ok(());
    };

    let mut control = BytesStart::new(tag);
    control.push_attribute(("ref", format!("/{ROOT}/{}", question.name).as_str()));
    w.write_event(Event::Start(control))?;
    if let Some(label) = &question.label {
        write_text_element(w, "label", label)?;
    }
    w.write_event(Event::End(BytesEnd::new(tag)))?;
    Ok(())
}

/// Write `<tag>text</tag>`.
fn write_text_element(w: &mut Writer<Vec<u8>>, tag: &str, text: &str) -> Result<(), XformError> {
    w.write_event(Event::Start(BytesStart::new(tag)))?;
    w.write_event(Event::Text(BytesText::new(text)))?;
    w.write_event(Event::End(BytesEnd::new(tag)))?;
    Ok(())
}
