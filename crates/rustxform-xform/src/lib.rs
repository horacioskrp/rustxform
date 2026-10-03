//! Emit XForm XML from the [`Survey`] model.
//!
//! Produces the `<model>` (optional `<itext>`, primary instance, choice
//! secondary instances and binds) and the `<body>` controls. A single-language
//! form uses inline labels; a multi-language form switches to `<itext>` with
//! `jr:itext(...)` references.

use std::collections::HashMap;

use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use rustxform_core::{
    Container, Control, Kind, Localized, Media, Node, Question, SelectType, Survey,
};
use rustxform_expr::rewrite_references;

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
    let multilingual = !survey.languages.is_empty();

    let mut w = Writer::new(Vec::new());
    w.write_event(Event::Decl(BytesDecl::new("1.0", None, None)))?;

    let mut html = BytesStart::new("h:html");
    for (prefix, uri) in NAMESPACES {
        html.push_attribute((*prefix, *uri));
    }
    if survey.entity.is_some() {
        html.push_attribute((
            "xmlns:entities",
            "http://www.opendatakit.org/xforms/entities",
        ));
    }
    w.write_event(Event::Start(html))?;

    open(&mut w, "h:head")?;
    text_element(&mut w, "h:title", title)?;

    let mut model = BytesStart::new("model");
    model.push_attribute(("odk:xforms-version", "1.0.0"));
    if survey.entity.is_some() {
        model.push_attribute(("entities:entities-version", "2024.1.0"));
    }
    w.write_event(Event::Start(model))?;

    let itext_langs: Vec<&str> = if multilingual {
        survey.languages.iter().map(String::as_str).collect()
    } else {
        vec!["default"]
    };
    if multilingual || any_media(&survey.children) {
        write_itext(&mut w, survey, &itext_langs, multilingual)?;
    }
    write_primary_instance(&mut w, survey, form_id)?;
    write_choice_instances(&mut w, survey, multilingual)?;
    if uses_last_saved(&survey.children) {
        let mut instance = BytesStart::new("instance");
        instance.push_attribute(("id", "__last-saved"));
        instance.push_attribute(("src", "jr://instance/last-saved"));
        w.write_event(Event::Empty(instance))?;
    }
    let index = build_index(&survey.children);
    write_binds(
        &mut w,
        &survey.children,
        &format!("/{ROOT}"),
        &index,
        multilingual,
    )?;
    if survey.entity.is_some() {
        write_entity_binds(&mut w, survey, &index)?;
    }
    write_meta_binds(&mut w, survey)?;

    close(&mut w, "model")?;
    close(&mut w, "h:head")?;

    let mut body = BytesStart::new("h:body");
    if let Some(style) = &survey.settings.style {
        body.push_attribute(("class", style.as_str()));
    }
    w.write_event(Event::Start(body))?;
    write_body(
        &mut w,
        &survey.children,
        &format!("/{ROOT}"),
        multilingual,
        &index,
    )?;
    close(&mut w, "h:body")?;

    close(&mut w, "h:html")?;

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

// --- Translations (itext) ---------------------------------------------------

/// The default language: the configured one if declared, else the first.
fn default_language(survey: &Survey) -> Option<&str> {
    match survey.settings.default_language.as_deref() {
        Some(d) if survey.languages.iter().any(|l| l == d) => Some(d),
        _ => survey.languages.first().map(String::as_str),
    }
}

/// Does any question anywhere in the tree carry label media?
fn any_media(nodes: &[Node]) -> bool {
    nodes.iter().any(|node| match node {
        Node::Question(q) => !q.media.is_empty(),
        Node::Group(c) | Node::Repeat(c) => any_media(&c.children),
    })
}

/// Whether a question's label is rendered via itext (multilingual or media).
fn label_uses_itext(q: &Question, ml: bool) -> bool {
    (ml && q.label.is_multilingual()) || !q.media.is_empty()
}

/// Whether a question's hint is rendered via itext.
fn hint_uses_itext(q: &Question, ml: bool) -> bool {
    ml && q.hint.is_multilingual()
}

/// Emit the `<itext>` block: one `<translation>` per language, choice texts
/// first (multilingual only), then question label/hint texts.
fn write_itext(w: &mut W, survey: &Survey, langs: &[&str], ml: bool) -> Result<(), XformError> {
    let default = if ml {
        default_language(survey)
    } else {
        Some("default")
    };
    let mut selects: Vec<(&str, Option<&str>)> = Vec::new();
    collect_selects(&survey.children, &mut selects);

    open(w, "itext")?;
    for &lang in langs {
        let mut translation = BytesStart::new("translation");
        translation.push_attribute(("lang", lang));
        if Some(lang) == default {
            translation.push_attribute(("default", "true()"));
        }
        w.write_event(Event::Start(translation))?;

        if ml {
            for (list, file) in &selects {
                if file.is_some() {
                    continue; // external files carry their own labels
                }
                if let Some(choice_list) = survey.choice_list(list) {
                    for (i, item) in choice_list.items.iter().enumerate() {
                        let id = format!("{list}-{i}");
                        write_text(w, &id, item.label.for_lang(lang).unwrap_or_default())?;
                    }
                }
            }
        }
        write_itext_questions(w, &survey.children, &format!("/{ROOT}"), lang, ml)?;

        close(w, "translation")?;
    }
    close(w, "itext")?;
    Ok(())
}

/// Emit `<text>` entries for questions whose label/hint use itext.
fn write_itext_questions(
    w: &mut W,
    nodes: &[Node],
    parent: &str,
    lang: &str,
    ml: bool,
) -> Result<(), XformError> {
    for node in nodes {
        match node {
            Node::Question(q) => {
                let path = format!("{parent}/{}", q.name);
                if q.constraint_message.is_multilingual() {
                    write_text(
                        w,
                        &format!("{path}:jr:constraintMsg"),
                        q.constraint_message.for_lang(lang).unwrap_or_default(),
                    )?;
                }
                if q.required_message.is_multilingual() {
                    write_text(
                        w,
                        &format!("{path}:jr:requiredMsg"),
                        q.required_message.for_lang(lang).unwrap_or_default(),
                    )?;
                }
                if label_uses_itext(q, ml) {
                    let value = if ml {
                        q.label.for_lang(lang).unwrap_or_default()
                    } else {
                        q.label.single().unwrap_or_default()
                    };
                    write_label_text(w, &format!("{path}:label"), value, &q.media, lang, ml)?;
                }
                if hint_uses_itext(q, ml) {
                    write_text(
                        w,
                        &format!("{path}:hint"),
                        q.hint.for_lang(lang).unwrap_or_default(),
                    )?;
                }
            }
            Node::Group(c) | Node::Repeat(c) => {
                write_itext_questions(w, &c.children, &format!("{parent}/{}", c.name), lang, ml)?;
            }
        }
    }
    Ok(())
}

/// Emit `<text id><value>…</value>[<value form="..">jr://..</value>]</text>`.
fn write_label_text(
    w: &mut W,
    id: &str,
    value: &str,
    media: &[Media],
    lang: &str,
    ml: bool,
) -> Result<(), XformError> {
    let mut text = BytesStart::new("text");
    text.push_attribute(("id", id));
    w.write_event(Event::Start(text))?;
    text_element(w, "value", value)?;
    for entry in media {
        let file = if ml {
            entry.files.for_lang(lang)
        } else {
            entry.files.single()
        };
        let Some(file) = file else {
            continue;
        };
        let directory = match entry.form.as_str() {
            "image" => "images",
            "audio" => "audio",
            "video" => "video",
            other => other,
        };
        let mut element = BytesStart::new("value");
        element.push_attribute(("form", entry.form.as_str()));
        w.write_event(Event::Start(element))?;
        w.write_event(Event::Text(BytesText::new(&format!(
            "jr://{directory}/{file}"
        ))))?;
        close(w, "value")?;
    }
    close(w, "text")?;
    Ok(())
}

/// Emit a single `<text id="..">` with a `<value>`.
fn write_text(w: &mut W, id: &str, value: &str) -> Result<(), XformError> {
    let mut text = BytesStart::new("text");
    text.push_attribute(("id", id));
    w.write_event(Event::Start(text))?;
    text_element(w, "value", value)?;
    close(w, "text")?;
    Ok(())
}

// --- Primary instance -------------------------------------------------------

/// `<instance><data id="..">…tree…<meta>…</meta></data></instance>`.
fn write_primary_instance(w: &mut W, survey: &Survey, form_id: &str) -> Result<(), XformError> {
    open(w, "instance")?;
    let mut root = BytesStart::new(ROOT);
    root.push_attribute(("id", form_id));
    if let Some(version) = &survey.settings.version {
        root.push_attribute(("version", version.as_str()));
    }
    w.write_event(Event::Start(root))?;

    for node in &survey.children {
        write_instance_node(w, node)?;
    }

    open(w, "meta")?;
    if let Some(entity) = &survey.entity {
        let mut element = BytesStart::new("entity");
        element.push_attribute(("dataset", entity.dataset.as_str()));
        element.push_attribute(("create", "1"));
        element.push_attribute(("id", ""));
        w.write_event(Event::Start(element))?;
        w.write_event(Event::Empty(BytesStart::new("label")))?;
        close(w, "entity")?;
    }
    w.write_event(Event::Empty(BytesStart::new("instanceID")))?;
    if survey.settings.instance_name.is_some() {
        w.write_event(Event::Empty(BytesStart::new("instanceName")))?;
    }
    close(w, "meta")?;
    close(w, ROOT)?;
    close(w, "instance")?;
    Ok(())
}

/// Emit a node into the primary instance.
fn write_instance_node(w: &mut W, node: &Node) -> Result<(), XformError> {
    match node {
        Node::Question(q) => match &q.default {
            Some(value) => text_element(w, &q.name, value)?,
            None => w.write_event(Event::Empty(BytesStart::new(q.name.as_str())))?,
        },
        Node::Group(g) => write_instance_container(w, g, false)?,
        Node::Repeat(r) => {
            // `repeat_count` synthesizes a sibling `<name_count>` calculate node.
            if r.count.is_some() {
                w.write_event(Event::Empty(BytesStart::new(
                    format!("{}_count", r.name).as_str(),
                )))?;
            }
            // A repeat emits its template plus one live instance.
            write_instance_container(w, r, true)?;
            write_instance_container(w, r, false)?;
        }
    }
    Ok(())
}

/// Emit `<name [jr:template=""]>children…</name>` into the instance.
fn write_instance_container(w: &mut W, c: &Container, template: bool) -> Result<(), XformError> {
    let mut start = BytesStart::new(c.name.as_str());
    if template {
        start.push_attribute(("jr:template", ""));
    }
    w.write_event(Event::Start(start))?;
    for node in &c.children {
        write_instance_node(w, node)?;
    }
    w.write_event(Event::End(BytesEnd::new(c.name.as_str())))?;
    Ok(())
}

// --- Choice secondary instances --------------------------------------------

/// Secondary instances for each referenced choice list, in first-use order.
fn write_choice_instances(
    w: &mut W,
    survey: &Survey,
    multilingual: bool,
) -> Result<(), XformError> {
    let mut seen: Vec<(&str, Option<&str>)> = Vec::new();
    collect_selects(&survey.children, &mut seen);

    for (list, file) in seen {
        if let Some(file) = file {
            // External source: an empty instance pointing at the file.
            let directory = if file.ends_with(".csv") {
                "file-csv"
            } else {
                "file"
            };
            let mut instance = BytesStart::new("instance");
            instance.push_attribute(("id", list));
            instance.push_attribute(("src", format!("jr://{directory}/{file}").as_str()));
            w.write_event(Event::Empty(instance))?;
            continue;
        }

        let Some(choice_list) = survey.choice_list(list) else {
            continue;
        };
        let mut instance = BytesStart::new("instance");
        instance.push_attribute(("id", list));
        w.write_event(Event::Start(instance))?;
        open(w, "root")?;
        for (i, choice) in choice_list.items.iter().enumerate() {
            open(w, "item")?;
            if multilingual {
                text_element(w, "itextId", &format!("{list}-{i}"))?;
                text_element(w, "name", &choice.name)?;
            } else {
                text_element(w, "name", &choice.name)?;
                if let Some(label) = choice.label.single() {
                    text_element(w, "label", label)?;
                }
            }
            for (key, value) in &choice.extra {
                text_element(w, key, value)?;
            }
            close(w, "item")?;
        }
        close(w, "root")?;
        close(w, "instance")?;
    }
    Ok(())
}

/// Collect referenced selects in document order as `(instance id, file)`,
/// de-duplicated by id. `file` is `Some` for external `select_*_from_file`.
fn collect_selects<'a>(nodes: &'a [Node], seen: &mut Vec<(&'a str, Option<&'a str>)>) {
    for node in nodes {
        match node {
            Node::Question(q) => {
                if let Kind::Select { list, file, .. } = &q.kind {
                    if !seen.iter().any(|(id, _)| *id == list.as_str()) {
                        seen.push((list.as_str(), file.as_deref()));
                    }
                }
            }
            Node::Group(c) | Node::Repeat(c) => collect_selects(&c.children, seen),
        }
    }
}

// --- Binds ------------------------------------------------------------------

/// Emit binds for every leaf question under `parent`, depth-first.
fn write_binds(
    w: &mut W,
    nodes: &[Node],
    parent: &str,
    index: &HashMap<String, Vec<Step>>,
    ml: bool,
) -> Result<(), XformError> {
    for node in nodes {
        match node {
            Node::Question(q) => {
                let path = format!("{parent}/{}", q.name);
                write_bind(w, q, &path, index, ml)?;
                write_action(w, q, &path)?;
            }
            Node::Group(c) => {
                write_binds(w, &c.children, &format!("{parent}/{}", c.name), index, ml)?;
            }
            Node::Repeat(c) => {
                if let Some(count) = &c.count {
                    let mut bind = BytesStart::new("bind");
                    bind.push_attribute(("nodeset", format!("{parent}/{}_count", c.name).as_str()));
                    bind.push_attribute(("type", "string"));
                    bind.push_attribute(("readonly", "true()"));
                    bind.push_attribute(("calculate", count.as_str()));
                    w.write_event(Event::Empty(bind))?;
                }
                write_binds(w, &c.children, &format!("{parent}/{}", c.name), index, ml)?;
            }
        }
    }
    Ok(())
}

/// Write the `<bind nodeset="..">` for a question.
fn write_bind(
    w: &mut W,
    question: &Question,
    nodeset: &str,
    index: &HashMap<String, Vec<Step>>,
    ml: bool,
) -> Result<(), XformError> {
    let empty: Vec<Step> = Vec::new();
    let context = index.get(&question.name).unwrap_or(&empty);
    let resolve = |name: &str| resolve_ref(index, context, name);

    let mut bind = BytesStart::new("bind");
    bind.push_attribute(("nodeset", nodeset));

    match &question.kind {
        Kind::Builtin(b) => {
            if let Some(p) = b.preload {
                bind.push_attribute(("jr:preload", p.preload));
                bind.push_attribute(("type", b.bind_type));
                bind.push_attribute(("jr:preloadParams", p.params));
            } else {
                bind.push_attribute(("type", b.bind_type));
                push_logic_attributes(&mut bind, question, &resolve, b.readonly, nodeset, ml);
            }
        }
        Kind::Select { select, .. } => {
            let bind_type = match select {
                SelectType::Rank => "odk:rank",
                _ => "string",
            };
            bind.push_attribute(("type", bind_type));
            push_logic_attributes(&mut bind, question, &resolve, false, nodeset, ml);
        }
        Kind::Unknown(_) => {
            bind.push_attribute(("type", "string"));
            push_logic_attributes(&mut bind, question, &resolve, false, nodeset, ml);
        }
    }
    if let Some(save_to) = &question.save_to {
        bind.push_attribute(("entities:saveto", save_to.as_str()));
    }
    w.write_event(Event::Empty(bind))?;
    Ok(())
}

/// Emit a model-level action (e.g. `<odk:recordaudio>`) right after a
/// question's bind, when its type declares one.
fn write_action(w: &mut W, q: &Question, reference: &str) -> Result<(), XformError> {
    let Kind::Builtin(b) = &q.kind else {
        return Ok(());
    };
    let Some(action) = b.action else {
        return Ok(());
    };
    let mut element = BytesStart::new(action.element);
    element.push_attribute(("ref", reference));
    element.push_attribute(("event", action.event));
    if let Some(quality) = q.parameter("quality") {
        element.push_attribute(("odk:quality", quality));
    }
    w.write_event(Event::Empty(element))?;
    Ok(())
}

/// Emit the entity binds: the `@id` bind + its `uuid()` setvalue, and the
/// label bind when a label expression is declared.
fn write_entity_binds(
    w: &mut W,
    survey: &Survey,
    index: &HashMap<String, Vec<Step>>,
) -> Result<(), XformError> {
    let Some(entity) = &survey.entity else {
        return Ok(());
    };

    let mut id_bind = BytesStart::new("bind");
    id_bind.push_attribute(("nodeset", format!("/{ROOT}/meta/entity/@id").as_str()));
    id_bind.push_attribute(("readonly", "true()"));
    id_bind.push_attribute(("type", "string"));
    w.write_event(Event::Empty(id_bind))?;

    let mut setvalue = BytesStart::new("setvalue");
    setvalue.push_attribute(("ref", format!("/{ROOT}/meta/entity/@id").as_str()));
    setvalue.push_attribute(("event", "odk-instance-first-load"));
    setvalue.push_attribute(("value", "uuid()"));
    w.write_event(Event::Empty(setvalue))?;

    if let Some(label) = &entity.label {
        let calculate = rewrite_references(label, |name: &str| resolve_ref(index, &[], name));
        let mut label_bind = BytesStart::new("bind");
        label_bind.push_attribute(("nodeset", format!("/{ROOT}/meta/entity/label").as_str()));
        label_bind.push_attribute(("calculate", calculate.as_str()));
        label_bind.push_attribute(("readonly", "true()"));
        label_bind.push_attribute(("type", "string"));
        w.write_event(Event::Empty(label_bind))?;
    }
    Ok(())
}

/// Add the `readonly`/`required`/`relevant`/`constraint`/`calculate`
/// attributes, rewriting `${…}` references via `resolve`.
fn push_logic_attributes(
    bind: &mut BytesStart<'_>,
    q: &Question,
    resolve: &impl Fn(&str) -> Option<String>,
    base_readonly: bool,
    nodeset: &str,
    ml: bool,
) {
    if let Some(value) = readonly_value(q, base_readonly, resolve) {
        bind.push_attribute(("readonly", value.as_str()));
    }
    if let Some(value) = flag_value(q.required.as_deref(), resolve) {
        bind.push_attribute(("required", value.as_str()));
    }
    push_message(bind, "jr:requiredMsg", &q.required_message, nodeset, ml);
    if let Some(expr) = &q.relevant {
        bind.push_attribute(("relevant", rewrite_references(expr, resolve).as_str()));
    }
    if let Some(expr) = &q.constraint {
        bind.push_attribute(("constraint", rewrite_references(expr, resolve).as_str()));
    }
    push_message(bind, "jr:constraintMsg", &q.constraint_message, nodeset, ml);
    if let Some(expr) = &q.calculation {
        bind.push_attribute(("calculate", rewrite_references(expr, resolve).as_str()));
    }
}

/// Push a `jr:*Msg` attribute: an itext reference when multilingual, else the
/// plain single-language text.
fn push_message(bind: &mut BytesStart<'_>, attr: &str, msg: &Localized, nodeset: &str, ml: bool) {
    if ml && msg.is_multilingual() {
        bind.push_attribute((attr, format!("jr:itext('{nodeset}:{attr}')").as_str()));
    } else if let Some(text) = msg.single() {
        bind.push_attribute((attr, text));
    }
}

/// Effective `readonly` value: forced by type, or from the `read_only` column.
fn readonly_value(
    q: &Question,
    base_readonly: bool,
    resolve: &impl Fn(&str) -> Option<String>,
) -> Option<String> {
    if base_readonly {
        return Some("true()".to_owned());
    }
    flag_value(q.readonly.as_deref(), resolve)
}

/// Resolve a boolean-ish column: `yes`/`true` → `true()`, `no`/`false` → none,
/// otherwise a rewritten expression.
fn flag_value(raw: Option<&str>, resolve: &impl Fn(&str) -> Option<String>) -> Option<String> {
    let value = raw?;
    let lowered = value.trim().to_ascii_lowercase();
    match lowered.as_str() {
        "yes" | "true" | "true()" => Some("true()".to_owned()),
        "no" | "false" | "false()" | "" => None,
        _ => Some(rewrite_references(value, resolve)),
    }
}

/// Write the `meta` binds: the fixed `instanceID`, and `instanceName` when a
/// form `instance_name` is set.
fn write_meta_binds(w: &mut W, survey: &Survey) -> Result<(), XformError> {
    let mut bind = BytesStart::new("bind");
    bind.push_attribute(("nodeset", format!("/{ROOT}/meta/instanceID").as_str()));
    bind.push_attribute(("type", "string"));
    bind.push_attribute(("readonly", "true()"));
    bind.push_attribute(("jr:preload", "uid"));
    w.write_event(Event::Empty(bind))?;

    if let Some(name_expr) = &survey.settings.instance_name {
        let mut bind = BytesStart::new("bind");
        bind.push_attribute(("nodeset", format!("/{ROOT}/meta/instanceName").as_str()));
        bind.push_attribute(("type", "string"));
        bind.push_attribute(("calculate", name_expr.as_str()));
        w.write_event(Event::Empty(bind))?;
    }
    Ok(())
}

/// A step on the path from the instance root to a node.
#[derive(Clone)]
struct Step {
    name: String,
    repeat: bool,
}

/// Index every question by name to its path of [`Step`]s from the root.
fn build_index(nodes: &[Node]) -> HashMap<String, Vec<Step>> {
    let mut index = HashMap::new();
    let mut prefix: Vec<Step> = Vec::new();
    index_nodes(nodes, &mut prefix, &mut index);
    index
}

fn index_nodes(nodes: &[Node], prefix: &mut Vec<Step>, index: &mut HashMap<String, Vec<Step>>) {
    for node in nodes {
        match node {
            Node::Question(q) => {
                let mut path = prefix.clone();
                path.push(Step {
                    name: q.name.clone(),
                    repeat: false,
                });
                index.insert(q.name.clone(), path);
            }
            Node::Group(c) => {
                prefix.push(Step {
                    name: c.name.clone(),
                    repeat: false,
                });
                index_nodes(&c.children, prefix, index);
                prefix.pop();
            }
            Node::Repeat(c) => {
                prefix.push(Step {
                    name: c.name.clone(),
                    repeat: true,
                });
                index_nodes(&c.children, prefix, index);
                prefix.pop();
            }
        }
    }
}

/// XPath from a `context` node to a `target` node: relative when they share a
/// repeat ancestor, absolute otherwise.
fn reference_xpath(context: &[Step], target: &[Step]) -> String {
    let mut common = 0;
    while common < context.len()
        && common < target.len()
        && context[common].name == target[common].name
    {
        common += 1;
    }

    let shares_repeat = target[..common].iter().any(|s| s.repeat);
    if shares_repeat {
        let ups = context.len() - common;
        let mut parts: Vec<&str> = Vec::with_capacity(ups + target.len() - common);
        parts.extend(std::iter::repeat_n("..", ups));
        parts.extend(target[common..].iter().map(|s| s.name.as_str()));
        parts.join("/")
    } else {
        let mut path = format!("/{ROOT}");
        for step in target {
            path.push('/');
            path.push_str(&step.name);
        }
        path
    }
}

// --- Body -------------------------------------------------------------------

/// Emit body controls for `nodes` under `parent`.
fn write_body(
    w: &mut W,
    nodes: &[Node],
    parent: &str,
    ml: bool,
    index: &HashMap<String, Vec<Step>>,
) -> Result<(), XformError> {
    for node in nodes {
        match node {
            Node::Question(q) => write_control(w, q, &format!("{parent}/{}", q.name), ml, index)?,
            Node::Group(g) => {
                write_group(w, g, &format!("{parent}/{}", g.name), false, ml, index)?;
            }
            Node::Repeat(r) => {
                write_group(w, r, &format!("{parent}/{}", r.name), true, ml, index)?;
            }
        }
    }
    Ok(())
}

/// Write `<group ref="..">`, with an inner `<repeat>` when `repeat` is set.
fn write_group(
    w: &mut W,
    c: &Container,
    path: &str,
    repeat: bool,
    ml: bool,
    index: &HashMap<String, Vec<Step>>,
) -> Result<(), XformError> {
    let mut group = BytesStart::new("group");
    group.push_attribute(("ref", path));
    if let Some(appearance) = &c.appearance {
        group.push_attribute(("appearance", appearance.as_str()));
    }
    w.write_event(Event::Start(group))?;
    if let Some(label) = &c.label {
        text_element(w, "label", label)?;
    }

    if repeat {
        let mut r = BytesStart::new("repeat");
        r.push_attribute(("nodeset", path));
        if c.count.is_some() {
            r.push_attribute(("jr:count", format!(" {path}_count ").as_str()));
        }
        w.write_event(Event::Start(r))?;
        write_body(w, &c.children, path, ml, index)?;
        close(w, "repeat")?;
    } else {
        write_body(w, &c.children, path, ml, index)?;
    }

    close(w, "group")?;
    Ok(())
}

/// Write the body control for a question (nothing for data-only types).
fn write_control(
    w: &mut W,
    question: &Question,
    reference: &str,
    ml: bool,
    index: &HashMap<String, Vec<Step>>,
) -> Result<(), XformError> {
    match &question.kind {
        Kind::Builtin(b) => match b.control {
            None => Ok(()),
            Some(Control::Input) => control(w, "input", reference, question, ml, index, &[]),
            Some(Control::Trigger) => control(w, "trigger", reference, question, ml, index, &[]),
            Some(Control::Upload { mediatype }) => control(
                w,
                "upload",
                reference,
                question,
                ml,
                index,
                &[("mediatype", mediatype)],
            ),
            Some(Control::Range) => write_range(w, reference, question, ml, index),
        },
        Kind::Select { .. } => write_select(w, reference, question, ml, index),
        Kind::Unknown(_) => Ok(()),
    }
}

/// Write `<tag ref=".." [appearance] extra..>[label][hint]</tag>`.
fn control(
    w: &mut W,
    tag: &str,
    reference: &str,
    question: &Question,
    ml: bool,
    index: &HashMap<String, Vec<Step>>,
    extra: &[(&str, &str)],
) -> Result<(), XformError> {
    let mut element = BytesStart::new(tag);
    element.push_attribute(("ref", reference));
    if let Some(appearance) = &question.appearance {
        element.push_attribute(("appearance", appearance.as_str()));
    }
    for attr in extra {
        element.push_attribute(*attr);
    }
    w.write_event(Event::Start(element))?;
    write_label(w, question, reference, ml, index)?;
    write_hint(w, question, reference, ml)?;
    close(w, tag)?;
    Ok(())
}

/// Write a `<range>` control with `start`/`end`/`step` from `parameters`.
fn write_range(
    w: &mut W,
    reference: &str,
    question: &Question,
    ml: bool,
    index: &HashMap<String, Vec<Step>>,
) -> Result<(), XformError> {
    let mut element = BytesStart::new("range");
    element.push_attribute(("ref", reference));
    if let Some(appearance) = &question.appearance {
        element.push_attribute(("appearance", appearance.as_str()));
    }
    for key in ["start", "end", "step"] {
        if let Some(value) = question.parameter(key) {
            element.push_attribute((key, value));
        }
    }
    w.write_event(Event::Start(element))?;
    write_label(w, question, reference, ml, index)?;
    write_hint(w, question, reference, ml)?;
    close(w, "range")?;
    Ok(())
}

/// Write a `<select1>`/`<select>` control with an itemset over a list instance.
fn write_select(
    w: &mut W,
    reference: &str,
    question: &Question,
    ml: bool,
    index: &HashMap<String, Vec<Step>>,
) -> Result<(), XformError> {
    let Kind::Select { select, list, file } = &question.kind else {
        return Ok(());
    };
    let external = file.is_some();

    let tag = match select {
        SelectType::One => "select1",
        SelectType::Multiple => "select",
        SelectType::Rank => "odk:rank",
    };
    let mut element = BytesStart::new(tag);
    element.push_attribute(("ref", reference));
    if let Some(appearance) = &question.appearance {
        element.push_attribute(("appearance", appearance.as_str()));
    }
    w.write_event(Event::Start(element))?;
    write_label(w, question, reference, ml, index)?;
    write_hint(w, question, reference, ml)?;

    // A `choice_filter` adds an XPath predicate to the itemset nodeset.
    let predicate = question
        .choice_filter
        .as_ref()
        .map(|filter| {
            let empty: Vec<Step> = Vec::new();
            let context = index.get(&question.name).unwrap_or(&empty);
            let resolve = |name: &str| resolve_ref(index, context, name);
            format!("[{}]", rewrite_references(filter, resolve))
        })
        .unwrap_or_default();

    // Raw content keeps the apostrophes in the nodeset literal (not `&apos;`).
    let itemset = BytesStart::from_content(
        format!("itemset nodeset=\"instance('{list}')/root/item{predicate}\""),
        "itemset".len(),
    );
    w.write_event(Event::Start(itemset))?;
    ref_element(w, "value", "name")?;
    if ml && !external {
        ref_element(w, "label", "jr:itext(itextId)")?;
    } else {
        ref_element(w, "label", "label")?;
    }
    close(w, "itemset")?;

    close(w, tag)?;
    Ok(())
}

/// Write a control's `<label>`: an itext reference when multilingual, an
/// `<output>`-bearing mixed label when it has `${…}`, else plain text.
fn write_label(
    w: &mut W,
    q: &Question,
    reference: &str,
    ml: bool,
    index: &HashMap<String, Vec<Step>>,
) -> Result<(), XformError> {
    if label_uses_itext(q, ml) {
        write_itext_ref(w, "label", &format!("{reference}:label"))?;
    } else if let Some(text) = q.label.single() {
        if text.contains("${") {
            write_output_label(w, text, q, index)?;
        } else {
            text_element(w, "label", text)?;
        }
    }
    Ok(())
}

/// Write a `<label>` whose `${name}` references become `<output>` elements.
fn write_output_label(
    w: &mut W,
    text: &str,
    question: &Question,
    index: &HashMap<String, Vec<Step>>,
) -> Result<(), XformError> {
    open(w, "label")?;
    let mut rest = text;
    while let Some(start) = rest.find("${") {
        let (literal, tail) = rest.split_at(start);
        if !literal.is_empty() {
            w.write_event(Event::Text(BytesText::new(literal)))?;
        }
        let tail = &tail[2..];
        match tail.find('}') {
            Some(end) => {
                let name = &tail[..end];
                match resolve_reference(index, &question.name, name) {
                    Some(xpath) => {
                        let mut output = BytesStart::new("output");
                        output.push_attribute(("value", format!(" {xpath} ").as_str()));
                        w.write_event(Event::Empty(output))?;
                    }
                    None => w.write_event(Event::Text(BytesText::new(&format!("${{{name}}}"))))?,
                }
                rest = &tail[end + 1..];
            }
            None => {
                w.write_event(Event::Text(BytesText::new("${")))?;
                rest = tail;
            }
        }
    }
    if !rest.is_empty() {
        w.write_event(Event::Text(BytesText::new(rest)))?;
    }
    close(w, "label")?;
    Ok(())
}

/// Resolve `${target}` referenced from the question named `context`.
fn resolve_reference(
    index: &HashMap<String, Vec<Step>>,
    context: &str,
    target: &str,
) -> Option<String> {
    let empty: Vec<Step> = Vec::new();
    let context = index.get(context).unwrap_or(&empty);
    resolve_ref(index, context, target)
}

/// Resolve a `${name}` reference (including `last-saved#name`) to its XPath.
fn resolve_ref(index: &HashMap<String, Vec<Step>>, context: &[Step], name: &str) -> Option<String> {
    if let Some(rest) = name.strip_prefix("last-saved#") {
        let target = index.get(rest)?;
        // Absolute path into the last-saved secondary instance.
        return Some(format!(
            "instance('__last-saved'){}",
            reference_xpath(&[], target)
        ));
    }
    let target = index.get(name)?;
    Some(reference_xpath(context, target))
}

/// Does any expression reference `last-saved#…`?
fn uses_last_saved(nodes: &[Node]) -> bool {
    nodes.iter().any(|node| match node {
        Node::Question(q) => {
            [
                &q.relevant,
                &q.constraint,
                &q.required,
                &q.readonly,
                &q.calculation,
            ]
            .into_iter()
            .flatten()
            .flat_map(|e| rustxform_expr::reference_names(e))
            .any(|r| r.starts_with("last-saved#"))
                || q.label
                    .single()
                    .map(|t| t.contains("${last-saved#"))
                    .unwrap_or(false)
        }
        Node::Group(c) | Node::Repeat(c) => uses_last_saved(&c.children),
    })
}

/// Write a control's `<hint>` when present.
fn write_hint(w: &mut W, q: &Question, reference: &str, ml: bool) -> Result<(), XformError> {
    if hint_uses_itext(q, ml) {
        write_itext_ref(w, "hint", &format!("{reference}:hint"))?;
    } else if let Some(text) = q.hint.single() {
        text_element(w, "hint", text)?;
    }
    Ok(())
}

/// Write `<tag ref="jr:itext('id')"/>` with literal apostrophes.
fn write_itext_ref(w: &mut W, tag: &str, id: &str) -> Result<(), XformError> {
    let element = BytesStart::from_content(format!("{tag} ref=\"jr:itext('{id}')\""), tag.len());
    w.write_event(Event::Empty(element))?;
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

/// Write an opening tag with no attributes.
fn open(w: &mut W, tag: &str) -> Result<(), XformError> {
    w.write_event(Event::Start(BytesStart::new(tag)))?;
    Ok(())
}

/// Write a closing tag.
fn close(w: &mut W, tag: &str) -> Result<(), XformError> {
    w.write_event(Event::End(BytesEnd::new(tag)))?;
    Ok(())
}
