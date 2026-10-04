# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and the project aims to adhere
to [Semantic Versioning](https://semver.org/).

## [0.2.0] - 2026-10-04

Broader XLSForm parity and a near-complete reverse parser.

### Added

- **Parity (compile)**: `hidden`; `trigger` + `calculation` dynamic recalculation
  (`odk:setvalue`); `select_* … or_other`; `guidance_hint`; encrypted forms
  (`public_key` → `<submission base64RsaPublicKey>`); submission settings
  (`submission_url` / `auto_send` / `auto_delete`); upload `appearance` and image
  `max-pixels`; `pulldata()` CSV instances; `background-geopoint`. The golden
  corpus grows to 34 forms, all byte-identical after C14N.
- **Reverse (`xform2json`)**: recovers settings (version/style/instance_name),
  the group/repeat tree (with `repeat_count`), full bind logic and messages,
  `appearance`, inline choice lists and `select_*_from_file`, top-level data-only
  nodes (`calculate`, metadata preloads, audit), entities (create/update,
  `save_to`), and `<itext>` — multilingual labels/hints/messages/choice labels,
  label media, and single-language guidance/media. Locked by a round-trip
  property test (`emit == emit∘parse∘emit`, byte-identical) over 20 forms.

### Known limitations

- Reverse parsing does not recover data-only nodes nested inside groups/repeats,
  or reconstruct `pulldata()` CSV instances.

## [0.1.1] - 2026-10-03

Documentation release — no functional or API changes.

### Changed

- Each crate now ships its own detailed `README.md` (role, pipeline position,
  public API, a runnable example), shown on its crates.io page.
- Added crates.io metadata (`keywords`, `categories`) to every crate.

## [0.1.0] - 2026-10-03

First release: a Rust library and CLI that compile an XLSForm into an XForm,
verified against the reference compiler on a golden corpus.

### Added

- **Inputs**: Markdown tables, CSV, and XLSX/XLS (via `calamine`); the CLI picks
  the reader by file extension.
- **Question types**: text, integer, decimal, range (with `parameters` and
  defaults), date, time, dateTime, geopoint/geotrace/geoshape, barcode,
  image/audio/video/file uploads, note, acknowledge, calculate, and the
  `phone number` preset.
- **Selects**: `select_one` / `select_multiple` / `rank`, inline choice lists or
  `select_*_from_file` external instances, cascading `choice_filter` with extra
  choice columns, and `osm` tag captures.
- **Structure**: groups, repeats (`jr:template` + `repeat_count`/`jr:count`),
  arbitrary nesting, and `field-list`.
- **Logic**: `${…}` reference rewriting (absolute, or relative within a shared
  repeat; `last-saved#…`), applied to `relevant` / `constraint` / `required` /
  `read_only` / `calculation` / `choice_filter`, and `${…}` outputs in labels.
- **Localization**: multi-language `label` / `hint` / `constraint_message` /
  `required_message` / media via `<itext>`, default language, and single-
  language inline labels; `media::` images/audio/video (single or per-language).
- **Metadata & actions**: `start`/`end`/`today`/`deviceid`/`username`/
  `phonenumber`/`email`/`simserial`/`subscriberid` preloads, `start-geopoint`
  (`odk:setgeopoint`), `background-audio` (`odk:recordaudio`), and `audit`.
- **Entities**: create and update forms, `save_to`, `create_if` / `update_if`.
- **Settings**: `form_title`, `form_id`, `version`, `instance_name`, `style`,
  `default_language`.
- **Validation**: duplicate/empty names, broken references, unknown choice
  lists, malformed range parameters and geo defaults; non-fatal warnings for
  missing settings, unrecognized survey columns, and unknown language tags
  (ISO 639-1). The CLI rejects invalid forms and offers a `--json` report.
- **Reverse**: `rustxform-xform2json` recovers a survey model from an XForm
  (title, id, flat questions).
- **ODK Validate**: optional `--odk-validate <jar>` runs the Java validator on
  the output.
- **Tests**: a data-driven corpus of 53 golden fixtures, each byte-identical to
  pyxform after C14N; a type cross-check passes 32/32. Regenerate with
  `scripts/gen_corpus.py`.

### Known limitations

- `xform2json` reconstructs a flat subset (no groups/itext/choices/entities).
- A few exotic widgets are not implemented; unknown types degrade to a plain
  string bind rather than failing.

[0.2.0]: https://github.com/horacioskrp/rustxform/releases/tag/v0.2.0
[0.1.1]: https://github.com/horacioskrp/rustxform/releases/tag/v0.1.1
[0.1.0]: https://github.com/horacioskrp/rustxform/releases/tag/v0.1.0
