//! Form validations.
//!
//! Checks (unique names, broken `${}` references, choices, geo, range,
//! settings, header misspellings) are added incrementally from Phase 7.
//! Phase 0 exposes the entry point only.

use rustxform_core::Survey;

/// A single validation problem found in a survey.
///
/// No variants exist yet; checks add them from Phase 7.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {}

/// Validate a [`Survey`], returning all problems found.
///
/// Phase 0 performs no checks and always returns an empty list.
#[must_use]
pub fn validate(_survey: &Survey) -> Vec<ValidationError> {
    Vec::new()
}
