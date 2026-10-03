//! Normalize a raw [`Workbook`] into the [`Survey`] model.
//!
//! This is the semantic core: type parsing, group/repeat nesting, choices,
//! settings and localization. Phase 5 reads `label`/`hint` column families
//! (with `::Lang` qualifiers) and collects the form's languages.

use rustxform_core::{
    Choice, ChoiceList, Container, Kind, Localized, Node, Question, Settings, Survey,
    resolve_builtin,
};
use rustxform_reader::{Sheet, Workbook};

/// An error produced while normalizing a workbook into a survey.
///
/// No variants exist yet; parsing rules add them in later phases.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {}

/// Normalize a workbook into a [`Survey`].
///
/// # Errors
///
/// Currently infallible; validation and strict parsing add failure modes
/// later.
pub fn workbook_to_survey(workbook: &Workbook) -> Result<Survey, ParseError> {
    let mut survey = Survey::default();

    if let Some(sheet) = workbook.sheet("settings") {
        survey.settings = parse_settings(sheet);
    }
    if let Some(sheet) = workbook.sheet("choices") {
        survey.choices = parse_choices(sheet);
    }
    if let Some(sheet) = workbook.sheet("survey") {
        survey.children = parse_nodes(sheet);
    }
    survey.languages = collect_languages(workbook.sheet("survey"), workbook.sheet("choices"));

    Ok(survey)
}

/// Read the single-row `settings` sheet into [`Settings`].
fn parse_settings(sheet: &Sheet) -> Settings {
    let mut settings = Settings::default();
    let (Some(header), Some(values)) = (sheet.rows.first(), sheet.rows.get(1)) else {
        return settings;
    };

    for (col, value) in header.iter().zip(values.iter()) {
        if value.is_empty() {
            continue;
        }
        let value = Some(value.clone());
        match col.as_str() {
            "form_title" => settings.title = value,
            "form_id" => settings.form_id = value,
            "version" => settings.version = value,
            "default_language" => settings.default_language = value,
            _ => {}
        }
    }
    settings
}

/// Indices of a `label`/`hint` column family: the plain column plus any
/// language-qualified ones.
#[derive(Default)]
struct LocColumns {
    plain: Option<usize>,
    langs: Vec<(String, usize)>,
}

/// Locate a localizable column family (`base` is `label` or `hint`).
fn loc_columns(header: &[String], base: &str) -> LocColumns {
    let prefix = format!("{base}::");
    let mut cols = LocColumns::default();
    for (i, head) in header.iter().enumerate() {
        if head == base {
            cols.plain = Some(i);
        } else if let Some(lang) = head.strip_prefix(&prefix) {
            cols.langs.push((lang.trim().to_owned(), i));
        }
    }
    cols
}

/// Build a [`Localized`] value for a row from a column family.
fn localized(cols: &LocColumns, row: &[String]) -> Localized {
    let get = |i: usize| row.get(i).map_or("", String::as_str);
    Localized {
        default: cols
            .plain
            .map(get)
            .filter(|s| !s.is_empty())
            .map(str::to_owned),
        langs: cols
            .langs
            .iter()
            .filter_map(|(lang, i)| {
                let text = get(*i);
                (!text.is_empty()).then(|| (lang.clone(), text.to_owned()))
            })
            .collect(),
    }
}

/// Collect declared languages in order across the survey and choices sheets.
fn collect_languages(survey: Option<&Sheet>, choices: Option<&Sheet>) -> Vec<String> {
    let mut languages: Vec<String> = Vec::new();
    let mut add = |header: &[String], bases: &[&str]| {
        for head in header {
            if let Some((base, lang)) = head.split_once("::") {
                if bases.contains(&base.trim()) {
                    let lang = lang.trim().to_owned();
                    if !languages.contains(&lang) {
                        languages.push(lang);
                    }
                }
            }
        }
    };
    if let Some(header) = survey.and_then(|s| s.rows.first()) {
        add(header, &["label", "hint"]);
    }
    if let Some(header) = choices.and_then(|s| s.rows.first()) {
        add(header, &["label"]);
    }
    languages
}

/// An open group/repeat being accumulated on the parse stack.
enum Frame {
    Group(Container),
    Repeat(Container),
}

/// Read the `survey` sheet rows into a node tree, honoring group/repeat markers.
fn parse_nodes(sheet: &Sheet) -> Vec<Node> {
    let Some((header, data)) = sheet.rows.split_first() else {
        return Vec::new();
    };
    let column = |name: &str| header.iter().position(|h| h == name);
    let type_col = column("type");
    let name_col = column("name");
    let plain_label_col = column("label");
    let appearance_col = column("appearance");
    let calc_col = column("calculation");
    let relevant_col = column("relevant");
    let constraint_col = column("constraint");
    let required_col = column("required");
    let readonly_col = column("read_only").or_else(|| column("readonly"));
    let cmsg_col = column("constraint_message");
    let rmsg_col = column("required_message");
    let params_col = column("parameters");
    let label_cols = loc_columns(header, "label");
    let hint_cols = loc_columns(header, "hint");
    let media_cols = loc_columns(header, "media");

    let mut root: Vec<Node> = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();

    for row in data {
        let cell = |col: Option<usize>| col.and_then(|i| row.get(i)).map_or("", String::as_str);
        let type_token = cell(type_col);
        if type_token.is_empty() {
            continue;
        }
        let container = || Container {
            name: cell(name_col).to_owned(),
            label: optional(cell(plain_label_col)),
            children: Vec::new(),
        };

        match type_token {
            "begin_group" | "begin group" => stack.push(Frame::Group(container())),
            "begin_repeat" | "begin repeat" => stack.push(Frame::Repeat(container())),
            "end_group" | "end group" => {
                if let Some(Frame::Group(group)) = stack.pop() {
                    place(&mut root, &mut stack, Node::Group(group));
                }
            }
            "end_repeat" | "end repeat" => {
                if let Some(Frame::Repeat(repeat)) = stack.pop() {
                    place(&mut root, &mut stack, Node::Repeat(repeat));
                }
            }
            _ => {
                let question = Question {
                    kind: parse_kind(type_token),
                    name: cell(name_col).to_owned(),
                    label: localized(&label_cols, row),
                    hint: localized(&hint_cols, row),
                    appearance: optional(cell(appearance_col)),
                    calculation: optional(cell(calc_col)),
                    relevant: optional(cell(relevant_col)),
                    constraint: optional(cell(constraint_col)),
                    required: optional(cell(required_col)),
                    readonly: optional(cell(readonly_col)),
                    constraint_message: optional(cell(cmsg_col)),
                    required_message: optional(cell(rmsg_col)),
                    parameters: parse_parameters(cell(params_col)),
                    media: media_cols
                        .langs
                        .iter()
                        .filter_map(|(form, i)| {
                            let file = cell(Some(*i));
                            (!file.is_empty()).then(|| (form.clone(), file.to_owned()))
                        })
                        .collect(),
                };
                place(&mut root, &mut stack, Node::Question(question));
            }
        }
    }

    // Flush any unclosed containers, innermost first, so nothing is lost.
    while let Some(frame) = stack.pop() {
        let node = match frame {
            Frame::Group(g) => Node::Group(g),
            Frame::Repeat(r) => Node::Repeat(r),
        };
        place(&mut root, &mut stack, node);
    }
    root
}

/// Append a finished node to the innermost open container, or to the root.
fn place(root: &mut Vec<Node>, stack: &mut [Frame], node: Node) {
    match stack.last_mut() {
        Some(Frame::Group(c) | Frame::Repeat(c)) => c.children.push(node),
        None => root.push(node),
    }
}

/// Resolve a `type` column token into a [`Kind`].
fn parse_kind(token: &str) -> Kind {
    let mut parts = token.split_whitespace();
    let head = parts.next().unwrap_or_default();

    match head {
        "select_one" => Kind::Select {
            multiple: false,
            list: parts.next().unwrap_or_default().to_owned(),
        },
        "select_multiple" => Kind::Select {
            multiple: true,
            list: parts.next().unwrap_or_default().to_owned(),
        },
        other => {
            resolve_builtin(other).map_or_else(|| Kind::Unknown(other.to_owned()), Kind::Builtin)
        }
    }
}

/// Read the `choices` sheet into choice lists, preserving sheet order.
fn parse_choices(sheet: &Sheet) -> Vec<ChoiceList> {
    let Some((header, data)) = sheet.rows.split_first() else {
        return Vec::new();
    };
    let column = |name: &str| header.iter().position(|h| h == name);
    let list_col = column("list_name");
    let name_col = column("name");
    let label_cols = loc_columns(header, "label");

    let mut lists: Vec<ChoiceList> = Vec::new();
    for row in data {
        let cell = |col: Option<usize>| col.and_then(|i| row.get(i)).map_or("", String::as_str);

        let list = cell(list_col);
        if list.is_empty() {
            continue;
        }
        let choice = Choice {
            name: cell(name_col).to_owned(),
            label: localized(&label_cols, row),
        };
        match lists.iter_mut().find(|l| l.name == list) {
            Some(existing) => existing.items.push(choice),
            None => lists.push(ChoiceList {
                name: list.to_owned(),
                items: vec![choice],
            }),
        }
    }
    lists
}

/// Map an empty cell to `None`, otherwise `Some(owned)`.
fn optional(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

/// Parse a `parameters` cell (`key=value key=value`) into pairs.
fn parse_parameters(raw: &str) -> Vec<(String, String)> {
    raw.split_whitespace()
        .filter_map(|pair| {
            pair.split_once('=')
                .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
        })
        .collect()
}
