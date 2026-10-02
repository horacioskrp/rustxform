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
| 5 | Multilingual labels & advanced columns | ⬜ |
| 6 | XLSX/XLS/CSV readers & CLI parity | ⬜ |
| 7 | Validations | ⬜ |

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

## Phase 5 — Multilingual & advanced columns

- [ ] `label::Lang`, `hint`, `constraint_message`, media columns
- [ ] Multi-language `<itext>` and default language
- [ ] `parameters`, `appearance`, full `settings`
- [ ] **GO:** translation/parameter fixtures pass

## Phase 6 — Binary readers & CLI

- [ ] XLSX/XLS and CSV readers
- [ ] CLI option parity; warnings as JSON
- [ ] **GO:** real spreadsheet fixtures pass

## Phase 7 — Validations

- [ ] Unique names, broken references, choices, geo, range, settings
- [ ] Clear, actionable error messages
- [ ] **GO:** validation fixtures pass → v1 release
