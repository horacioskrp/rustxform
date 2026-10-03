//! Structural validations and non-fatal warnings over a parsed form.
//!
//! [`validate`] returns blocking [`ValidationError`]s (duplicate/empty names,
//! broken `${…}` references, unknown choice lists, malformed `range`
//! parameters and geo defaults). [`warnings`] returns non-fatal [`Warning`]s
//! (missing settings, unrecognized survey columns), which need the raw
//! workbook headers.

use std::collections::HashSet;

use rustxform_core::{Control, Kind, Node, Question, Survey};
use rustxform_expr::reference_names;
use rustxform_reader::Workbook;

/// A blocking validation problem.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    /// A node name is used more than once.
    #[error("duplicate node name `{0}`")]
    DuplicateName(String),
    /// A node has an empty name.
    #[error("a `{0}` node has an empty name")]
    EmptyName(String),
    /// An expression references a name that is not a node in the form.
    #[error("`{owner}` references unknown name `{reference}`")]
    UnknownReference {
        /// Name of the question whose expression holds the reference.
        owner: String,
        /// The unresolved `${…}` name.
        reference: String,
    },
    /// A `select_*` question references a choice list that does not exist.
    #[error("`{question}` references unknown choice list `{list}`")]
    UnknownChoiceList {
        /// Name of the offending select question.
        question: String,
        /// The missing choice list name.
        list: String,
    },
    /// A `range` question has invalid `parameters`.
    #[error("`{question}` has invalid range parameters: {reason}")]
    InvalidRange {
        /// Name of the range question.
        question: String,
        /// Why the parameters are invalid.
        reason: String,
    },
    /// A geo question has a malformed `default` value.
    #[error("`{question}` has a malformed geo default `{value}`")]
    InvalidGeoDefault {
        /// Name of the geo question.
        question: String,
        /// The offending default value.
        value: String,
    },
}

/// A non-fatal warning.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Warning {
    /// A recommended setting is absent (a default is used).
    #[error("the `{0}` setting is missing; a default is used")]
    MissingSetting(String),
    /// A survey column header is not recognized (possibly a typo).
    #[error("unrecognized survey column `{0}`")]
    UnknownColumn(String),
}

/// Validate a [`Survey`], returning all blocking problems (empty when valid).
#[must_use]
pub fn validate(survey: &Survey) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let mut names: HashSet<String> = HashSet::new();
    collect_names(&survey.children, &mut names, &mut errors);

    let lists: HashSet<&str> = survey.choices.iter().map(|c| c.name.as_str()).collect();
    check_nodes(&survey.children, &names, &lists, &mut errors);

    errors
}

/// Collect non-fatal warnings; needs the raw workbook for header checks.
#[must_use]
pub fn warnings(workbook: &Workbook, survey: &Survey) -> Vec<Warning> {
    let mut warnings = Vec::new();

    if survey.settings.title.is_none() {
        warnings.push(Warning::MissingSetting("form_title".to_owned()));
    }
    if survey.settings.form_id.is_none() {
        warnings.push(Warning::MissingSetting("form_id".to_owned()));
    }

    if let Some(header) = workbook.sheet("survey").and_then(|s| s.rows.first()) {
        for column in header {
            if !column.is_empty() && !is_known_survey_column(column) {
                warnings.push(Warning::UnknownColumn(column.clone()));
            }
        }
    }

    warnings
}

/// Gather node names, reporting empty and duplicate names as it goes.
fn collect_names(nodes: &[Node], names: &mut HashSet<String>, errors: &mut Vec<ValidationError>) {
    for node in nodes {
        let (name, kind, children) = match node {
            Node::Question(q) => (&q.name, "question", None),
            Node::Group(c) => (&c.name, "group", Some(&c.children)),
            Node::Repeat(c) => (&c.name, "repeat", Some(&c.children)),
        };

        if name.is_empty() {
            errors.push(ValidationError::EmptyName(kind.to_owned()));
        } else if !names.insert(name.clone()) {
            errors.push(ValidationError::DuplicateName(name.clone()));
        }

        if let Some(children) = children {
            collect_names(children, names, errors);
        }
    }
}

/// Check selects, references, ranges and geo defaults against known names.
fn check_nodes(
    nodes: &[Node],
    names: &HashSet<String>,
    lists: &HashSet<&str>,
    errors: &mut Vec<ValidationError>,
) {
    for node in nodes {
        match node {
            Node::Question(q) => check_question(q, names, lists, errors),
            Node::Group(c) | Node::Repeat(c) => check_nodes(&c.children, names, lists, errors),
        }
    }
}

fn check_question(
    q: &Question,
    names: &HashSet<String>,
    lists: &HashSet<&str>,
    errors: &mut Vec<ValidationError>,
) {
    match &q.kind {
        Kind::Select {
            list, file: None, ..
        } if !lists.contains(list.as_str()) => {
            errors.push(ValidationError::UnknownChoiceList {
                question: q.name.clone(),
                list: list.clone(),
            });
        }
        Kind::Builtin(b) if b.control == Some(Control::Range) => {
            if let Some(reason) = range_problem(q) {
                errors.push(ValidationError::InvalidRange {
                    question: q.name.clone(),
                    reason,
                });
            }
        }
        Kind::Builtin(b) if is_geo(b.bind_type) => {
            if let Some(value) = &q.default {
                if !is_valid_geo(value) {
                    errors.push(ValidationError::InvalidGeoDefault {
                        question: q.name.clone(),
                        value: value.clone(),
                    });
                }
            }
        }
        _ => {}
    }

    for expr in [
        &q.relevant,
        &q.constraint,
        &q.required,
        &q.readonly,
        &q.calculation,
        &q.choice_filter,
    ]
    .into_iter()
    .flatten()
    {
        for reference in reference_names(expr) {
            if !reference.contains('#') && !names.contains(&reference) {
                errors.push(ValidationError::UnknownReference {
                    owner: q.name.clone(),
                    reference,
                });
            }
        }
    }
}

/// Return a reason when a range question's parameters are invalid.
fn range_problem(q: &Question) -> Option<String> {
    let number = |key| q.parameter(key).and_then(|v| v.parse::<f64>().ok());
    let (Some(start), Some(end)) = (number("start"), number("end")) else {
        return Some("missing or non-numeric `start`/`end`".to_owned());
    };
    if start >= end {
        return Some(format!("`start` ({start}) must be less than `end` ({end})"));
    }
    if let Some(step) = q.parameter("step") {
        match step.parse::<f64>() {
            Ok(step) if step > 0.0 => {}
            _ => return Some("`step` must be a positive number".to_owned()),
        }
    }
    None
}

/// Whether a bind type is one of the geo shapes.
fn is_geo(bind_type: &str) -> bool {
    matches!(bind_type, "geopoint" | "geotrace" | "geoshape")
}

/// A geo default is `;`-separated points, each `lat lon altitude accuracy`.
fn is_valid_geo(value: &str) -> bool {
    value
        .split(';')
        .filter(|p| !p.trim().is_empty())
        .all(|point| {
            let coords: Vec<&str> = point.split_whitespace().collect();
            coords.len() == 4 && coords.iter().all(|c| c.parse::<f64>().is_ok())
        })
}

/// Whether a survey column header (sans `::` qualifier) is recognized.
fn is_known_survey_column(column: &str) -> bool {
    let base = column.split("::").next().unwrap_or(column).trim();
    matches!(
        base,
        "type"
            | "name"
            | "label"
            | "hint"
            | "media"
            | "image"
            | "audio"
            | "video"
            | "appearance"
            | "relevant"
            | "constraint"
            | "constraint_message"
            | "required"
            | "required_message"
            | "read_only"
            | "readonly"
            | "calculation"
            | "parameters"
            | "default"
            | "choice_filter"
            | "repeat_count"
            | "trigger"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustxform_parse::workbook_to_survey;
    use rustxform_reader::read_markdown;

    fn parse(md: &str) -> (Workbook, Survey) {
        let wb = read_markdown(md).unwrap();
        let survey = workbook_to_survey(&wb).unwrap();
        (wb, survey)
    }

    fn survey(md: &str) -> Survey {
        parse(md).1
    }

    #[test]
    fn valid_form_reports_nothing() {
        let md = "\
| survey |         |      |            |
|        | type    | name | label      |
|        | integer | age  | Age        |
|        | text    | who  | ${age} > 0 |
";
        assert!(validate(&survey(md)).is_empty());
    }

    #[test]
    fn flags_duplicate_names() {
        let md = "\
| survey |         |     |
|        | type    | name |
|        | text    | q    |
|        | integer | q    |
";
        assert!(validate(&survey(md)).contains(&ValidationError::DuplicateName("q".to_owned())));
    }

    #[test]
    fn flags_unknown_reference() {
        let md = "\
| survey |      |      |             |
|        | type | name | relevant    |
|        | text | q    | ${ghost} = 1 |
";
        assert!(
            validate(&survey(md)).contains(&ValidationError::UnknownReference {
                owner: "q".to_owned(),
                reference: "ghost".to_owned(),
            })
        );
    }

    #[test]
    fn flags_unknown_choice_list() {
        let md = "\
| survey |                  |      |
|        | type             | name |
|        | select_one missing | pick |
";
        assert!(
            validate(&survey(md)).contains(&ValidationError::UnknownChoiceList {
                question: "pick".to_owned(),
                list: "missing".to_owned(),
            })
        );
    }

    #[test]
    fn flags_invalid_range() {
        let md = "\
| survey |       |      |            |
|        | type  | name | parameters |
|        | range | r    | start=5 end=1 |
";
        let errors = validate(&survey(md));
        assert!(errors.iter().any(
            |e| matches!(e, ValidationError::InvalidRange { question, .. } if question == "r")
        ));
    }

    #[test]
    fn flags_malformed_geo_default() {
        let md = "\
| survey |          |      |         |
|        | type     | name | default |
|        | geopoint | loc  | 1 2     |
";
        assert!(
            validate(&survey(md)).contains(&ValidationError::InvalidGeoDefault {
                question: "loc".to_owned(),
                value: "1 2".to_owned(),
            })
        );
    }

    #[test]
    fn warns_on_missing_settings_and_unknown_columns() {
        let md = "\
| survey |      |      |          |
|        | type | name | relevnt  |
|        | text | q    |          |
";
        let (wb, survey) = parse(md);
        let warnings = warnings(&wb, &survey);
        assert!(warnings.contains(&Warning::UnknownColumn("relevnt".to_owned())));
        assert!(warnings.contains(&Warning::MissingSetting("form_id".to_owned())));
    }
}
