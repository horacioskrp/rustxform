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

Remaining ideas: multilingual `constraint_message`/media, choice filters,
`search()`/external selects, `search`/cascading selects, published crate.
