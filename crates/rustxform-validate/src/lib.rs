//! Structural validations over a parsed [`Survey`].
//!
//! Checks run before (or instead of) emission and report every problem found,
//! rather than stopping at the first. Phase 7 covers duplicate and empty node
//! names, broken `${…}` references, and selects pointing at an unknown choice
//! list.

use std::collections::HashSet;

use rustxform_core::{Kind, Node, Question, Survey};
use rustxform_expr::reference_names;

/// A single validation problem found in a survey.
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
}

/// Validate a [`Survey`], returning all problems found (empty when valid).
#[must_use]
pub fn validate(survey: &Survey) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let mut names: HashSet<String> = HashSet::new();
    collect_names(&survey.children, &mut names, &mut errors);

    let lists: HashSet<&str> = survey.choices.iter().map(|c| c.name.as_str()).collect();
    check_nodes(&survey.children, &names, &lists, &mut errors);

    errors
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

/// Check selects and expression references against the known names and lists.
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
    if let Kind::Select { list, .. } = &q.kind {
        if !lists.contains(list.as_str()) {
            errors.push(ValidationError::UnknownChoiceList {
                question: q.name.clone(),
                list: list.clone(),
            });
        }
    }

    let expressions = [
        &q.relevant,
        &q.constraint,
        &q.required,
        &q.readonly,
        &q.calculation,
    ];
    for expr in expressions.into_iter().flatten() {
        for reference in reference_names(expr) {
            // `#`-qualified references (e.g. `last-saved#x`) are not plain nodes.
            if !reference.contains('#') && !names.contains(&reference) {
                errors.push(ValidationError::UnknownReference {
                    owner: q.name.clone(),
                    reference,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustxform_parse::workbook_to_survey;
    use rustxform_reader::read_markdown;

    fn survey(md: &str) -> Survey {
        workbook_to_survey(&read_markdown(md).unwrap()).unwrap()
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
}
