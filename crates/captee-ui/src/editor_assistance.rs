use captee_platform::{LspPosition, LspRange, TinymistCompletion};
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

pub fn lsp_position(source: &str, cursor: usize) -> Option<LspPosition> {
    if cursor > source.len() || !source.is_char_boundary(cursor) {
        return None;
    }
    let prefix = &source[..cursor];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let line_start = prefix.rfind('\n').map_or(0, |offset| offset + 1);
    Some(LspPosition {
        line: u32::try_from(line).ok()?,
        character: u32::try_from(source[line_start..cursor].encode_utf16().count()).ok()?,
    })
}

pub fn lsp_range_to_bytes(source: &str, range: LspRange) -> Option<Range<usize>> {
    let start = lsp_position_to_byte(source, range.start)?;
    let end = lsp_position_to_byte(source, range.end)?;
    (start <= end).then_some(start..end)
}

pub fn tinymist_completion_edit(
    source: &str,
    cursor: usize,
    item: &TinymistCompletion,
) -> Option<CompletionEdit> {
    let mut range = item
        .range
        .and_then(|range| lsp_range_to_bytes(source, range))
        .unwrap_or_else(|| command_prefix_range(source, cursor));
    if source.get(range.clone()).is_some_and(|prefix| prefix.starts_with('#'))
        && !item.insert_text.starts_with('#')
    {
        range.start += 1;
    }
    (range.end <= source.len())
        .then(|| CompletionEdit { range, replacement: item.insert_text.clone() })
}

pub fn has_typst_command_prefix(source: &str, cursor: usize) -> bool {
    source.get(command_prefix_range(source, cursor)).is_some_and(|prefix| prefix.starts_with('#'))
}

fn lsp_position_to_byte(source: &str, position: LspPosition) -> Option<usize> {
    let mut line_start = 0;
    for _ in 0..position.line {
        let newline = source[line_start..].find('\n')?;
        line_start += newline + 1;
    }
    let line_end =
        source[line_start..].find('\n').map_or(source.len(), |offset| line_start + offset);
    let line = &source[line_start..line_end];
    let target = usize::try_from(position.character).ok()?;
    let mut units = 0;
    for (offset, character) in line.char_indices() {
        if units == target {
            return Some(line_start + offset);
        }
        units += character.len_utf16();
        if units > target {
            return None;
        }
    }
    (units == target).then_some(line_end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_prefix_handles_unicode_before_cursor() {
        assert_eq!(command_prefix_range("é #he", 5), 3..5);
        assert_eq!(command_prefix_range("#he", 2), 0..2);
    }

    #[test]
    fn lsp_positions_use_utf16_characters() {
        assert_eq!(lsp_position("one\n😀#im", 11), Some(LspPosition { line: 1, character: 5 }));
        let range = LspRange {
            start: LspPosition { line: 1, character: 2 },
            end: LspPosition { line: 1, character: 5 },
        };
        assert_eq!(lsp_range_to_bytes("one\n😀#im", range), Some(8..11));
    }

    #[test]
    fn tinymist_edit_preserves_hash_for_plain_insert_text() {
        let item =
            TinymistCompletion { label: "image".into(), insert_text: "image".into(), range: None };
        let edit = tinymist_completion_edit("#im", 3, &item).expect("edit");
        assert_eq!(edit.range, 1..3);
    }
}
