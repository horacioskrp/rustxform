//! Rewrite `${name}` references inside XLSForm expressions into XPath.
//!
//! Full XPath is never evaluated: expressions are only scanned for `${name}`
//! references, which are replaced by the XPath produced by a caller-supplied
//! resolver. Each replacement is padded with surrounding spaces, matching the
//! reference compiler's behavior.

/// Rewrite every `${name}` in `expr` using `resolve`.
///
/// `resolve(name)` returns the XPath for the referenced node, or `None` to
/// leave the `${name}` token untouched. Resolved references are substituted
/// as ` <xpath> ` (space-padded).
#[must_use]
pub fn rewrite_references(expr: &str, resolve: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(expr.len());
    let mut rest = expr;

    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find('}') {
            Some(end) => {
                let name = &after[..end];
                match resolve(name) {
                    Some(xpath) => {
                        out.push(' ');
                        out.push_str(&xpath);
                        out.push(' ');
                    }
                    None => {
                        out.push_str("${");
                        out.push_str(name);
                        out.push('}');
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                // Unterminated reference: emit verbatim and stop scanning.
                out.push_str("${");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_known_references_with_padding() {
        let out = rewrite_references("${age} >= 18", |name| {
            (name == "age").then(|| "/data/age".to_owned())
        });
        assert_eq!(out, " /data/age  >= 18");
    }

    #[test]
    fn leaves_unknown_references_untouched() {
        let out = rewrite_references("${ghost} + 1", |_| None);
        assert_eq!(out, "${ghost} + 1");
    }
}
