//! Normalize a raw [`Workbook`] into the [`Survey`] model.
//!
//! This is the semantic core: alias resolution, type parsing, group/repeat
//! nesting, choices and settings. Phase 2 handles the `survey`, `choices` and
//! `settings` sheets for flat forms; groups and repeats arrive later.

use rustxform_core::{Choice, ChoiceList, Kind, Question, Settings, Survey, resolve_builtin};
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
        survey.children = parse_questions(sheet);
    }

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

/// Read the `survey` sheet rows into questions, skipping typeless rows.
fn parse_questions(sheet: &Sheet) -> Vec<Question> {
    let Some((header, data)) = sheet.rows.split_first() else {
        return Vec::new();
    };
    let column = |name: &str| header.iter().position(|h| h == name);
    let type_col = column("type");
    let name_col = column("name");
    let label_col = column("label");
    let calc_col = column("calculation");

    let mut questions = Vec::with_capacity(data.len());
    for row in data {
        let cell = |col: Option<usize>| col.and_then(|i| row.get(i)).map_or("", String::as_str);

        let type_token = cell(type_col);
        if type_token.is_empty() {
            continue;
        }
        questions.push(Question {
            kind: parse_kind(type_token),
            name: cell(name_col).to_owned(),
            label: optional(cell(label_col)),
            calculation: optional(cell(calc_col)),
        });
    }
    questions
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
    let label_col = column("label");

    let mut lists: Vec<ChoiceList> = Vec::new();
    for row in data {
        let cell = |col: Option<usize>| col.and_then(|i| row.get(i)).map_or("", String::as_str);

        let list = cell(list_col);
        if list.is_empty() {
            continue;
        }
        let choice = Choice {
            name: cell(name_col).to_owned(),
            label: optional(cell(label_col)),
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
