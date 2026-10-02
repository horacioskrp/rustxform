//! rustxform: compile an XLSForm into an XForm.
//!
//! This facade wires the pipeline end to end:
//! read → parse → emit. Phase 0 runs the full chain against stubs, so it
//! produces a well-formed XForm skeleton rather than a complete form.
//!
//! # Example
//!
//! ```
//! let xform = rustxform::convert_markdown("| survey |").unwrap();
//! assert!(xform.contains("h:html"));
//! ```

use rustxform_parse::ParseError;
use rustxform_reader::ReadError;
use rustxform_xform::XformError;

/// An error from the end-to-end conversion, one per pipeline stage.
#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    /// The workbook reader failed.
    #[error(transparent)]
    Read(#[from] ReadError),
    /// Normalizing the workbook into a survey failed.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// Emitting the XForm failed.
    #[error(transparent)]
    Xform(#[from] XformError),
}

/// Convert a Markdown XLSForm into an XForm XML string.
///
/// # Errors
///
/// Returns [`ConvertError`] if any pipeline stage fails.
pub fn convert_markdown(src: &str) -> Result<String, ConvertError> {
    let workbook = rustxform_reader::read_markdown(src)?;
    let survey = rustxform_parse::workbook_to_survey(&workbook)?;
    let xform = rustxform_xform::survey_to_xform(&survey)?;
    Ok(xform)
}
