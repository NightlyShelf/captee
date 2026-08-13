//! Formatter and literal search/replace boundaries.

use std::fmt;
use std::ops::Range;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub trait Formatter {
    type Error;

    fn format(&self, source: &str) -> Result<String, Self::Error>;
}

#[derive(Debug, Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation<T> {
    Completed(T),
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplaceError {
    EmptyQuery,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceResult {
    pub text: String,
    pub replacements: usize,
}

pub fn find_literal(source: &str, query: &str) -> Result<Vec<Range<usize>>, ReplaceError> {
    if query.is_empty() {
        return Err(ReplaceError::EmptyQuery);
    }
    Ok(source.match_indices(query).map(|(start, _)| start..start + query.len()).collect())
}

pub fn replace_literal(
    source: &str,
    query: &str,
    replacement: &str,
    confirmed: bool,
) -> Result<Operation<ReplaceResult>, ReplaceError> {
    if query.is_empty() {
        return Err(ReplaceError::EmptyQuery);
    }
    if !confirmed {
        return Ok(Operation::Cancelled);
    }
    let replacements = source.match_indices(query).count();
    Ok(Operation::Completed(ReplaceResult {
        text: source.replace(query, replacement),
        replacements,
    }))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoringError(pub String);

impl fmt::Display for AuthoringError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for AuthoringError {}

#[cfg(test)]
mod tests {
    use super::*;

    struct IdentityFormatter;

    impl Formatter for IdentityFormatter {
        type Error = AuthoringError;

        fn format(&self, source: &str) -> Result<String, Self::Error> {
            Ok(source.trim().to_owned())
        }
    }

    #[test]
    fn formatter_is_a_trait_boundary() {
        assert_eq!(IdentityFormatter.format(" note ").expect("format"), "note");
    }

    #[test]
    fn find_and_confirmed_replace_are_literal() {
        assert_eq!(find_literal("a a", "a").expect("find"), vec![0..1, 2..3]);
        let result = replace_literal("a a", "a", "b", true).expect("replace");
        assert_eq!(
            result,
            Operation::Completed(ReplaceResult { text: "b b".to_owned(), replacements: 2 })
        );
    }

    #[test]
    fn cancelled_replace_does_not_mutate_source() {
        assert_eq!(replace_literal("a", "a", "b", false).expect("cancel"), Operation::Cancelled);
    }
}
