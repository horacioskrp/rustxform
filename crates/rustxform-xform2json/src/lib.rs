//! Parse an XForm back into the [`Survey`] model (the reverse direction).
//!
//! This reconstructs the common forward output: the form title and id, and a
//! flat list of body questions with their inferred type, inline label and (for
//! selects) choice-list id. Groups/repeats, `itext` translations, reconstructed
//! choice lists and entities are not yet recovered — questions nested in groups
//! are flattened. Use it to import or inspect an existing XForm.

use std::collections::HashMap;

use quick_xml::events::Event;
use quick_xml::reader::Reader;
use rustxform_core::{Kind, Localized, Node, Question, SelectType, Survey, resolve_builtin};

/// An error produced while parsing an XForm.
#[derive(Debug, thiserror::Error)]
pub enum Xform2JsonError {
    /// The XML could not be parsed.
    #[error("xml parse error: {0}")]
    Xml(String),
}

/// Body control element names handled by the reverse parser.
const CONTROLS: &[&str] = &[
    "input", "select1", "select", "rank", "trigger", "upload", "range",
];

/// Parse an XForm document into a [`Survey`].
///
/// # Errors
///
/// Returns [`Xform2JsonError::Xml`] if the document is not well-formed.
pub fn xform_to_survey(xml: &str) -> Result<Survey, Xform2JsonError> {
    let binds = collect_binds(xml)?;
    let (title, form_id) = collect_metadata(xml)?;
    let questions = collect_questions(xml, &binds)?;

    let mut survey = Survey::default();
    survey.settings.title = title;
    survey.settings.form_id = form_id;
    survey.children = questions.into_iter().map(Node::Question).collect();
    Ok(survey)
}

/// `nodeset` → bind `type`, from the model binds.
fn collect_binds(xml: &str) -> Result<HashMap<String, String>, Xform2JsonError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut binds = HashMap::new();
    loop {
        match read(&mut reader)? {
            Event::Eof => break,
            Event::Start(e) | Event::Empty(e) if local_name(e.name().as_ref()) == "bind" => {
                let nodeset = attr(&e, "nodeset");
                let bind_type = attr(&e, "type");
                if let (Some(nodeset), Some(bind_type)) = (nodeset, bind_type) {
                    binds.insert(nodeset, bind_type);
                }
            }
            _ => {}
        }
    }
    Ok(binds)
}

/// Form `title` (from `<h:title>`) and `form_id` (the primary instance root id).
fn collect_metadata(xml: &str) -> Result<(Option<String>, Option<String>), Xform2JsonError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut title = None;
    let mut form_id = None;
    let mut in_title = false;
    let mut expect_root = false;
    loop {
        match read(&mut reader)? {
            Event::Eof => break,
            Event::Start(e) => match local_name(e.name().as_ref()).as_str() {
                "title" => in_title = true,
                "instance" if attr(&e, "id").is_none() && form_id.is_none() => expect_root = true,
                _ if expect_root => {
                    form_id = attr(&e, "id");
                    expect_root = false;
                }
                _ => {}
            },
            Event::Text(t) if in_title => {
                title = Some(t.unescape().map_err(xml_err)?.into_owned());
            }
            Event::End(e) if local_name(e.name().as_ref()) == "title" => in_title = false,
            _ => {}
        }
    }
    Ok((title, form_id))
}

/// A body control being assembled.
struct Partial {
    tag: String,
    reference: String,
    mediatype: Option<String>,
    list: Option<String>,
    label: Option<String>,
}

/// Walk the body controls into a flat list of questions.
fn collect_questions(
    xml: &str,
    binds: &HashMap<String, String>,
) -> Result<Vec<Question>, Xform2JsonError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut questions = Vec::new();
    let mut current: Option<Partial> = None;
    let mut in_label = false;
    let mut in_itemset = false;

    loop {
        match read(&mut reader)? {
            Event::Eof => break,
            Event::Start(e) | Event::Empty(e) => {
                let name = local_name(e.name().as_ref());
                if CONTROLS.contains(&name.as_str()) {
                    current = Some(Partial {
                        tag: name,
                        reference: attr(&e, "ref").unwrap_or_default(),
                        mediatype: attr(&e, "mediatype"),
                        list: None,
                        label: None,
                    });
                } else if name == "itemset" {
                    in_itemset = true;
                    if let Some(p) = current.as_mut() {
                        p.list = attr(&e, "nodeset").and_then(|n| instance_id(&n));
                    }
                } else if name == "label" && current.is_some() && !in_itemset {
                    in_label = true;
                }
            }
            Event::Text(t) if in_label => {
                if let Some(p) = current.as_mut() {
                    p.label = Some(t.unescape().map_err(xml_err)?.into_owned());
                }
            }
            Event::End(e) => match local_name(e.name().as_ref()).as_str() {
                "label" => in_label = false,
                "itemset" => in_itemset = false,
                name if CONTROLS.contains(&name) => {
                    if let Some(p) = current.take() {
                        questions.push(to_question(p, binds));
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    Ok(questions)
}

/// Build a [`Question`] from an assembled control and the binds.
fn to_question(p: Partial, binds: &HashMap<String, String>) -> Question {
    let name = p.reference.rsplit('/').next().unwrap_or("").to_owned();
    let bind_type = binds
        .get(&p.reference)
        .map(String::as_str)
        .unwrap_or("string");
    let kind = infer_kind(&p.tag, bind_type, p.list, p.mediatype.as_deref());

    Question {
        kind,
        name,
        label: Localized {
            default: p.label,
            langs: Vec::new(),
        },
        hint: Localized::default(),
        appearance: None,
        calculation: None,
        relevant: None,
        constraint: None,
        required: None,
        readonly: None,
        constraint_message: Localized::default(),
        required_message: Localized::default(),
        parameters: Vec::new(),
        media: Vec::new(),
        default: None,
        choice_filter: None,
        save_to: None,
    }
}

/// Infer a [`Kind`] from a control tag, bind type and (for selects) list.
fn infer_kind(tag: &str, bind_type: &str, list: Option<String>, mediatype: Option<&str>) -> Kind {
    let select = match tag {
        "select1" => Some(SelectType::One),
        "select" => Some(SelectType::Multiple),
        "rank" => Some(SelectType::Rank),
        _ => None,
    };
    match select {
        Some(select) => Kind::Select {
            select,
            list: list.unwrap_or_default(),
            file: None,
        },
        None => {
            let token = builtin_token(tag, bind_type, mediatype);
            resolve_builtin(&token).map_or(Kind::Unknown(token.clone()), Kind::Builtin)
        }
    }
}

/// Recover the XLSForm type token for a non-select control.
fn builtin_token(tag: &str, bind_type: &str, mediatype: Option<&str>) -> String {
    match tag {
        "trigger" => "acknowledge",
        "range" => "range",
        "upload" => match mediatype {
            Some("audio/*") => "audio",
            Some("video/*") => "video",
            _ => "image",
        },
        _ => match bind_type {
            "int" => "integer",
            "decimal" => "decimal",
            "date" => "date",
            "time" => "time",
            "dateTime" => "dateTime",
            "geopoint" => "geopoint",
            "geotrace" => "geotrace",
            "geoshape" => "geoshape",
            "barcode" => "barcode",
            _ => "text",
        },
    }
    .to_owned()
}

/// Extract `cities` from `instance('cities')/root/item[...]`.
fn instance_id(nodeset: &str) -> Option<String> {
    let start = nodeset.find("instance('")? + "instance('".len();
    let end = nodeset[start..].find('\'')? + start;
    Some(nodeset[start..end].to_owned())
}

/// The local part of a possibly-prefixed XML name (`h:title` → `title`).
fn local_name(raw: &[u8]) -> String {
    let name = String::from_utf8_lossy(raw);
    name.rsplit(':').next().unwrap_or(&name).to_owned()
}

/// Read one event, mapping XML errors.
fn read(reader: &mut Reader<&[u8]>) -> Result<Event<'static>, Xform2JsonError> {
    reader.read_event().map(Event::into_owned).map_err(xml_err)
}

/// An attribute value as an owned `String`, if present.
fn attr(e: &quick_xml::events::BytesStart<'_>, key: &str) -> Option<String> {
    e.attributes().flatten().find_map(|a| {
        (local_name(a.key.as_ref()) == key)
            .then(|| String::from_utf8_lossy(a.value.as_ref()).into_owned())
    })
}

fn xml_err(e: impl std::fmt::Display) -> Xform2JsonError {
    Xform2JsonError::Xml(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE: &str = r#"<?xml version="1.0"?>
<h:html xmlns="http://www.w3.org/2002/xforms" xmlns:h="http://www.w3.org/1999/xhtml">
  <h:head>
    <h:title>Simple</h:title>
    <model>
      <instance>
        <data id="simple"><name/><age/><meta><instanceID/></meta></data>
      </instance>
      <bind nodeset="/data/name" type="string"/>
      <bind nodeset="/data/age" type="int"/>
    </model>
  </h:head>
  <h:body>
    <input ref="/data/name"><label>Your name</label></input>
    <input ref="/data/age"><label>Your age</label></input>
  </h:body>
</h:html>"#;

    #[test]
    fn recovers_title_id_and_questions() {
        let survey = xform_to_survey(SIMPLE).unwrap();
        assert_eq!(survey.settings.title.as_deref(), Some("Simple"));
        assert_eq!(survey.settings.form_id.as_deref(), Some("simple"));
        assert_eq!(survey.children.len(), 2);

        let Node::Question(q0) = &survey.children[0] else {
            panic!("expected question");
        };
        assert_eq!(q0.name, "name");
        assert_eq!(q0.label.single(), Some("Your name"));
        assert_eq!(q0.kind, Kind::Builtin(resolve_builtin("text").unwrap()));

        let Node::Question(q1) = &survey.children[1] else {
            panic!("expected question");
        };
        assert_eq!(q1.name, "age");
        assert_eq!(q1.kind, Kind::Builtin(resolve_builtin("integer").unwrap()));
    }

    #[test]
    fn recovers_select_list() {
        let xml = r#"<h:html xmlns:h="http://www.w3.org/1999/xhtml"><h:head><model>
            <bind nodeset="/data/city" type="string"/>
            </model></h:head><h:body>
            <select1 ref="/data/city"><label>City</label>
              <itemset nodeset="instance('cities')/root/item"><value ref="name"/></itemset>
            </select1></h:body></h:html>"#;
        let survey = xform_to_survey(xml).unwrap();
        let Node::Question(q) = &survey.children[0] else {
            panic!("expected question");
        };
        assert_eq!(
            q.kind,
            Kind::Select {
                select: SelectType::One,
                list: "cities".to_owned(),
                file: None,
            }
        );
    }
}
