//! Parse an XForm back into the [`Survey`] model (the reverse direction).
//!
//! This reconstructs the form title and id, and the body questions with their
//! inferred type, inline label and hint, `appearance`, and the full bind logic
//! (`relevant` / `constraint` / `required` / `read_only` / `calculate` and the
//! constraint/required messages), recovered as literal XPath. Because the
//! forward emitter only rewrites `${…}` tokens and passes other expressions
//! through unchanged, a flat single-language form round-trips byte-identically:
//! `emit(xform_to_survey(emit(survey))) == emit(survey)`.
//!
//! Not yet recovered (questions nested in groups are flattened): group/repeat
//! structure, `itext` translations, reconstructed choice lists, entities, and
//! data-only nodes such as `calculate`. Use it to import or inspect an XForm.

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

/// A `<bind>`'s recovered attributes, keyed by nodeset in [`collect_binds`].
#[derive(Default, Clone)]
struct Bind {
    ty: Option<String>,
    relevant: Option<String>,
    constraint: Option<String>,
    required: Option<String>,
    readonly: Option<String>,
    calculate: Option<String>,
    constraint_msg: Option<String>,
    required_msg: Option<String>,
}

/// `nodeset` → its [`Bind`], from the model binds.
fn collect_binds(xml: &str) -> Result<HashMap<String, Bind>, Xform2JsonError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut binds = HashMap::new();
    loop {
        match read(&mut reader)? {
            Event::Eof => break,
            Event::Start(e) | Event::Empty(e) if local_name(e.name().as_ref()) == "bind" => {
                if let Some(nodeset) = attr(&e, "nodeset") {
                    binds.insert(
                        nodeset,
                        Bind {
                            ty: attr(&e, "type"),
                            relevant: attr(&e, "relevant"),
                            constraint: attr(&e, "constraint"),
                            required: attr(&e, "required"),
                            readonly: attr(&e, "readonly"),
                            calculate: attr(&e, "calculate"),
                            constraint_msg: attr(&e, "constraintMsg"),
                            required_msg: attr(&e, "requiredMsg"),
                        },
                    );
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
    appearance: Option<String>,
    list: Option<String>,
    label: Option<String>,
    hint: Option<String>,
}

/// Walk the body controls into a flat list of questions.
fn collect_questions(
    xml: &str,
    binds: &HashMap<String, Bind>,
) -> Result<Vec<Question>, Xform2JsonError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut questions = Vec::new();
    let mut current: Option<Partial> = None;
    let mut in_label = false;
    let mut in_hint = false;
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
                        appearance: attr(&e, "appearance"),
                        list: None,
                        label: None,
                        hint: None,
                    });
                } else if name == "itemset" {
                    in_itemset = true;
                    if let Some(p) = current.as_mut() {
                        p.list = attr(&e, "nodeset").and_then(|n| instance_id(&n));
                    }
                } else if name == "label" && current.is_some() && !in_itemset {
                    in_label = true;
                } else if name == "hint" && current.is_some() {
                    in_hint = true;
                }
            }
            Event::Text(t) if in_label => {
                if let Some(p) = current.as_mut() {
                    p.label = Some(t.unescape().map_err(xml_err)?.into_owned());
                }
            }
            Event::Text(t) if in_hint => {
                if let Some(p) = current.as_mut() {
                    p.hint = Some(t.unescape().map_err(xml_err)?.into_owned());
                }
            }
            Event::End(e) => match local_name(e.name().as_ref()).as_str() {
                "label" => in_label = false,
                "hint" => in_hint = false,
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
fn to_question(p: Partial, binds: &HashMap<String, Bind>) -> Question {
    let name = p.reference.rsplit('/').next().unwrap_or("").to_owned();
    let bind = binds.get(&p.reference).cloned().unwrap_or_default();
    let bind_type = bind.ty.as_deref().unwrap_or("string");
    let kind = infer_kind(&p.tag, bind_type, p.list, p.mediatype.as_deref());

    Question {
        kind,
        name,
        label: single_lang(p.label),
        hint: single_lang(p.hint),
        guidance_hint: Localized::default(),
        appearance: p.appearance,
        calculation: bind.calculate,
        relevant: bind.relevant,
        constraint: bind.constraint,
        required: bind.required,
        readonly: bind.readonly,
        constraint_message: single_lang(bind.constraint_msg),
        required_message: single_lang(bind.required_msg),
        parameters: Vec::new(),
        media: Vec::new(),
        default: None,
        choice_filter: None,
        save_to: None,
        trigger: None,
    }
}

/// Wrap an optional single-language string into a [`Localized`].
fn single_lang(text: Option<String>) -> Localized {
    Localized {
        default: text,
        langs: Vec::new(),
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

/// An attribute value as an owned, entity-decoded `String`, if present.
///
/// Decoding matters: a recovered `&gt;` must become `>` so the forward emitter
/// re-escapes it once (not to `&amp;gt;`), keeping round-trips byte-identical.
fn attr(e: &quick_xml::events::BytesStart<'_>, key: &str) -> Option<String> {
    e.attributes().flatten().find_map(|a| {
        (local_name(a.key.as_ref()) == key).then(|| {
            a.unescape_value()
                .map(std::borrow::Cow::into_owned)
                .unwrap_or_else(|_| String::from_utf8_lossy(a.value.as_ref()).into_owned())
        })
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

    /// Compile markdown to an XForm, parse it back, re-emit, and return both
    /// XForms. They must be byte-identical for the form to round-trip.
    fn round_trip(md: &str) -> (String, String) {
        let workbook = rustxform_reader::read_markdown(md).unwrap();
        let survey = rustxform_parse::workbook_to_survey(&workbook).unwrap();
        let first = rustxform_xform::survey_to_xform(&survey).unwrap();
        let recovered = xform_to_survey(&first).unwrap();
        let second = rustxform_xform::survey_to_xform(&recovered).unwrap();
        (first, second)
    }

    #[test]
    fn round_trips_flat_forms() {
        let forms = [
            // Question types with inline labels.
            "| survey |\n|  | type | name | label |\n|  | text | a | A |\n\
             |  | integer | b | B |\n|  | decimal | c | C |\n|  | date | d | D |\n\
             |  | geopoint | e | E |\n|  | barcode | f | F |\n",
            // Logic columns, recovered as literal XPath (note the `>`).
            "| survey |\n|  | type | name | label | relevant | constraint | required |\n\
             |  | integer | age | Age |  | . > 0 | yes |\n\
             |  | text | nm | Nm | /data/age > 18 |  |  |\n",
            // Messages, hint and appearance.
            "| survey |\n|  | type | name | label | hint | appearance | constraint | constraint_message |\n\
             |  | integer | q | Q | Enter a number | multiline | . > 0 | Must be positive |\n",
            // A note re-emits identically (read-only string input).
            "| survey |\n|  | type | name | label |\n|  | note | n | Read me |\n|  | text | q | Q |\n",
        ];
        for md in forms {
            let (first, second) = round_trip(md);
            assert_eq!(first, second, "round-trip differs for form:\n{md}");
        }
    }
}
