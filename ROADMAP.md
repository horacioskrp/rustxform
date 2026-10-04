# Roadmap

`rustxform` is built phase by phase, each gated by a **GO** check: a phase is
done only when its whole checklist and all tests pass. Correctness is driven by
golden tests (compile a form, compare the XForm after XML canonicalization).

| Phase | Theme | Status |
| ----- | ------------------------------------------- | ------ |
| 0 | Workspace, CI & conformance harness | ✅ done |
| 1 | Walking skeleton — one form end to end | ✅ done |
| 2 | Question types & choices | ✅ done |
| 3 | Groups & repeats | ✅ done |
| 4 | Expressions & `${ref}` rewriting | ✅ done |
| 5 | Multilingual labels & advanced columns | ✅ done |
| 6 | XLSX/XLS/CSV readers & CLI parity | ✅ done |
| 7 | Validations | ✅ done |
| 8 | Broaden parity (dynamic actions & niche types) | ✅ done |
| 9 | Enrich `xform2json` toward round-trip | 🚧 in progress |

## Phase 0 — Workspace & oracle ✅

- [x] Cargo workspace (edition 2024), pipeline crates, CLI
- [x] CI: `fmt`, `clippy -D warnings`, `build`, `test`
- [x] Conformance harness: XML canonicalization + unified diff
- [x] Green build/test/lint

## Phase 1 — Walking skeleton ✅

A flat form with `text` / `integer` / `note` questions compiles end to end.

- [x] Markdown table reader (with unit tests)
- [x] Parse `survey` + `settings` sheets into the survey model
- [x] Emit `<instance>`, `<bind>` and `<body>` controls
- [x] **GO:** the simple-form golden test passes

## Phase 2 — Question types & choices ✅

- [x] `select_one` / `select_multiple` with a choices sheet and secondary instances
- [x] Type table (text/integer/decimal/date/time/dateTime/geo*/barcode/note/
      acknowledge/image/audio/video/calculate) and metadata preloads
      (start/end/today/deviceid)
- [x] **GO:** choices and types/metadata golden fixtures pass

## Phase 3 — Groups & repeats ✅

- [x] `begin/end group`, `begin/end repeat`, arbitrary nesting (tree model)
- [x] Repeat emits a `jr:template` plus one live instance; full nested paths
- [x] **GO:** group/repeat golden fixture passes

## Phase 4 — Expressions & references ✅

- [x] Scan and rewrite `${name}` references (space-padded, like the reference)
- [x] Absolute paths, and relative paths when a repeat ancestor is shared
- [x] Applied to `relevant`/`constraint`/`required`/`read_only`/`calculation`
- [x] **GO:** logic/references golden fixture passes

## Phase 5 — Multilingual & advanced columns ✅

- [x] `label::Lang` / `hint::Lang` → `<itext>` translations, default language
- [x] Choice itext (`itextId`) and `jr:itext(...)` label/hint references
- [x] `appearance` on controls; single-language forms keep inline labels
- [x] **GO:** multilingual golden fixture passes
- [ ] Deferred: `constraint_message`, media columns, `parameters`, `${}` label outputs

## Phase 6 — Binary readers & CLI ✅

- [x] XLSX/XLS reader (calamine) and CSV reader (shares the sheet convention)
- [x] CLI dispatches by extension (`.md` / `.csv` / `.xlsx` / `.xls`)
- [x] **GO:** `.xlsx` and `.csv` fixtures produce the same XForm as Markdown

## Phase 7 — Validations ✅

- [x] Duplicate/empty node names, broken `${…}` references, unknown choice lists
- [x] Clear, actionable error messages; `*_checked` APIs and CLI rejection
- [x] **GO:** valid forms pass, invalid forms are rejected with the right error

v1 scope complete.

## Post-v1 (done)

- [x] Advanced columns: `constraint_message`/`required_message`, `parameters`
      (range `start`/`end`/`step`), `${}` outputs in labels, `media::` images/
      audio/video (itext), `default` values
- [x] Validations: range parameters, geo defaults; warnings for missing
      settings and unrecognized survey columns
- [x] CLI `--json` report (status / warnings / errors)
- [x] More metadata preloads (username, phonenumber, email, simserial,
      subscriberid)
- [x] `field-list` group appearance, `repeat_count` (`jr:count`)
- [x] Settings: `version`, `instance_name`, `style`

Also done: `select_*_from_file`, `last-saved#`, multilingual
`constraint_message`/`required_message`, `choice_filter` (cascading selects),
multilingual `media::`, and language-tag warnings (ISO 639-1 list + 3-letter
shape; unlimited languages per form).

Also done (v2): **Entities** (create-entity forms), **xform2json** (reverse
parser, flat subset), **ODK Validate** integration (`--odk-validate <jar>`).

Parity widgets: **rank**, **background-audio** (`odk:recordaudio`),
**start-geopoint** (`odk:setgeopoint`), **audit**, **osm** (`osm/*` upload with
tags), **xml-external**, and entity **update** / **create_if** / **update_if**.
Cross-checked against pyxform: all supported forms are byte-identical after C14N.

Intentionally out of scope: `choices` header-coherence warnings (would flag
legitimate cascade columns).

## Phase 8 — Broaden parity ✅

Each item lands with a golden fixture (oracle-generated, byte-identical after
C14N) before it is checked off.

- [x] `hidden` type (string bind, no body control)
- [x] `trigger` + `calculation` → `odk:setvalue` dynamic recalculation
      (`event="xforms-value-changed"`, injected into the trigger node's control)
- [x] `or_other` on `select_one` / `select_multiple` (synthetic "other" choice +
      linked text question + `selected(../q, 'other')` relevant; single-language)
- [x] `guidance_hint` (itext `<value form="guidance">`, forces itext when present)
- [x] Encrypted forms: `public_key` → `<submission base64RsaPublicKey=…>`
- [x] Submission settings: `submission_url`, `auto_send`, `auto_delete`
- [x] Appearance on uploads (`annotate`/`signature`/`draw`) and image
      `max-pixels` → `orx:max-pixels` bind attribute
- [x] `pulldata()` → synthesized CSV external instance (`jr://file-csv/…`)
- [x] `background-geopoint` (triggered `odk:setgeopoint`, no value)
- [x] **GO:** new parity fixtures pass; full 34-form corpus byte-identical

## Phase 9 — Enrich `xform2json` 🚧

Grow the reverse parser from a flat subset toward a faithful inverse, locked in
by a round-trip property test.

- [x] Full binds: `relevant`, `constraint`, `required`, `read_only`,
      `calculate`, constraint/required messages (recovered as literal XPath)
- [x] Recover inline `label`/`hint` and control `appearance`
- [x] Reconstruct the group/repeat tree (`<group>`/`<repeat>`, `repeat_count`)
- [x] Round-trip property test: `emit == emit∘parse∘emit` holds byte-identically
      on single-language forms (types, logic, messages, hint, appearance, and
      groups / repeats / counted repeats / nesting)
- [ ] XPath → `${name}` recovery (cosmetic; round-trip already holds via
      literal pass-through, since the emitter only rewrites `${…}`)
- [x] Reconstruct inline choice lists from secondary instances (name/label +
      cascade `extra` columns); select questions keep their list id
- [x] Recover top-level data-only nodes (`calculate`, metadata preloads, audit)
      by merging the primary-instance order with the body tree (nested data-only
      deferred)
- [x] Decode `<itext>` into multilingual `label`/`hint`, choice labels,
      `languages` and `default_language` (media/message itext deferred)
- [x] Recover entities (dataset + create/update, `@id`/label/`create_if`/
      `update_if`, `save_to`) from `<meta><entity>` and its calculate binds
- [x] Recover settings: `version`, `style`, `instance_name`
- [x] Recover `select_*_from_file` (external CSV/XML instance → `file`)
- [ ] **GO:** round-trip is idempotent on the full corpus (not just flat forms)
