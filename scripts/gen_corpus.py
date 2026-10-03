#!/usr/bin/env python3
"""Regenerate the conformance corpus under tests/fixtures/.

The *golden* XForm for each form is produced by **pyxform** (the reference
XLSForm compiler) and the pair is written only if, after XML C14N
canonicalization, it is byte-identical to **rustxform**'s own output. rustxform
cannot generate its own golden (that would be a tautological test), so this
step intentionally depends on pyxform + lxml, which are Python.

Each form is built as an .xlsx (fed to pyxform) and serialized to the Markdown
table format (fed to rustxform). For a match, both `<name>.md` and `<name>.xml`
are written; the Rust test runner (`cargo test`) then checks them with no
Python needed.

Usage (needs Docker; see scripts/docker-dev.ps1 for why we build in a container):

    # 1. Build the CLI binary (Linux) and copy it somewhere shared:
    docker run --rm -v "${PWD}:/work" -w /work \
      -v rustxform-target:/tmp/target -e CARGO_TARGET_DIR=/tmp/target \
      rust:latest bash -c "cargo build -q -p rustxform-cli && cp /tmp/target/debug/rustxform /work/.rustxform-bin"

    # 2. Run this script in a Python container with pyxform + lxml:
    docker run --rm -v "${PWD}:/work" -w /work \
      -e RUSTXFORM_BIN=/work/.rustxform-bin python:3.12-slim \
      bash -c "pip install --quiet pyxform openpyxl lxml && python scripts/gen_corpus.py"

Environment:
    RUSTXFORM_BIN   path to the built rustxform binary (required)
    FIXTURES_DIR    output dir (default: <repo>/crates/rustxform/tests/fixtures)
"""

import os
import subprocess
import sys
import tempfile
from pathlib import Path

try:
    from openpyxl import Workbook
    from lxml import etree
    from pyxform.xls2xform import xls2xform_convert
except ImportError as exc:  # pragma: no cover - environment guard
    sys.exit(f"missing dependency: {exc}. Install with: pip install pyxform openpyxl lxml")

D = chr(36)  # '$', kept out of shell interpolation
REPO = Path(__file__).resolve().parent.parent
FIX = Path(os.environ.get("FIXTURES_DIR", REPO / "crates/rustxform/tests/fixtures"))
RUST = os.environ.get("RUSTXFORM_BIN")
if not RUST or not Path(RUST).exists():
    sys.exit("set RUSTXFORM_BIN to the built rustxform binary (see the module docstring)")

S = [["form_title", "form_id"], ["F", "f"]]


def to_md(sheets):
    out = []
    for name, rows in sheets.items():
        out.append(f"| {name} |")
        for r in rows:
            cells = " | ".join("" if c is None else str(c) for c in r)
            out.append("|  | " + cells + " |")
    return "\n".join(out) + "\n"


def build_xlsx(path, sheets):
    wb = Workbook()
    first = True
    for name, rows in sheets.items():
        ws = wb.active if first else wb.create_sheet(name)
        if first:
            ws.title = name
            first = False
        for r in rows:
            ws.append(r)
    wb.save(path)


def canon(path):
    parser = etree.XMLParser(remove_blank_text=True)
    return etree.tostring(etree.parse(path, parser), method="c14n")


# The corpus: combined-feature forms inspired by pyxform's test scenarios.
FORMS = {
    "c_inputs": {"survey": [["type", "name", "label"],
        ["text", "t", "T"], ["integer", "i", "I"], ["decimal", "d", "D"], ["date", "da", "Da"],
        ["time", "ti", "Ti"], ["dateTime", "dt", "Dt"], ["geopoint", "gp", "Gp"],
        ["geotrace", "gt", "Gt"], ["geoshape", "gs", "Gs"], ["barcode", "bc", "Bc"]], "settings": S},
    "c_uploads": {"survey": [["type", "name", "label"],
        ["image", "im", "Im"], ["audio", "au", "Au"], ["video", "vi", "Vi"], ["file", "fi", "Fi"]], "settings": S},
    "c_meta": {"survey": [["type", "name", "label"],
        ["start", "s", ""], ["end", "e", ""], ["today", "td", ""], ["deviceid", "dv", ""],
        ["username", "un", ""], ["text", "q", "Q"]], "settings": S},
    "c_nested_groups": {"survey": [["type", "name", "label"],
        ["begin_group", "g1", "G1"], ["begin_group", "g2", "G2"], ["text", "x", "X"],
        ["end_group", "", ""], ["end_group", "", ""]], "settings": S},
    "c_group_repeat_group": {"survey": [["type", "name", "label"],
        ["begin_repeat", "r", "R"], ["begin_group", "gg", "GG"], ["text", "y", "Y"],
        ["end_group", "", ""], ["end_repeat", "", ""]], "settings": S},
    "c_two_repeats": {"survey": [["type", "name", "label"],
        ["begin_repeat", "ra", "RA"], ["text", "a", "A"], ["end_repeat", "", ""],
        ["begin_repeat", "rb", "RB"], ["integer", "b", "B"], ["end_repeat", "", ""]], "settings": S},
    "c_repeat_count": {"survey": [["type", "name", "label", "repeat_count"],
        ["begin_repeat", "rc", "RC", "3"], ["text", "ri", "RI", ""], ["end_repeat", "", "", ""]], "settings": S},
    "c_select_many": {"survey": [["type", "name", "label"],
        ["select_one yn", "s1", "S1"], ["select_multiple col", "s2", "S2"]],
        "choices": [["list_name", "name", "label"], ["yn", "y", "Yes"], ["yn", "n", "No"],
        ["col", "r", "Red"], ["col", "g", "Green"], ["col", "b", "Blue"], ["col", "w", "White"]], "settings": S},
    "c_cascade2": {"survey": [["type", "name", "label", "choice_filter"],
        ["select_one st", "st", "St", ""], ["select_one ci", "ci", "Ci", "st=" + D + "{st}"]],
        "choices": [["list_name", "name", "label", "st"], ["st", "tx", "TX", ""], ["st", "ca", "CA", ""],
        ["ci", "au", "Austin", "tx"], ["ci", "la", "LA", "ca"]], "settings": S},
    "c_multilang3": {"survey": [["type", "name", "label::English (en)", "label::French (fr)", "label::Spanish (es)", "hint::English (en)", "hint::French (fr)", "hint::Spanish (es)"],
        ["text", "n", "Name", "Nom", "Nombre", "H", "I", "P"]],
        "settings": [["form_title", "form_id", "default_language"], ["F", "f", "English (en)"]]},
    "c_logic_cross": {"survey": [["type", "name", "label", "relevant", "constraint", "calculation"],
        ["integer", "age", "Age", "", ". >= 0", ""],
        ["begin_group", "gg", "GG", "", "", ""],
        ["text", "nm", "Nm", D + "{age} >= 18", "", ""],
        ["end_group", "", "", "", "", ""],
        ["integer", "dbl", "Dbl", "", "", D + "{age} * 2"]], "settings": S},
    "c_appearances": {"survey": [["type", "name", "label", "appearance"],
        ["text", "t", "T", "multiline"], ["select_one yn", "s", "S", "minimal"]],
        "choices": [["list_name", "name", "label"], ["yn", "y", "Y"], ["yn", "n", "N"]], "settings": S},
    "c_defaults": {"survey": [["type", "name", "label", "default"],
        ["integer", "a", "A", "7"], ["date", "d", "D", "2020-01-01"], ["geopoint", "g", "G", "1 2 0 0"]], "settings": S},
    "c_note_output": {"survey": [["type", "name", "label"],
        ["text", "nm", "Name"], ["note", "hi", "Hi " + D + "{nm}"]], "settings": S},
    "c_messages": {"survey": [["type", "name", "label", "constraint", "constraint_message", "required", "required_message"],
        ["integer", "a", "A", ". > 0", "Positive!", "yes", "Needed"]], "settings": S},
    "c_media_multi": {"survey": [["type", "name", "label::English (en)", "label::French (fr)", "media::image::English (en)", "media::image::French (fr)"],
        ["note", "p", "Look", "Regarde", "en.png", "fr.png"]],
        "settings": [["form_title", "form_id", "default_language"], ["F", "f", "English (en)"]]},
    "c_settings_full": {"survey": [["type", "name", "label"], ["text", "q", "Q"]],
        "settings": [["form_title", "form_id", "version", "instance_name", "style"],
        ["F", "f", "2024", "q", "pages"]]},
    "c_entity_create": {"survey": [["type", "name", "label", "save_to"], ["text", "sp", "Sp", "species"]],
        "entities": [["dataset", "label"], ["trees", D + "{sp}"]], "settings": S},
    "c_entity_update": {"survey": [["type", "name", "label", "save_to"],
        ["text", "tid", "Tid", ""], ["integer", "c", "C", "circ"]],
        "entities": [["dataset", "entity_id", "label"], ["trees", D + "{tid}", D + "{c}"]], "settings": S},
    "c_rank": {"survey": [["type", "name", "label"], ["rank col", "r", "R"]],
        "choices": [["list_name", "name", "label"], ["col", "a", "A"], ["col", "b", "B"]], "settings": S},
    "c_osm": {"survey": [["type", "name", "label"], ["osm tags", "b", "B"]],
        "osm": [["list_name", "name", "label"], ["tags", "name", "Name"], ["tags", "height", "Height"]], "settings": S},
    "c_xmlexternal": {"survey": [["type", "name", "label"], ["xml-external", "md", ""], ["text", "q", "Q"]], "settings": S},
    "c_actions": {"survey": [["type", "name", "label", "parameters"],
        ["background-audio", "ba", "", "quality=low"], ["start-geopoint", "sg", "", ""], ["text", "q", "Q", ""]], "settings": S},
    "c_audit": {"survey": [["type", "name", "label"], ["audit", "audit", ""], ["text", "q", "Q"]], "settings": S},
    "c_hidden": {"survey": [["type", "name", "label"], ["hidden", "h", ""], ["text", "q", "Q"]], "settings": S},
}


def main():
    tmp = Path(tempfile.mkdtemp(prefix="corpus-"))
    FIX.mkdir(parents=True, exist_ok=True)
    written = skipped = 0
    for name, sheets in FORMS.items():
        xlsx, ref, md, got = (tmp / f"{name}.{ext}" for ext in ("xlsx", "xml", "md", "got.xml"))
        build_xlsx(xlsx, sheets)
        try:
            xls2xform_convert(str(xlsx), str(ref), validate=False)
        except Exception as exc:  # noqa: BLE001 - report and skip
            print(f"[SKIP pyxform] {name}: {str(exc)[:70]}")
            skipped += 1
            continue
        md.write_text(to_md(sheets))
        result = subprocess.run([RUST, str(md), str(got)], capture_output=True, text=True)
        if result.returncode != 0:
            print(f"[SKIP rust-err] {name}: {(result.stderr or result.stdout).strip()[:70]}")
            skipped += 1
            continue
        if canon(ref) == canon(got):
            (FIX / f"{name}.md").write_text(to_md(sheets))
            (FIX / f"{name}.xml").write_text(ref.read_text())
            written += 1
        else:
            print(f"[SKIP differ] {name}")
            skipped += 1
    print(f"\n== wrote {written} corpus fixtures / {skipped} skipped — {len(FORMS)} forms ==")
    return 1 if skipped else 0


if __name__ == "__main__":
    sys.exit(main())
