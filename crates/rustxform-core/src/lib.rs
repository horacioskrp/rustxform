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
/// In Phase 0 children are a flat list of [`Question`]; groups and repeats
/// are introduced in Phase 3.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Survey {
    /// Form-level settings.
    pub settings: Settings,
    /// Ordered survey children.
    pub children: Vec<Question>,
}
