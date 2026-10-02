//! Normalize a raw [`Workbook`] into the [`Survey`] model.
//!
//! This is the semantic core: alias resolution, type parsing, group/repeat
//! nesting, choices and settings. Phase 0 is a stub; real normalization
//! begins in Phase 1.

use rustxform_core::Survey;
use rustxform_reader::Workbook;

/// An error produced while normalizing a workbook into a survey.
///
/// No variants exist yet; parsing rules add them from Phase 1 onward.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {}

/// Normalize a workbook into a [`Survey`].
///
/// Phase 0 returns an empty survey unconditionally.
pub fn workbook_to_survey(_workbook: &Workbook) -> Result<Survey, ParseError> {
    Ok(Survey::default())
}
