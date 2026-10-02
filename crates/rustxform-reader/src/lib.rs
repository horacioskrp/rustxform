//! Read an XLSForm source into a raw [`Workbook`] of string rows.
//!
//! Phase 1 implements the Markdown table reader used throughout the tests.
//! XLSX/XLS/CSV backends are added later.
//!
//! ## Markdown table format
//!
//! The first column selects the sheet: a row whose first cell is non-empty
//! opens a new sheet (the rest of that row is ignored); rows whose first cell
//! is empty contribute cells (from column 2 onward) to the current sheet. The
//! first such row is the header.
//!
//! ```text
//! | survey  |         |      |           |
//! |         | type    | name | label     |
//! |         | text    | q1   | Question  |
//! ```

/// A raw workbook: an ordered collection of named sheets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Workbook {
    /// Sheets in source order (typically `survey`, `choices`, `settings`).
    pub sheets: Vec<Sheet>,
}

impl Workbook {
    /// Return the sheet with the given name, if present.
    #[must_use]
    pub fn sheet(&self, name: &str) -> Option<&Sheet> {
        self.sheets.iter().find(|s| s.name == name)
    }
}

/// A single worksheet: a name and a grid of string cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sheet {
    /// Sheet name.
    pub name: String,
    /// Rows of cells; the first row is conventionally the header.
    pub rows: Vec<Vec<String>>,
}

/// An error produced while reading a workbook.
///
/// No variants exist yet; the binary (XLSX/XLS/CSV) backends add them later.
#[derive(Debug, thiserror::Error)]
pub enum ReadError {}

/// Read a Markdown table workbook.
///
/// # Errors
///
/// Currently infallible for Markdown input; the signature reserves room for
/// the binary backends.
pub fn read_markdown(src: &str) -> Result<Workbook, ReadError> {
    let mut workbook = Workbook::default();

    for line in src.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let cells = split_row(line);
        if is_separator(&cells) {
            continue;
        }

        let first = cells.first().map(String::as_str).unwrap_or_default();
        if first.is_empty() {
            if let Some(sheet) = workbook.sheets.last_mut() {
                sheet.rows.push(cells[1..].to_vec());
            }
        } else {
            workbook.sheets.push(Sheet {
                name: first.to_owned(),
                rows: Vec::new(),
            });
        }
    }

    Ok(workbook)
}

/// Split `| a | b | c |` into trimmed cells `[a, b, c]`.
fn split_row(line: &str) -> Vec<String> {
    let trimmed = line
        .strip_prefix('|')
        .unwrap_or(line)
        .strip_suffix('|')
        .unwrap_or(line);
    trimmed.split('|').map(|c| c.trim().to_owned()).collect()
}

/// A Markdown separator row such as `|---|:--:|` carries no data.
fn is_separator(cells: &[String]) -> bool {
    let has_dash = cells.iter().any(|c| c.contains('-'));
    has_dash
        && cells
            .iter()
            .all(|c| c.chars().all(|ch| ch == '-' || ch == ':'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_sheets_headers_and_rows() {
        let src = "\
| survey |         |      |           |
|        | type    | name | label     |
|        | text    | q1   | Question  |
| settings |            |         |
|          | form_title | form_id |
|          | Demo       | demo    |
";
        let wb = read_markdown(src).unwrap();
        assert_eq!(wb.sheets.len(), 2);

        let survey = wb.sheet("survey").unwrap();
        assert_eq!(survey.rows[0], ["type", "name", "label"]);
        assert_eq!(survey.rows[1], ["text", "q1", "Question"]);

        let settings = wb.sheet("settings").unwrap();
        assert_eq!(settings.rows[1], ["Demo", "demo"]);
    }

    #[test]
    fn skips_separator_rows() {
        let src = "\
| survey |      |
|        | type | name |
|        | ---  | ---  |
|        | text | q1   |
";
        let survey = read_markdown(src).unwrap().sheet("survey").unwrap().clone();
        // header + one data row; the separator line is dropped.
        assert_eq!(survey.rows.len(), 2);
        assert_eq!(survey.rows[1], ["text", "q1"]);
    }
}
