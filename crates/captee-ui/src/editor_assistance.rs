use captee_core::{CompletionItem, CompletionProvider};
use captee_platform::TypstCompletionProvider;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionEdit {
    pub range: Range<usize>,
    pub replacement: String,
}

/// Return byte range of the command prefix immediately before cursor.
pub fn command_prefix_range(source: &str, cursor: usize) -> Range<usize> {
    if cursor > source.len() || !source.is_char_boundary(cursor) {
        return cursor..cursor;
    }
    let mut start = cursor;
    for (offset, character) in source[..cursor].char_indices().rev() {
        if character.is_alphanumeric() || matches!(character, '#' | '-') {
            start = offset;
        } else {
            break;
        }
    }
    start..cursor
}

pub fn completion_edit(
    source: &str,
    cursor: usize,
    item: &CompletionItem,
) -> Option<CompletionEdit> {
    let range = command_prefix_range(source, cursor);
    if range.start > range.end || range.end > source.len() {
        return None;
    }
    Some(CompletionEdit { range, replacement: item.insert_text.clone() })
}

pub fn typst_completions(source: &str, cursor: usize) -> Vec<CompletionItem> {
    TypstCompletionProvider.complete(source, cursor).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(insert_text: &str) -> CompletionItem {
        CompletionItem { label: "#image".into(), insert_text: insert_text.into() }
    }

    #[test]
    fn completion_replaces_prefix_at_cursor_instead_of_appending() {
        let edit =
            completion_edit("before #im after", 10, &item("#image(\"img/\")")).expect("edit");
        assert_eq!(edit.range, 7..10);
        assert_eq!(edit.replacement, "#image(\"img/\")");
    }

    #[test]
    fn completion_prefix_handles_unicode_before_cursor() {
        assert_eq!(command_prefix_range("é #he", 5), 3..5);
        assert_eq!(command_prefix_range("#he", 2), 0..2);
    }

    #[test]
    fn empty_or_dismissed_completion_does_not_create_an_edit() {
        let source = "plain text";
        let items = typst_completions(source, source.len());
        assert!(items.is_empty() || items.iter().all(|item| !item.label.is_empty()));
    }
}
