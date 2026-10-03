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
    /// `instance_name` expression (populates `meta/instanceName`), if any.
    pub instance_name: Option<String>,
    /// `style` setting (becomes the body `class`), if any.
    pub style: Option<String>,
}

/// Localizable text from a `label`/`hint` column family.
///
/// A plain `label` column fills [`default`](Self::default); language-qualified
/// `label::Lang` columns fill [`langs`](Self::langs). The two are mutually
/// exclusive in practice.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Localized {
    /// Text from an unqualified column, if any.
    pub default: Option<String>,
    /// Per-language texts `(language, text)`, in language order.
    pub langs: Vec<(String, String)>,
}

impl Localized {
    /// `true` when no text is present at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.default.is_none() && self.langs.is_empty()
    }

    /// `true` when language-qualified texts are present.
    #[must_use]
    pub fn is_multilingual(&self) -> bool {
        !self.langs.is_empty()
    }

    /// The unqualified single-language text, if any.
    #[must_use]
    pub fn single(&self) -> Option<&str> {
        self.default.as_deref()
    }

    /// The text for a given language, if any.
    #[must_use]
    pub fn for_lang(&self, lang: &str) -> Option<&str> {
        self.langs
            .iter()
            .find(|(l, _)| l == lang)
            .map(|(_, t)| t.as_str())
    }
}

/// The resolved semantics of a question's `type` column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// A built-in type with a fixed XForm mapping.
    Builtin(Builtin),
    /// A `select_one` / `select_multiple` / `rank` over a choice list or file.
    Select {
        /// Which selection control this is.
        select: SelectType,
        /// The instance id: a choice-list name, or an external file's stem.
        list: String,
        /// External source filename for `select_*_from_file`; `None` inline.
        file: Option<String>,
    },
    /// An `osm <tagset>` capture: an `osm/*` upload with tag children.
    Osm {
        /// Name of the tag list (from the `osm` sheet).
        tagset: String,
    },
    /// A type token not yet supported; emitted as a plain string bind.
    Unknown(String),
}

/// Which selection control a [`Kind::Select`] renders as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectType {
    /// `select_one` → `<select1>`.
    One,
    /// `select_multiple` → `<select>`.
    Multiple,
    /// `rank` → `<odk:rank>`.
    Rank,
}

/// The XForm body control a built-in question renders as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// `<input>`.
    Input,
    /// `<range>` (uses `start`/`end`/`step` from `parameters`).
    Range,
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

/// A label media attachment: a form (`image`/`audio`/`video`) and its file(s),
/// which may be single- or multi-language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Media {
    /// The media form: `image`, `audio` or `video`.
    pub form: String,
    /// Filenames, single (unqualified) or per-language.
    pub files: Localized,
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
    /// A model-level action this type emits (e.g. `odk:recordaudio`).
    pub action: Option<Action>,
}

/// A model-level action element bound to a question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action {
    /// Element name, e.g. `odk:recordaudio` or `odk:setgeopoint`.
    pub element: &'static str,
    /// The `event` attribute, e.g. `odk-instance-load`.
    pub event: &'static str,
}

/// A single survey question from the `survey` sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    /// Resolved type semantics.
    pub kind: Kind,
    /// Node name; becomes the instance element name.
    pub name: String,
    /// Label text (single- or multi-language).
    pub label: Localized,
    /// Hint text (single- or multi-language).
    pub hint: Localized,
    /// `appearance` column value, if any.
    pub appearance: Option<String>,
    /// Expression from the `calculation` column, if any.
    pub calculation: Option<String>,
    /// Expression from the `relevant` column, if any.
    pub relevant: Option<String>,
    /// Expression from the `constraint` column, if any.
    pub constraint: Option<String>,
    /// Raw `required` column value (e.g. `yes` or an expression), if any.
    pub required: Option<String>,
    /// Raw `read_only` column value, if any.
    pub readonly: Option<String>,
    /// `constraint_message` column (single- or multi-language).
    pub constraint_message: Localized,
    /// `required_message` column (single- or multi-language).
    pub required_message: Localized,
    /// Parsed `parameters` column, as `(key, value)` pairs.
    pub parameters: Vec<(String, String)>,
    /// Label media (images/audio/video), possibly per-language.
    pub media: Vec<Media>,
    /// Default value (`default` column), placed in the primary instance.
    pub default: Option<String>,
    /// `choice_filter` expression for cascading selects, if any.
    pub choice_filter: Option<String>,
    /// `save_to` column: the entity property this answer populates, if any.
    pub save_to: Option<String>,
    /// `trigger` column: names a node whose change drives this question's
    /// `calculation` via an `odk:setvalue` action instead of a calculate bind.
    pub trigger: Option<String>,
}

impl Question {
    /// The value of a parameter from the `parameters` column, if present.
    #[must_use]
    pub fn parameter(&self, key: &str) -> Option<&str> {
        self.parameters
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// One option within a choice list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    /// Stored value (`name` column).
    pub name: String,
    /// Display label (single- or multi-language).
    pub label: Localized,
    /// Extra columns `(name, value)` for cascading filters, in header order.
    pub extra: Vec<(String, String)>,
}

/// A named list of choices from the `choices` sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceList {
    /// List name referenced by `select_*` questions.
    pub name: String,
    /// Options in sheet order.
    pub items: Vec<Choice>,
}

/// A container of child nodes: a group (`<group>`) or a repeat (`<repeat>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Container {
    /// Node name; becomes the enclosing instance element.
    pub name: String,
    /// Default-language label, if any.
    pub label: Option<String>,
    /// `appearance` column value (e.g. `field-list`), if any.
    pub appearance: Option<String>,
    /// `repeat_count` expression (repeats only), if any.
    pub count: Option<String>,
    /// Ordered child nodes.
    pub children: Vec<Node>,
}

/// A node in the survey tree: a question, a group, or a repeat.
#[expect(
    clippy::large_enum_variant,
    reason = "Question is by far the most common node; boxing it would add an allocation on the common path"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// A leaf question.
    Question(Question),
    /// A non-repeating group of nodes.
    Group(Container),
    /// A repeating group of nodes.
    Repeat(Container),
}

/// An ODK Entities declaration (the `entities` sheet): the survey creates an
/// entity in a dataset when submitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entity {
    /// Target dataset name.
    pub dataset: String,
    /// Label expression for the entity, if any.
    pub label: Option<String>,
    /// `entity_id` expression: present ⇒ update mode, absent ⇒ create.
    pub entity_id: Option<String>,
    /// `create_if` condition expression, if any.
    pub create_if: Option<String>,
    /// `update_if` condition expression, if any.
    pub update_if: Option<String>,
}

/// A parsed XLSForm survey: settings, a node tree and choice lists.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Survey {
    /// Form-level settings.
    pub settings: Settings,
    /// Ordered top-level nodes.
    pub children: Vec<Node>,
    /// Choice lists from the `choices` sheet.
    pub choices: Vec<ChoiceList>,
    /// Declared languages, in order; empty for a single-language form.
    pub languages: Vec<String>,
    /// Entities declaration from the `entities` sheet, if any.
    pub entity: Option<Entity>,
    /// Whether an `audit` metadata question is present.
    pub audit: bool,
    /// `xml-external` instance ids (each loads `jr://file/<id>.xml`).
    pub external_instances: Vec<String>,
    /// OSM tag lists from the `osm` sheet.
    pub osm_tags: Vec<ChoiceList>,
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
        action: None,
    };
    let input = Some(Control::Input);
    let preload = |preload, params| Some(Preload { preload, params });

    Some(match kind {
        "text" => b(input, "string", false, None),
        "integer" => b(input, "int", false, None),
        "decimal" => b(input, "decimal", false, None),
        "range" => b(Some(Control::Range), "int", false, None),
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
        "file" => b(
            Some(Control::Upload {
                mediatype: "application/*",
            }),
            "binary",
            false,
            None,
        ),
        "calculate" => b(None, "string", false, None),
        "hidden" => b(None, "string", false, None),
        "start" => b(None, "dateTime", false, preload("timestamp", "start")),
        "end" => b(None, "dateTime", false, preload("timestamp", "end")),
        "today" => b(None, "date", false, preload("date", "today")),
        "deviceid" => b(None, "string", false, preload("property", "deviceid")),
        "username" => b(None, "string", false, preload("property", "username")),
        "phonenumber" => b(None, "string", false, preload("property", "phonenumber")),
        "email" => b(None, "string", false, preload("property", "email")),
        "simserial" => b(None, "string", false, preload("property", "simserial")),
        "subscriberid" => b(None, "string", false, preload("property", "subscriberid")),
        "background-audio" => Builtin {
            control: None,
            bind_type: "binary",
            readonly: false,
            preload: None,
            action: Some(Action {
                element: "odk:recordaudio",
                event: "odk-instance-load",
            }),
        },
        "start-geopoint" => Builtin {
            control: None,
            bind_type: "geopoint",
            readonly: false,
            preload: None,
            action: Some(Action {
                element: "odk:setgeopoint",
                event: "odk-instance-first-load",
            }),
        },
        _ => return None,
    })
}
