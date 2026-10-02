//! Emit XForm XML from the [`Survey`] model.
//!
//! Produces the `<model>` (primary instance, choice secondary instances and
//! binds) and the `<body>` controls. Phase 2 supports flat forms: built-in
//! types, metadata preloads, `calculate`, and `select_one`/`select_multiple`
//! backed by choice lists. Groups, repeats and translations arrive later.

use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use rustxform_core::{Control, Kind, Question, Survey};

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

type W = Writer<Vec<u8>>;

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

    w.write_event(Event::Start(BytesStart::new("h:head")))?;
    text_element(&mut w, "h:title", title)?;

    let mut model = BytesStart::new("model");
    model.push_attribute(("odk:xforms-version", "1.0.0"));
    w.write_event(Event::Start(model))?;

    write_primary_instance(&mut w, survey, form_id)?;
    write_choice_instances(&mut w, survey)?;
    for question in &survey.children {
        write_bind(&mut w, question)?;
    }
    write_instance_id_bind(&mut w)?;

    w.write_event(Event::End(BytesEnd::new("model")))?;
    w.write_event(Event::End(BytesEnd::new("h:head")))?;

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

/// Primary instance: `<instance><data id="..">…<meta>…</meta></data></instance>`.
fn write_primary_instance(w: &mut W, survey: &Survey, form_id: &str) -> Result<(), XformError> {
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
    Ok(())
}

/// Secondary instances for each referenced choice list, in first-use order.
fn write_choice_instances(w: &mut W, survey: &Survey) -> Result<(), XformError> {
    let mut seen: Vec<&str> = Vec::new();
    for question in &survey.children {
        let Kind::Select { list, .. } = &question.kind else {
            continue;
        };
        if seen.contains(&list.as_str()) {
            continue;
        }
        seen.push(list);
        let Some(choice_list) = survey.choice_list(list) else {
            continue;
        };

        let mut instance = BytesStart::new("instance");
        instance.push_attribute(("id", list.as_str()));
        w.write_event(Event::Start(instance))?;
        w.write_event(Event::Start(BytesStart::new("root")))?;
        for choice in &choice_list.items {
            w.write_event(Event::Start(BytesStart::new("item")))?;
            text_element(w, "name", &choice.name)?;
            if let Some(label) = &choice.label {
                text_element(w, "label", label)?;
            }
            w.write_event(Event::End(BytesEnd::new("item")))?;
        }
        w.write_event(Event::End(BytesEnd::new("root")))?;
        w.write_event(Event::End(BytesEnd::new("instance")))?;
    }
    Ok(())
}

/// Write the `<bind>` for a question.
fn write_bind(w: &mut W, question: &Question) -> Result<(), XformError> {
    let nodeset = format!("/{ROOT}/{}", question.name);
    let mut bind = BytesStart::new("bind");
    bind.push_attribute(("nodeset", nodeset.as_str()));

    match &question.kind {
        Kind::Builtin(b) => {
            if let Some(p) = b.preload {
                bind.push_attribute(("jr:preload", p.preload));
                bind.push_attribute(("type", b.bind_type));
                bind.push_attribute(("jr:preloadParams", p.params));
            } else {
                bind.push_attribute(("type", b.bind_type));
                if b.readonly {
                    bind.push_attribute(("readonly", "true()"));
                }
                if let Some(calc) = &question.calculation {
                    bind.push_attribute(("calculate", calc.as_str()));
                }
            }
        }
        Kind::Select { .. } | Kind::Unknown(_) => {
            bind.push_attribute(("type", "string"));
            if let Some(calc) = &question.calculation {
                bind.push_attribute(("calculate", calc.as_str()));
            }
        }
    }
    w.write_event(Event::Empty(bind))?;
    Ok(())
}

/// Write the fixed `instanceID` metadata bind.
fn write_instance_id_bind(w: &mut W) -> Result<(), XformError> {
    let mut bind = BytesStart::new("bind");
    bind.push_attribute(("nodeset", format!("/{ROOT}/meta/instanceID").as_str()));
    bind.push_attribute(("type", "string"));
    bind.push_attribute(("readonly", "true()"));
    bind.push_attribute(("jr:preload", "uid"));
    w.write_event(Event::Empty(bind))?;
    Ok(())
}

/// Write the body control for a question (nothing for data-only types).
fn write_control(w: &mut W, question: &Question) -> Result<(), XformError> {
    let reference = format!("/{ROOT}/{}", question.name);
    match &question.kind {
        Kind::Builtin(b) => match b.control {
            None => Ok(()),
            Some(Control::Input) => simple_control(w, "input", &reference, question, &[]),
            Some(Control::Trigger) => simple_control(w, "trigger", &reference, question, &[]),
            Some(Control::Upload { mediatype }) => simple_control(
                w,
                "upload",
                &reference,
                question,
                &[("mediatype", mediatype)],
            ),
        },
        Kind::Select { multiple, list } => write_select(w, *multiple, list, &reference, question),
        Kind::Unknown(_) => Ok(()),
    }
}

/// Write `<tag ref=".." extra..>[<label>..</label>]</tag>`.
fn simple_control(
    w: &mut W,
    tag: &str,
    reference: &str,
    question: &Question,
    extra: &[(&str, &str)],
) -> Result<(), XformError> {
    let mut control = BytesStart::new(tag);
    control.push_attribute(("ref", reference));
    for attr in extra {
        control.push_attribute(*attr);
    }
    w.write_event(Event::Start(control))?;
    write_label(w, question)?;
    w.write_event(Event::End(BytesEnd::new(tag)))?;
    Ok(())
}

/// Write a `<select1>`/`<select>` control with an itemset over a list instance.
fn write_select(
    w: &mut W,
    multiple: bool,
    list: &str,
    reference: &str,
    question: &Question,
) -> Result<(), XformError> {
    let tag = if multiple { "select" } else { "select1" };
    let mut control = BytesStart::new(tag);
    control.push_attribute(("ref", reference));
    w.write_event(Event::Start(control))?;
    write_label(w, question)?;

    // Build the tag from raw content so the apostrophes in the nodeset are
    // emitted literally (matching reference output) rather than as `&apos;`.
    let itemset = BytesStart::from_content(
        format!("itemset nodeset=\"instance('{list}')/root/item\""),
        "itemset".len(),
    );
    w.write_event(Event::Start(itemset))?;
    ref_element(w, "value", "name")?;
    ref_element(w, "label", "label")?;
    w.write_event(Event::End(BytesEnd::new("itemset")))?;

    w.write_event(Event::End(BytesEnd::new(tag)))?;
    Ok(())
}

/// Write a question's `<label>` when it has one.
fn write_label(w: &mut W, question: &Question) -> Result<(), XformError> {
    if let Some(label) = &question.label {
        text_element(w, "label", label)?;
    }
    Ok(())
}

/// Write `<tag ref="value"/>`.
fn ref_element(w: &mut W, tag: &str, value: &str) -> Result<(), XformError> {
    let mut element = BytesStart::new(tag);
    element.push_attribute(("ref", value));
    w.write_event(Event::Empty(element))?;
    Ok(())
}

/// Write `<tag>text</tag>`.
fn text_element(w: &mut W, tag: &str, text: &str) -> Result<(), XformError> {
    w.write_event(Event::Start(BytesStart::new(tag)))?;
    w.write_event(Event::Text(BytesText::new(text)))?;
    w.write_event(Event::End(BytesEnd::new(tag)))?;
    Ok(())
}
