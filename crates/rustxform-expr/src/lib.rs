//! Tokenize XLSForm expressions and rewrite `${name}` references to XPath.
//!
//! Full XPath is never evaluated: expressions are only tokenized so that
//! `${question}` references can be rewritten into absolute paths and
//! validated. rustxform uses a `logos` lexer for this in Phase 4. Phase 0 is
//! an identity stub.

/// Rewrite `${name}` references in an expression into XPath.
///
/// Phase 0 returns the input unchanged; the real tokenizer-driven rewrite is
/// implemented in Phase 4.
#[must_use]
pub fn rewrite_references(expr: &str) -> String {
    expr.to_owned()
}
