# Contributing to rustxform

Thanks for your interest in improving rustxform! This document explains the
workflow, the quality gates, and the one rule that is non-negotiable here:
**conformance**.

By participating you agree to abide by our
[Code of Conduct](CODE_OF_CONDUCT.md).

## Branching model (git flow)

- **`main`** — released, tagged history. Protected; never pushed to directly.
- **`develop`** — integration branch for the next release. Protected.
- **feature branches** — branch off `develop`, named `feat/…`, `fix/…`,
  `docs/…`, etc.

Flow: `feature → PR → develop`, and for a release `develop → PR → main` followed
by a tag. Every change reaches `main` and `develop` through a pull request; CI
must be green before merging.

```bash
git switch develop && git pull
git switch -c feat/my-change
# …work…
git push -u origin feat/my-change     # then open a PR into develop
```

## Quality gates

Every PR must pass CI, which runs:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo build --all-targets --all-features
cargo test --all-features
```

Run them locally before pushing. On a machine where the native toolchain is
unavailable, build and test inside Docker — see the
[README](README.md#development).

## Conformance is the core rule

rustxform's correctness is defined by producing **byte-identical** output to the
reference XLSForm compiler. A new feature or question type is not "done" until a
golden fixture proves it:

1. Add a form to `scripts/gen_corpus.py`.
2. Regenerate the corpus (needs Docker + the reference compiler) — see the
   [README](README.md#conformance). A `<name>.md` / `<name>.xml` pair is written
   **only if** rustxform already matches the reference after XML C14N.
3. Commit the pair. The data-driven test `golden_fixtures_match_reference` then
   enforces it with no Python needed.

For the reverse direction (`rustxform-xform2json`), add a form to the
`round_trips_forms` property test instead: it asserts `emit == emit∘parse∘emit`
byte-for-byte, no external oracle required.

**Never hand-write or hand-edit a golden `.xml`.** The whole point is that the
reference compiler — not us — defines the expected output.

## Style

Follow the surrounding code and the Rust API Guidelines; the project also tracks
the Microsoft "Pragmatic Rust Guidelines". Keep public items documented
(`missing_docs` is a warning), and prefer small, focused commits with clear
messages.

## Reporting bugs & proposing features

Open a GitHub issue with a minimal XLSForm that reproduces the problem and the
XForm you expected. For security issues, follow [SECURITY.md](SECURITY.md)
instead of filing a public issue.

## Licensing

Unless you state otherwise, contributions are dual-licensed under
[MIT](LICENSE-MIT) and [Apache-2.0](LICENSE-APACHE), matching the project.
