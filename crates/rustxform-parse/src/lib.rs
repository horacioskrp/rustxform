//! Normalize a raw [`Workbook`] into the [`Survey`] model.
//!
//! This is the semantic core: type parsing, group/repeat nesting, choices,
//! settings and localization. Phase 5 reads `label`/`hint` column families
//! (with `::Lang` qualifiers) and collects the form's languages.

use rustxform_core::{
    Choice, ChoiceList, Container, Entity, Kind, Localized, Media, Node, Question, SelectType,
    Settings, Survey, resolve_builtin,
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
        let (children, or_other_lists) = parse_nodes(sheet);
        survey.children = children;
        survey.audit = extract_audit(&mut survey.children);
        add_or_other_choices(&mut survey.choices, &or_other_lists);
    }
    if let Some(sheet) = workbook.sheet("osm") {
        survey.osm_tags = parse_choices(sheet);
    }
    if let Some(sheet) = workbook.sheet("entities") {
        survey.entity = parse_entity(sheet);
    }
    survey.external_instances = extract_external(&mut survey.children);
    survey.languages = collect_languages(workbook.sheet("survey"), workbook.sheet("choices"));

    Ok(survey)
}

/// Remove top-level `xml-external` questions, returning their names (each
/// becomes an external secondary instance).
fn extract_external(children: &mut Vec<Node>) -> Vec<String> {
    let mut names = Vec::new();
    children.retain(|node| match node {
        Node::Question(q) if matches!(&q.kind, Kind::Unknown(t) if t == "xml-external") => {
            names.push(q.name.clone());
            false
        }
        _ => true,
    });
    names
}

/// Remove a top-level `audit` metadata question, returning whether one was
/// present (it moves under `meta` at emission time).
fn extract_audit(children: &mut Vec<Node>) -> bool {
    let position = children.iter().position(|node| {
        matches!(node, Node::Question(q) if matches!(&q.kind, Kind::Unknown(t) if t == "audit"))
    });
    match position {
        Some(i) => {
            children.remove(i);
            true
        }
        None => false,
    }
}

/// Append an `other`/`Other` option to each list used by an `… or_other`
/// select, unless one is already present.
fn add_or_other_choices(choices: &mut [ChoiceList], lists: &[String]) {
    for list in lists {
        if let Some(choice_list) = choices.iter_mut().find(|c| &c.name == list) {
            if !choice_list.items.iter().any(|item| item.name == "other") {
                choice_list.items.push(Choice {
                    name: "other".to_owned(),
                    label: Localized {
                        default: Some("Other".to_owned()),
                        langs: Vec::new(),
                    },
                    extra: Vec::new(),
                });
            }
        }
    }
}

/// Read the single-row `entities` sheet into an [`Entity`].
fn parse_entity(sheet: &Sheet) -> Option<Entity> {
    let (header, values) = (sheet.rows.first()?, sheet.rows.get(1)?);
    let column = |name: &str| header.iter().position(|h| h == name);
    let cell = |col: Option<usize>| col.and_then(|i| values.get(i)).map_or("", String::as_str);

    let dataset = cell(column("dataset"));
    if dataset.is_empty() {
        return None;
    }
    Some(Entity {
        dataset: dataset.to_owned(),
        label: optional(cell(column("label"))),
        entity_id: optional(cell(column("entity_id"))),
        create_if: optional(cell(column("create_if"))),
        update_if: optional(cell(column("update_if"))),
    })
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
            "instance_name" => settings.instance_name = value,
            "style" => settings.style = value,
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

/// A parsed `media::form[::lang]` column.
struct MediaColumn {
    form: String,
    lang: Option<String>,
    index: usize,
}

/// Locate the `media::…` columns and parse their form and optional language.
fn media_columns(header: &[String]) -> Vec<MediaColumn> {
    header
        .iter()
        .enumerate()
        .filter_map(|(index, head)| {
            let rest = head.strip_prefix("media::")?;
            let (form, lang) = match rest.split_once("::") {
                Some((form, lang)) => (form.trim().to_owned(), Some(lang.trim().to_owned())),
                None => (rest.trim().to_owned(), None),
            };
            Some(MediaColumn { form, lang, index })
        })
        .collect()
}

/// Build a row's media list, grouping files by form (single or per-language).
fn row_media(cols: &[MediaColumn], row: &[String]) -> Vec<Media> {
    let mut media: Vec<Media> = Vec::new();
    for col in cols {
        let value = row.get(col.index).map_or("", String::as_str);
        if value.is_empty() {
            continue;
        }
        let entry = match media.iter().position(|m| m.form == col.form) {
            Some(i) => &mut media[i],
            None => {
                media.push(Media {
                    form: col.form.clone(),
                    files: Localized::default(),
                });
                media.last_mut().expect("just pushed")
            }
        };
        match &col.lang {
            None => entry.files.default = Some(value.to_owned()),
            Some(lang) => entry.files.langs.push((lang.clone(), value.to_owned())),
        }
    }
    media
}

/// Collect declared languages in order across the survey and choices sheets.
fn collect_languages(survey: Option<&Sheet>, choices: Option<&Sheet>) -> Vec<String> {
    let mut languages: Vec<String> = Vec::new();
    let mut add = |header: &[String], bases: &[&str]| {
        for head in header {
            // `media::form::lang` carries the language after a second `::`.
            let parsed = match head.strip_prefix("media::") {
                Some(rest) => rest.split_once("::").map(|(_, lang)| ("media", lang)),
                None => head.split_once("::"),
            };
            if let Some((base, lang)) = parsed {
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
        add(
            header,
            &[
                "label",
                "hint",
                "guidance_hint",
                "constraint_message",
                "required_message",
                "media",
            ],
        );
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
fn parse_nodes(sheet: &Sheet) -> (Vec<Node>, Vec<String>) {
    let Some((header, data)) = sheet.rows.split_first() else {
        return (Vec::new(), Vec::new());
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
    let cmsg_cols = loc_columns(header, "constraint_message");
    let rmsg_cols = loc_columns(header, "required_message");
    let params_col = column("parameters");
    let default_col = column("default");
    let filter_col = column("choice_filter");
    let saveto_col = column("save_to");
    let trigger_col = column("trigger");
    let count_col = column("repeat_count");
    let label_cols = loc_columns(header, "label");
    let hint_cols = loc_columns(header, "hint");
    let guidance_cols = loc_columns(header, "guidance_hint");
    let media_cols = media_columns(header);

    let mut root: Vec<Node> = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    // Choice lists that need a synthetic `other` option (from `… or_other`).
    let mut or_other_lists: Vec<String> = Vec::new();

    for row in data {
        let cell = |col: Option<usize>| col.and_then(|i| row.get(i)).map_or("", String::as_str);
        let type_token = cell(type_col);
        if type_token.is_empty() {
            continue;
        }
        let container = || Container {
            name: cell(name_col).to_owned(),
            label: optional(cell(plain_label_col)),
            appearance: optional(cell(appearance_col)),
            count: optional(cell(count_col)),
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
                let mut question = Question {
                    kind: parse_kind(type_token),
                    name: cell(name_col).to_owned(),
                    label: localized(&label_cols, row),
                    hint: localized(&hint_cols, row),
                    guidance_hint: localized(&guidance_cols, row),
                    appearance: optional(cell(appearance_col)),
                    calculation: optional(cell(calc_col)),
                    relevant: optional(cell(relevant_col)),
                    constraint: optional(cell(constraint_col)),
                    required: optional(cell(required_col)),
                    readonly: optional(cell(readonly_col)),
                    constraint_message: localized(&cmsg_cols, row),
                    required_message: localized(&rmsg_cols, row),
                    parameters: parse_parameters(cell(params_col)),
                    media: row_media(&media_cols, row),
                    default: optional(cell(default_col)),
                    choice_filter: optional(cell(filter_col)),
                    save_to: optional(cell(saveto_col)),
                    trigger: optional(cell(trigger_col)),
                };
                if type_token.trim() == "phone number" {
                    if question.constraint.is_none() {
                        question.constraint = Some("regex(., '^\\d*$')".to_owned());
                    }
                    if question.hint.is_empty() {
                        question.hint.default = Some("Enter numbers only.".to_owned());
                    }
                }
                let qname = question.name.clone();
                place(&mut root, &mut stack, Node::Question(question));

                // `select_* <list> or_other`: add an `other` option to the list
                // and a sibling text field relevant only when `other` is chosen.
                if let ["select_one" | "select_multiple", list, "or_other"] =
                    type_token.split_whitespace().collect::<Vec<_>>().as_slice()
                {
                    or_other_lists.push((*list).to_owned());
                    let other = Question {
                        kind: resolve_builtin("text")
                            .map_or_else(|| Kind::Unknown("text".to_owned()), Kind::Builtin),
                        name: format!("{qname}_other"),
                        label: Localized {
                            default: Some("Specify other.".to_owned()),
                            langs: Vec::new(),
                        },
                        hint: Localized::default(),
                        guidance_hint: Localized::default(),
                        appearance: None,
                        calculation: None,
                        relevant: Some(format!("selected(../{qname}, 'other')")),
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
                        trigger: None,
                    };
                    place(&mut root, &mut stack, Node::Question(other));
                }
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
    (root, or_other_lists)
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

    if token.trim() == "phone number" {
        return resolve_builtin("text")
            .map_or_else(|| Kind::Unknown(token.to_owned()), Kind::Builtin);
    }
    let inline = |select, list: &str| Kind::Select {
        select,
        list: list.to_owned(),
        file: None,
    };
    match head {
        "select_one" => inline(SelectType::One, parts.next().unwrap_or_default()),
        "select_multiple" => inline(SelectType::Multiple, parts.next().unwrap_or_default()),
        "rank" => inline(SelectType::Rank, parts.next().unwrap_or_default()),
        "osm" => Kind::Osm {
            tagset: parts.next().unwrap_or_default().to_owned(),
        },
        "select_one_from_file" => {
            select_from_file(SelectType::One, parts.next().unwrap_or_default())
        }
        "select_multiple_from_file" => {
            select_from_file(SelectType::Multiple, parts.next().unwrap_or_default())
        }
        other => {
            resolve_builtin(other).map_or_else(|| Kind::Unknown(other.to_owned()), Kind::Builtin)
        }
    }
}

/// Build an external-file select whose instance id is the file's stem.
fn select_from_file(select: SelectType, file: &str) -> Kind {
    let stem = file.rsplit_once('.').map_or(file, |(name, _)| name);
    Kind::Select {
        select,
        list: stem.to_owned(),
        file: Some(file.to_owned()),
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

    // Extra columns (anything but list_name/name/label*) become item children
    // for cascading filters.
    let mut excluded: Vec<usize> = [list_col, name_col, label_cols.plain]
        .into_iter()
        .flatten()
        .collect();
    excluded.extend(label_cols.langs.iter().map(|(_, i)| *i));
    let extra_cols: Vec<(usize, &str)> = header
        .iter()
        .enumerate()
        .filter(|(i, h)| !excluded.contains(i) && !h.is_empty())
        .map(|(i, h)| (i, h.as_str()))
        .collect();

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
            extra: extra_cols
                .iter()
                .filter_map(|(i, key)| {
                    let value = cell(Some(*i));
                    (!value.is_empty()).then(|| ((*key).to_owned(), value.to_owned()))
                })
                .collect(),
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
