//! Read an XLSForm source into a raw [`Workbook`] of string rows.
//!
//! Three input shapes are supported:
//! - **Markdown** and **CSV** use the "first column selects the sheet"
//!   convention: a row whose first cell is non-empty opens a new sheet, and
//!   rows whose first cell is empty add cells (from column 2) to it.
//! - **XLSX/XLS** map each worksheet directly to a [`Sheet`].
//!
//! ```text
//! | survey  |         |      |           |
//! |         | type    | name | label     |
//! |         | text    | q1   | Question  |
//! ```

use std::io::Cursor;

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
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    /// Failed to open or read a spreadsheet (XLSX/XLS).
    #[error("spreadsheet error: {0}")]
    Spreadsheet(String),
    /// Failed to parse CSV input.
    #[error("csv error: {0}")]
    Csv(String),
}

/// Read a Markdown table workbook.
///
/// # Errors
///
/// Infallible for Markdown; returns `Result` for a uniform reader API.
pub fn read_markdown(src: &str) -> Result<Workbook, ReadError> {
    let rows = src.lines().filter_map(|line| {
        let line = line.trim();
        line.starts_with('|').then(|| split_pipe_row(line))
    });
    Ok(assemble(rows))
}

/// Read a CSV workbook using the same "first column selects the sheet"
/// convention as Markdown.
///
/// # Errors
///
/// Returns [`ReadError::Csv`] if the CSV cannot be parsed.
pub fn read_csv(src: &str) -> Result<Workbook, ReadError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(src.as_bytes());

    let mut rows: Vec<Vec<String>> = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|e| ReadError::Csv(e.to_string()))?;
        rows.push(record.iter().map(|c| c.trim().to_owned()).collect());
    }
    Ok(assemble(rows))
}

/// Read an XLSX or XLS workbook, mapping each worksheet to a [`Sheet`].
///
/// # Errors
///
/// Returns [`ReadError::Spreadsheet`] if the workbook cannot be read.
pub fn read_xlsx(bytes: &[u8]) -> Result<Workbook, ReadError> {
    use calamine::Reader;

    let mut book = calamine::open_workbook_auto_from_rs(Cursor::new(bytes))
        .map_err(|e| ReadError::Spreadsheet(e.to_string()))?;

    let mut workbook = Workbook::default();
    for name in book.sheet_names() {
        let range = book
            .worksheet_range(&name)
            .map_err(|e| ReadError::Spreadsheet(e.to_string()))?;
        let rows = range
            .rows()
            .map(|row| row.iter().map(cell_to_string).collect())
            .collect();
        workbook.sheets.push(Sheet { name, rows });
    }
    Ok(workbook)
}

/// Assemble rows into sheets using the first-column-selects-the-sheet rule.
fn assemble(rows: impl IntoIterator<Item = Vec<String>>) -> Workbook {
    let mut workbook = Workbook::default();
    for cells in rows {
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
    workbook
}

/// Split `| a | b | c |` into trimmed cells `[a, b, c]`.
fn split_pipe_row(line: &str) -> Vec<String> {
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

/// Render a spreadsheet cell as a string, as XLSForm authoring expects.
fn cell_to_string(cell: &calamine::Data) -> String {
    use calamine::Data;
    match cell {
        Data::String(s) => s.clone(),
        Data::Int(i) => i.to_string(),
        Data::Float(f) if f.fract() == 0.0 => (*f as i64).to_string(),
        Data::Float(f) => f.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        _ => String::new(),
    }
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
        assert_eq!(survey.rows.len(), 2);
        assert_eq!(survey.rows[1], ["text", "q1"]);
    }

    #[test]
    fn csv_uses_the_same_sheet_convention() {
        let src = "survey,,\n,type,name\n,text,q1\nsettings,\n,form_id\n,demo\n";
        let wb = read_csv(src).unwrap();
        assert_eq!(wb.sheet("survey").unwrap().rows[1], ["text", "q1"]);
        assert_eq!(wb.sheet("settings").unwrap().rows[1], ["demo"]);
    }
}
