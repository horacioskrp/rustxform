//! Core data model shared across the rustxform pipeline.
//!
//! These types are the normalized, backend-agnostic representation of an
//! XLSForm after parsing and before XForm emission. They grow as later phases
//! add groups, repeats, translations and parameters.

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

/// The resolved semantics of a question's `type` column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// A built-in type with a fixed XForm mapping.
    Builtin(Builtin),
    /// A `select_one` / `select_multiple` referencing a choice list.
    Select {
        /// `true` for `select_multiple`, `false` for `select_one`.
        multiple: bool,
        /// Name of the referenced choice list.
        list: String,
    },
    /// A type token not yet supported; emitted as a plain string bind.
    Unknown(String),
}

/// The XForm body control a built-in question renders as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// `<input>`.
    Input,
    /// `<trigger>` (e.g. `acknowledge`).
    Trigger,
    /// `<upload>` with the given `mediatype`.
    Upload {
        /// The `mediatype` attribute, e.g. `image/*`.
        mediatype: &'static str,
    },
}

/// A metadata preload binding (`jr:preload` + `jr:preloadParams`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preload {
    /// The `jr:preload` value, e.g. `timestamp` or `property`.
    pub preload: &'static str,
    /// The `jr:preloadParams` value, e.g. `start` or `deviceid`.
    pub params: &'static str,
}

/// The XForm mapping for a built-in question type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Builtin {
    /// Body control, or `None` for data-only types (`calculate`, preloads).
    pub control: Option<Control>,
    /// XForm bind data type (e.g. `string`, `int`, `binary`).
    pub bind_type: &'static str,
    /// Whether the bind is read-only.
    pub readonly: bool,
    /// Metadata preload, if this type is a preload.
    pub preload: Option<Preload>,
}

/// A single survey question from the `survey` sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    /// Resolved type semantics.
    pub kind: Kind,
    /// Node name; becomes the instance element name.
    pub name: String,
    /// Default-language label, if any.
    pub label: Option<String>,
    /// Expression from the `calculation` column, if any.
    pub calculation: Option<String>,
}

/// One option within a choice list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    /// Stored value (`name` column).
    pub name: String,
    /// Display label, if any.
    pub label: Option<String>,
}

/// A named list of choices from the `choices` sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceList {
    /// List name referenced by `select_*` questions.
    pub name: String,
    /// Options in sheet order.
    pub items: Vec<Choice>,
}

/// A parsed XLSForm survey: settings, children and choice lists.
///
/// Children are a flat list of [`Question`] for now; groups and repeats are
/// introduced in Phase 3.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Survey {
    /// Form-level settings.
    pub settings: Settings,
    /// Ordered survey children.
    pub children: Vec<Question>,
    /// Choice lists from the `choices` sheet.
    pub choices: Vec<ChoiceList>,
}

impl Survey {
    /// Return the choice list with the given name, if present.
    #[must_use]
    pub fn choice_list(&self, name: &str) -> Option<&ChoiceList> {
        self.choices.iter().find(|c| c.name == name)
    }
}

/// Resolve a single XLSForm type token to its built-in XForm mapping.
///
/// Returns `None` for `select_*` (handled separately) and unknown tokens.
/// The table grows as later phases add question kinds.
#[must_use]
pub fn resolve_builtin(kind: &str) -> Option<Builtin> {
    let b = |control, bind_type, readonly, preload| Builtin {
        control,
        bind_type,
        readonly,
        preload,
    };
    let input = Some(Control::Input);
    let preload = |preload, params| Some(Preload { preload, params });

    Some(match kind {
        "text" => b(input, "string", false, None),
        "integer" => b(input, "int", false, None),
        "decimal" => b(input, "decimal", false, None),
        "range" => b(input, "int", false, None),
        "date" => b(input, "date", false, None),
        "time" => b(input, "time", false, None),
        "dateTime" => b(input, "dateTime", false, None),
        "geopoint" => b(input, "geopoint", false, None),
        "geotrace" => b(input, "geotrace", false, None),
        "geoshape" => b(input, "geoshape", false, None),
        "barcode" => b(input, "barcode", false, None),
        "note" => b(input, "string", true, None),
        "acknowledge" => b(Some(Control::Trigger), "string", false, None),
        "image" => b(
            Some(Control::Upload {
                mediatype: "image/*",
            }),
            "binary",
            false,
            None,
        ),
        "audio" => b(
            Some(Control::Upload {
                mediatype: "audio/*",
            }),
            "binary",
            false,
            None,
        ),
        "video" => b(
            Some(Control::Upload {
                mediatype: "video/*",
            }),
            "binary",
            false,
            None,
        ),
        "calculate" => b(None, "string", false, None),
        "start" => b(None, "dateTime", false, preload("timestamp", "start")),
        "end" => b(None, "dateTime", false, preload("timestamp", "end")),
        "today" => b(None, "date", false, preload("date", "today")),
        "deviceid" => b(None, "string", false, preload("property", "deviceid")),
        _ => return None,
    })
}
