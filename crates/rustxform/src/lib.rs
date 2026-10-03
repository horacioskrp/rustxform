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
use rustxform_reader::{ReadError, Workbook};
use rustxform_xform::XformError;

#[doc(inline)]
pub use rustxform_validate::ValidationError;

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
    convert_workbook(rustxform_reader::read_markdown(src)?)
}

/// Convert a CSV XLSForm into an XForm XML string.
///
/// # Errors
///
/// Returns [`ConvertError`] if any pipeline stage fails.
pub fn convert_csv(src: &str) -> Result<String, ConvertError> {
    convert_workbook(rustxform_reader::read_csv(src)?)
}

/// Convert an XLSX/XLS XLSForm (raw bytes) into an XForm XML string.
///
/// # Errors
///
/// Returns [`ConvertError`] if any pipeline stage fails.
pub fn convert_xlsx(bytes: &[u8]) -> Result<String, ConvertError> {
    convert_workbook(rustxform_reader::read_xlsx(bytes)?)
}

/// Parse a workbook and emit its XForm.
fn convert_workbook(workbook: Workbook) -> Result<String, ConvertError> {
    let survey = rustxform_parse::workbook_to_survey(&workbook)?;
    Ok(rustxform_xform::survey_to_xform(&survey)?)
}

/// An error from a validated build: a pipeline failure or validation problems.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// A read/parse/emit pipeline stage failed.
    #[error(transparent)]
    Convert(#[from] ConvertError),
    /// The form is structurally invalid.
    #[error("the form has {} validation error(s)", .0.len())]
    Invalid(Vec<ValidationError>),
}

/// Validate then convert a Markdown XLSForm.
///
/// # Errors
///
/// Returns [`BuildError::Invalid`] with every problem when the form is
/// invalid, or [`BuildError::Convert`] if a pipeline stage fails.
pub fn convert_markdown_checked(src: &str) -> Result<String, BuildError> {
    build_checked(rustxform_reader::read_markdown(src).map_err(ConvertError::from)?)
}

/// Validate then convert a CSV XLSForm.
///
/// # Errors
///
/// See [`convert_markdown_checked`].
pub fn convert_csv_checked(src: &str) -> Result<String, BuildError> {
    build_checked(rustxform_reader::read_csv(src).map_err(ConvertError::from)?)
}

/// Validate then convert an XLSX/XLS XLSForm.
///
/// # Errors
///
/// See [`convert_markdown_checked`].
pub fn convert_xlsx_checked(bytes: &[u8]) -> Result<String, BuildError> {
    build_checked(rustxform_reader::read_xlsx(bytes).map_err(ConvertError::from)?)
}

/// Parse, validate, then emit; returning all validation errors if any.
fn build_checked(workbook: Workbook) -> Result<String, BuildError> {
    let survey = rustxform_parse::workbook_to_survey(&workbook).map_err(ConvertError::from)?;
    let errors = rustxform_validate::validate(&survey);
    if !errors.is_empty() {
        return Err(BuildError::Invalid(errors));
    }
    Ok(rustxform_xform::survey_to_xform(&survey).map_err(ConvertError::from)?)
}
