//! Emit XForm XML from the [`Survey`] model.
//!
//! Produces the `<model>` (primary instance, choice secondary instances and
//! binds) and the `<body>` controls. Phase 3 walks the node tree: groups
//! nest, and repeats emit a `jr:template` plus one live instance. Multi-
//! language translations arrive later.

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
    let index = build_index(&survey.children);
    write_binds(&mut w, &survey.children, &format!("/{ROOT}"), &index)?;
    write_instance_id_bind(&mut w)?;

    w.write_event(Event::End(BytesEnd::new("model")))?;
    w.write_event(Event::End(BytesEnd::new("h:head")))?;

    w.write_event(Event::Start(BytesStart::new("h:body")))?;
    write_body(&mut w, &survey.children, &format!("/{ROOT}"))?;
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

// --- Primary instance -------------------------------------------------------

/// `<instance><data id="..">…tree…<meta>…</meta></data></instance>`.
fn write_primary_instance(w: &mut W, survey: &Survey, form_id: &str) -> Result<(), XformError> {
    w.write_event(Event::Start(BytesStart::new("instance")))?;
    let mut root = BytesStart::new(ROOT);
    root.push_attribute(("id", form_id));
    w.write_event(Event::Start(root))?;

    for node in &survey.children {
        write_instance_node(w, node)?;
    }

    w.write_event(Event::Start(BytesStart::new("meta")))?;
    w.write_event(Event::Empty(BytesStart::new("instanceID")))?;
    w.write_event(Event::End(BytesEnd::new("meta")))?;
    w.write_event(Event::End(BytesEnd::new(ROOT)))?;
    w.write_event(Event::End(BytesEnd::new("instance")))?;
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
fn write_choice_instances(w: &mut W, survey: &Survey) -> Result<(), XformError> {
    let mut seen: Vec<&str> = Vec::new();
    collect_lists(&survey.children, &mut seen);

    for list in seen {
        let Some(choice_list) = survey.choice_list(list) else {
            continue;
        };
        let mut instance = BytesStart::new("instance");
        instance.push_attribute(("id", list));
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
    if let Some(expr) = &q.relevant {
        bind.push_attribute(("relevant", rewrite_references(expr, resolve).as_str()));
    }
    if let Some(expr) = &q.constraint {
        bind.push_attribute(("constraint", rewrite_references(expr, resolve).as_str()));
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

// --- Body -------------------------------------------------------------------

/// Emit body controls for `nodes` under `parent`.
fn write_body(w: &mut W, nodes: &[Node], parent: &str) -> Result<(), XformError> {
    for node in nodes {
        match node {
            Node::Question(q) => write_control(w, q, &format!("{parent}/{}", q.name))?,
            Node::Group(g) => write_group(w, g, &format!("{parent}/{}", g.name), false)?,
            Node::Repeat(r) => write_group(w, r, &format!("{parent}/{}", r.name), true)?,
        }
    }
    Ok(())
}

/// Write `<group ref="..">`, with an inner `<repeat>` when `repeat` is set.
fn write_group(w: &mut W, c: &Container, path: &str, repeat: bool) -> Result<(), XformError> {
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
        write_body(w, &c.children, path)?;
        w.write_event(Event::End(BytesEnd::new("repeat")))?;
    } else {
        write_body(w, &c.children, path)?;
    }

    w.write_event(Event::End(BytesEnd::new("group")))?;
    Ok(())
}

/// Write the body control for a question (nothing for data-only types).
fn write_control(w: &mut W, question: &Question, reference: &str) -> Result<(), XformError> {
    match &question.kind {
        Kind::Builtin(b) => match b.control {
            None => Ok(()),
            Some(Control::Input) => simple_control(w, "input", reference, question, &[]),
            Some(Control::Trigger) => simple_control(w, "trigger", reference, question, &[]),
            Some(Control::Upload { mediatype }) => simple_control(
                w,
                "upload",
                reference,
                question,
                &[("mediatype", mediatype)],
            ),
        },
        Kind::Select { multiple, list } => write_select(w, *multiple, list, reference, question),
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
