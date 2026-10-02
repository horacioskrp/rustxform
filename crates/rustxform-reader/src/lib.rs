//! Read an XLSForm source into a raw [`Workbook`] of string rows.
//!
//! Phase 0 ships the type surface and a Markdown reader stub. The real
//! Markdown table reader lands in Phase 1; XLSX/XLS/CSV backends later.

/// A raw workbook: an ordered collection of named sheets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Workbook {
    /// Sheets in source order (typically `survey`, `choices`, `settings`).
    pub sheets: Vec<Sheet>,
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
/// No variants exist yet; the Markdown reader adds them in Phase 1.
#[derive(Debug, thiserror::Error)]
pub enum ReadError {}

/// Read a Markdown table workbook.
///
/// Phase 0 returns an empty workbook unconditionally; the real parser is
/// implemented in Phase 1.
pub fn read_markdown(_src: &str) -> Result<Workbook, ReadError> {
    Ok(Workbook::default())
}
