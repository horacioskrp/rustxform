//! Emit XForm XML from the [`Survey`] model.
//!
//! Produces the `<model>` (optional `<itext>`, primary instance, choice
//! secondary instances and binds) and the `<body>` controls. A single-language
//! form uses inline labels; a multi-language form switches to `<itext>` with
//! `jr:itext(...)` references.

use std::collections::HashMap;

use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use rustxform_core::{Container, Control, Kind, Node, Question, Survey};
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
    w.write_event(Event::Start(html))?;

    open(&mut w, "h:head")?;
    text_element(&mut w, "h:title", title)?;

    let mut model = BytesStart::new("model");
    model.push_attribute(("odk:xforms-version", "1.0.0"));
    w.write_event(Event::Start(model))?;

    if multilingual {
        write_itext(&mut w, survey)?;
    }
    write_primary_instance(&mut w, survey, form_id)?;
    write_choice_instances(&mut w, survey, multilingual)?;
    let index = build_index(&survey.children);
    write_binds(&mut w, &survey.children, &format!("/{ROOT}"), &index)?;
    write_instance_id_bind(&mut w)?;

    close(&mut w, "model")?;
    close(&mut w, "h:head")?;

    open(&mut w, "h:body")?;
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

/// Emit the `<itext>` block: one `<translation>` per language, choice texts
/// first, then question label/hint texts in document order.
fn write_itext(w: &mut W, survey: &Survey) -> Result<(), XformError> {
    let default = default_language(survey);
    let mut lists: Vec<&str> = Vec::new();
    collect_lists(&survey.children, &mut lists);

    open(w, "itext")?;
    for lang in &survey.languages {
        let mut translation = BytesStart::new("translation");
        translation.push_attribute(("lang", lang.as_str()));
        if Some(lang.as_str()) == default {
            translation.push_attribute(("default", "true()"));
        }
        w.write_event(Event::Start(translation))?;

        for list in &lists {
            if let Some(choice_list) = survey.choice_list(list) {
                for (i, item) in choice_list.items.iter().enumerate() {
                    let id = format!("{list}-{i}");
                    write_text(w, &id, item.label.for_lang(lang).unwrap_or_default())?;
                }
            }
        }
        write_itext_questions(w, &survey.children, &format!("/{ROOT}"), lang)?;

        close(w, "translation")?;
    }
    close(w, "itext")?;
    Ok(())
}

/// Emit `<text id><value>…</value></text>` entries for questions under `parent`.
fn write_itext_questions(
    w: &mut W,
    nodes: &[Node],
    parent: &str,
    lang: &str,
) -> Result<(), XformError> {
    for node in nodes {
        match node {
            Node::Question(q) => {
                let path = format!("{parent}/{}", q.name);
                if q.label.is_multilingual() {
                    write_text(
                        w,
                        &format!("{path}:label"),
                        q.label.for_lang(lang).unwrap_or_default(),
                    )?;
                }
                if q.hint.is_multilingual() {
                    write_text(
                        w,
                        &format!("{path}:hint"),
                        q.hint.for_lang(lang).unwrap_or_default(),
                    )?;
                }
            }
            Node::Group(c) | Node::Repeat(c) => {
                write_itext_questions(w, &c.children, &format!("{parent}/{}", c.name), lang)?;
            }
        }
    }
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
    w.write_event(Event::Start(root))?;

    for node in &survey.children {
        write_instance_node(w, node)?;
    }

    open(w, "meta")?;
    w.write_event(Event::Empty(BytesStart::new("instanceID")))?;
    close(w, "meta")?;
    close(w, ROOT)?;
    close(w, "instance")?;
    Ok(())
}

/// Emit a node into the primary instance.
fn write_instance_node(w: &mut W, node: &Node) -> Result<(), XformError> {
    match node {
        Node::Question(q) => w.write_event(Event::Empty(BytesStart::new(q.name.as_str())))?,
        Node::Group(g) => write_instance_container(w, g, false)?,
        Node::Repeat(r) => {
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
    let mut seen: Vec<&str> = Vec::new();
    collect_lists(&survey.children, &mut seen);

    for list in seen {
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
            close(w, "item")?;
        }
        close(w, "root")?;
        close(w, "instance")?;
    }
    Ok(())
}

/// Collect referenced choice-list names in document order, de-duplicated.
fn collect_lists<'a>(nodes: &'a [Node], seen: &mut Vec<&'a str>) {
    for node in nodes {
        match node {
            Node::Question(q) => {
                if let Kind::Select { list, .. } = &q.kind {
                    if !seen.contains(&list.as_str()) {
                        seen.push(list);
                    }
                }
            }
            Node::Group(c) | Node::Repeat(c) => collect_lists(&c.children, seen),
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
) -> Result<(), XformError> {
    for node in nodes {
        match node {
            Node::Question(q) => write_bind(w, q, &format!("{parent}/{}", q.name), index)?,
            Node::Group(c) | Node::Repeat(c) => {
                write_binds(w, &c.children, &format!("{parent}/{}", c.name), index)?;
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
) -> Result<(), XformError> {
    let empty: Vec<Step> = Vec::new();
    let context = index.get(&question.name).unwrap_or(&empty);
    let resolve = |name: &str| {
        index
            .get(name)
            .map(|target| reference_xpath(context, target))
    };

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
                push_logic_attributes(&mut bind, question, &resolve, b.readonly);
            }
        }
        Kind::Select { .. } | Kind::Unknown(_) => {
            bind.push_attribute(("type", "string"));
            push_logic_attributes(&mut bind, question, &resolve, false);
        }
    }
    w.write_event(Event::Empty(bind))?;
    Ok(())
}

/// Add the `readonly`/`required`/`relevant`/`constraint`/`calculate`
/// attributes, rewriting `${…}` references via `resolve`.
fn push_logic_attributes(
    bind: &mut BytesStart<'_>,
    q: &Question,
    resolve: &impl Fn(&str) -> Option<String>,
    base_readonly: bool,
) {
    if let Some(value) = readonly_value(q, base_readonly, resolve) {
        bind.push_attribute(("readonly", value.as_str()));
    }
    if let Some(value) = flag_value(q.required.as_deref(), resolve) {
        bind.push_attribute(("required", value.as_str()));
    }
    if let Some(msg) = &q.required_message {
        bind.push_attribute(("jr:requiredMsg", msg.as_str()));
    }
    if let Some(expr) = &q.relevant {
        bind.push_attribute(("relevant", rewrite_references(expr, resolve).as_str()));
    }
    if let Some(expr) = &q.constraint {
        bind.push_attribute(("constraint", rewrite_references(expr, resolve).as_str()));
    }
    if let Some(msg) = &q.constraint_message {
        bind.push_attribute(("jr:constraintMsg", msg.as_str()));
    }
    if let Some(expr) = &q.calculation {
        bind.push_attribute(("calculate", rewrite_references(expr, resolve).as_str()));
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
    w.write_event(Event::Start(group))?;
    if let Some(label) = &c.label {
        text_element(w, "label", label)?;
    }

    if repeat {
        let mut r = BytesStart::new("repeat");
        r.push_attribute(("nodeset", path));
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
        Kind::Select { multiple, list } => {
            write_select(w, *multiple, list, reference, question, ml, index)
        }
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
    multiple: bool,
    list: &str,
    reference: &str,
    question: &Question,
    ml: bool,
    index: &HashMap<String, Vec<Step>>,
) -> Result<(), XformError> {
    let tag = if multiple { "select" } else { "select1" };
    let mut element = BytesStart::new(tag);
    element.push_attribute(("ref", reference));
    if let Some(appearance) = &question.appearance {
        element.push_attribute(("appearance", appearance.as_str()));
    }
    w.write_event(Event::Start(element))?;
    write_label(w, question, reference, ml, index)?;
    write_hint(w, question, reference, ml)?;

    // Raw content keeps the apostrophes in the nodeset literal (not `&apos;`).
    let itemset = BytesStart::from_content(
        format!("itemset nodeset=\"instance('{list}')/root/item\""),
        "itemset".len(),
    );
    w.write_event(Event::Start(itemset))?;
    ref_element(w, "value", "name")?;
    if ml {
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
    if ml && q.label.is_multilingual() {
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

/// Resolve `${target}` referenced from `context` to its XForm XPath.
fn resolve_reference(
    index: &HashMap<String, Vec<Step>>,
    context: &str,
    target: &str,
) -> Option<String> {
    let context = index.get(context)?;
    let target = index.get(target)?;
    Some(reference_xpath(context, target))
}

/// Write a control's `<hint>` when present.
fn write_hint(w: &mut W, q: &Question, reference: &str, ml: bool) -> Result<(), XformError> {
    if ml && q.hint.is_multilingual() {
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
