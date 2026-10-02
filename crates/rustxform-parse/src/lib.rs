//! Normalize a raw [`Workbook`] into the [`Survey`] model.
//!
//! This is the semantic core: alias resolution, type parsing, group/repeat
//! nesting, choices and settings. Phase 1 handles the `survey` and `settings`
//! sheets for flat forms; later phases add choices, groups and repeats.

use rustxform_core::{Question, Settings, Survey};
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

/// Read the `survey` sheet rows into questions, skipping blank/typeless rows.
fn parse_questions(sheet: &Sheet) -> Vec<Question> {
    let Some((header, data)) = sheet.rows.split_first() else {
        return Vec::new();
    };
    let column = |name: &str| header.iter().position(|h| h == name);
    let (type_col, name_col, label_col) = (column("type"), column("name"), column("label"));

    let mut questions = Vec::with_capacity(data.len());
    for row in data {
        let cell = |col: Option<usize>| col.and_then(|i| row.get(i)).map_or("", String::as_str);

        let kind = cell(type_col);
        if kind.is_empty() {
            continue;
        }
        let label = cell(label_col);
        questions.push(Question {
            kind: kind.to_owned(),
            name: cell(name_col).to_owned(),
            label: (!label.is_empty()).then(|| label.to_owned()),
        });
    }
    questions
}
