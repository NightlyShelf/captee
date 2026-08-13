use captee_platform::{LspPosition, LspRange, TinymistCompletion};
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionEdit {
    pub range: Range<usize>,
    pub replacement: String,
    pub cursor: usize,
    pub selection: Option<Range<usize>>,
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

pub fn visible_lsp_range_to_bytes(source: &str, range: LspRange) -> Option<Range<usize>> {
    let range = lsp_range_to_bytes(source, range)?;
    if range.start != range.end {
        return Some(range);
    }
    if range.start > 0 {
        let start = source[..range.start].char_indices().next_back()?.0;
        return Some(start..range.end);
    }
    source.chars().next().map(|character| range.start..character.len_utf8())
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
    if range.end > source.len() {
        return None;
    }
    let (replacement, cursor, selection) = if item.is_snippet {
        plain_text_snippet(&item.insert_text)
    } else {
        (item.insert_text.clone(), item.insert_text.len(), None)
    };
    Some(CompletionEdit { range, replacement, cursor, selection })
}

fn plain_text_snippet(snippet: &str) -> (String, usize, Option<Range<usize>>) {
    let bytes = snippet.as_bytes();
    let mut output = String::with_capacity(snippet.len());
    let mut first_tab_stop = None;
    let mut first_selection = None;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\\' && index + 1 < bytes.len() {
            output.push(bytes[index + 1] as char);
            index += 2;
            continue;
        }
        if bytes[index] != b'$' {
            let character = snippet[index..].chars().next().expect("character boundary");
            output.push(character);
            index += character.len_utf8();
            continue;
        }
        let start = index;
        index += 1;
        if index < bytes.len() && bytes[index].is_ascii_digit() {
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
            first_tab_stop.get_or_insert(output.len());
            continue;
        }
        if index < bytes.len() && bytes[index] == b'{' {
            index += 1;
            let number_start = index;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
            if number_start == index {
                output.push('$');
                index = start + 1;
                continue;
            }
            first_tab_stop.get_or_insert(output.len());
            if index < bytes.len() && bytes[index] == b':' {
                index += 1;
                let default_start = index;
                while index < bytes.len() && bytes[index] != b'}' {
                    index += 1;
                }
                let selection_start = output.len();
                output.push_str(&snippet[default_start..index]);
                if first_selection.is_none() && output.len() > selection_start {
                    first_selection = Some(selection_start..output.len());
                }
            } else {
                while index < bytes.len() && bytes[index] != b'}' {
                    index += 1;
                }
            }
            if index < bytes.len() {
                index += 1;
            }
            continue;
        }
        output.push('$');
    }
    let cursor = first_tab_stop.unwrap_or(output.len());
    (output, cursor, first_selection)
}

pub fn has_typst_command_prefix(source: &str, cursor: usize) -> bool {
    source.get(command_prefix_range(source, cursor)).is_some_and(|prefix| prefix.starts_with('#'))
}

pub fn should_request_tinymist_completion(source: &str, cursor: usize) -> bool {
    if cursor > source.len() || !source.is_char_boundary(cursor) {
        return false;
    }
    if has_typst_command_prefix(source, cursor) {
        return true;
    }

    let before_cursor = &source[..cursor];
    let mut closed_parentheses = 0;
    for (offset, character) in before_cursor.char_indices().rev() {
        match character {
            ')' => closed_parentheses += 1,
            '(' if closed_parentheses > 0 => closed_parentheses -= 1,
            '(' if is_typst_function_call(&before_cursor[..offset]) => return true,
            _ => {}
        }
    }
    false
}

fn is_typst_function_call(before_parenthesis: &str) -> bool {
    let trimmed = before_parenthesis.trim_end();
    let name_start = trimmed
        .char_indices()
        .rev()
        .take_while(|(_, character)| {
            character.is_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
        .last()
        .map_or(trimmed.len(), |(offset, _)| offset);
    name_start < trimmed.len() && trimmed[..name_start].ends_with('#')
}

pub fn completion_response_is_current(
    expected_uri: &str,
    expected_version: i32,
    latest_request: Option<u64>,
    uri: &str,
    version: i32,
    request_id: u64,
) -> bool {
    expected_uri == uri && expected_version == version && latest_request == Some(request_id)
}

pub fn diagnostics_response_is_current(
    expected_uri: &str,
    expected_version: i32,
    uri: &str,
    version: Option<i32>,
) -> bool {
    expected_uri == uri && version.is_none_or(|version| version == expected_version)
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
        let item = TinymistCompletion {
            label: "image".into(),
            insert_text: "image".into(),
            range: None,
            is_snippet: false,
            description: None,
            detail: None,
        };
        let edit = tinymist_completion_edit("#im", 3, &item).expect("edit");
        assert_eq!(edit.range, 1..3);
    }

    #[test]
    fn snippet_placeholders_are_hidden_and_cursor_uses_first_tab_stop() {
        let item = TinymistCompletion {
            label: "align".into(),
            insert_text: "align(${1:})$0".into(),
            range: None,
            is_snippet: true,
            description: None,
            detail: None,
        };
        let edit = tinymist_completion_edit("#ali", 4, &item).expect("edit");
        assert_eq!(edit.replacement, "align()");
        assert_eq!(edit.cursor, 6);
        assert_eq!(edit.selection, None);
    }

    #[test]
    fn snippet_placeholder_text_is_selected_for_replacement() {
        let item = TinymistCompletion {
            label: "align".into(),
            insert_text: "align(${1:alignment}, ${2:body})".into(),
            range: None,
            is_snippet: true,
            description: None,
            detail: None,
        };
        let edit = tinymist_completion_edit("#ali", 4, &item).expect("edit");
        assert_eq!(edit.replacement, "align(alignment, body)");
        assert_eq!(edit.selection, Some(6..15));
    }

    #[test]
    fn zero_width_diagnostic_marks_the_previous_character() {
        let range = LspRange {
            start: LspPosition { line: 0, character: 1 },
            end: LspPosition { line: 0, character: 1 },
        };
        assert_eq!(visible_lsp_range_to_bytes("#", range), Some(0..1));
    }

    #[test]
    fn completion_requests_continue_inside_typst_function_calls() {
        for source in ["#image(", "#image(\n  wi", "#image(width: 2cm, fit: \"co"] {
            assert!(should_request_tinymist_completion(source, source.len()));
        }
        for source in ["text (without command", "#image() text"] {
            assert!(!should_request_tinymist_completion(source, source.len()));
        }
    }

    #[test]
    fn stale_lsp_responses_are_rejected() {
        assert!(completion_response_is_current("main", 4, Some(8), "main", 4, 8));
        assert!(!completion_response_is_current("main", 4, Some(9), "main", 3, 8));
        assert!(!completion_response_is_current("main", 4, Some(9), "main", 4, 8));
        assert!(diagnostics_response_is_current("main", 4, "main", Some(4)));
        assert!(!diagnostics_response_is_current("main", 4, "main", Some(3)));
        assert!(!diagnostics_response_is_current("main", 4, "capture", None));
    }
}
