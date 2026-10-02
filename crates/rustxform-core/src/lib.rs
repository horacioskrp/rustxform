//! Core data model shared across the rustxform pipeline.
//!
//! These types are the normalized, backend-agnostic representation of an
//! XLSForm after parsing and before XForm emission. They are intentionally
//! minimal in Phase 0 and grow as later phases add question kinds, groups,
//! repeats, translations and parameters.

/// Form-level settings, sourced from the XLSForm `settings` sheet.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    /// Form identifier (`form_id`); defaults to `data` when absent.
    pub form_id: Option<String>,
    /// Human-readable form title.
    pub title: Option<String>,
    /// Form version string.
    pub version: Option<String>,
    /// Default language label, e.g. `French (fr)`.
    pub default_language: Option<String>,
}

/// A single survey question from the `survey` sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    /// XLSForm `type` token (e.g. `text`, `integer`, `select_one yes_no`).
    pub kind: String,
    /// Node name; becomes the instance element name.
    pub name: String,
    /// Default-language label, if any.
    pub label: Option<String>,
}

/// A parsed XLSForm survey: settings plus an ordered list of children.
///
/// Children are a flat list of [`Question`] for now; groups and repeats are
/// introduced in Phase 3.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Survey {
    /// Form-level settings.
    pub settings: Settings,
    /// Ordered survey children.
    pub children: Vec<Question>,
}

/// XForm mapping for a question type: optional body control and bind type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeSpec {
    /// Body control tag (e.g. `input`, `select`, `upload`), or `None` for
    /// data-only types such as `calculate`.
    pub control_tag: Option<&'static str>,
    /// XForm bind data type (e.g. `string`, `int`).
    pub bind_type: &'static str,
    /// Whether the bind is read-only.
    pub readonly: bool,
}

/// Resolve an XLSForm question type token to its XForm mapping.
///
/// Returns `None` for types not yet supported. The table grows as later
/// phases add question kinds.
#[must_use]
pub fn resolve_type(kind: &str) -> Option<TypeSpec> {
    let spec = |control_tag, bind_type, readonly| TypeSpec {
        control_tag,
        bind_type,
        readonly,
    };
    match kind {
        "text" => Some(spec(Some("input"), "string", false)),
        "integer" => Some(spec(Some("input"), "int", false)),
        "decimal" => Some(spec(Some("input"), "decimal", false)),
        "note" => Some(spec(Some("input"), "string", true)),
        _ => None,
    }
}
