use captee_platform::{tinymist_function_arguments, LspPosition, LspRange, TinymistCompletion};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

pub type FunctionArgumentCache = BTreeMap<String, Vec<TinymistCompletion>>;

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
    if let Some(edit) = quoted_value_completion_edit(source, cursor, item) {
        return Some(edit);
    }
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

fn quoted_value_completion_edit(
    source: &str,
    cursor: usize,
    item: &TinymistCompletion,
) -> Option<CompletionEdit> {
    if cursor > source.len()
        || !source.is_char_boundary(cursor)
        || !item.label.starts_with('"')
        || !item.label.ends_with('"')
    {
        return None;
    }
    let previous_quote = source[..cursor].rfind('"')?;
    let (open, end) = if previous_quote + 1 == cursor {
        (source[..previous_quote].rfind('"')?, cursor)
    } else {
        let close = source[cursor..].find('"').map_or(cursor, |offset| cursor + offset + 1);
        (previous_quote, close)
    };
    let argument_start = source[..open].rfind([',', '(']).map_or(0, |offset| offset + 1);
    if !source[argument_start..open].contains(':') {
        return None;
    }
    Some(CompletionEdit {
        range: open..end,
        replacement: item.label.clone(),
        cursor: item.label.len(),
        selection: None,
    })
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

pub fn contextual_completion_items(
    source: &str,
    cursor: usize,
    items: Vec<TinymistCompletion>,
    argument_cache: &mut FunctionArgumentCache,
) -> Vec<TinymistCompletion> {
    for item in &items {
        let arguments = tinymist_function_arguments(item);
        if !arguments.is_empty() {
            argument_cache.insert(item.label.clone(), arguments);
        }
    }
    let Some(context) = function_argument_context(source, cursor) else {
        return items;
    };
    if !context.after_comma || context.in_value {
        return items;
    }
    argument_cache
        .get(&context.function)
        .into_iter()
        .flatten()
        .filter(|item| {
            !context.used_names.contains(&item.label) && item.label.starts_with(&context.prefix)
        })
        .cloned()
        .collect()
}

struct FunctionArgumentContext {
    function: String,
    prefix: String,
    used_names: BTreeSet<String>,
    after_comma: bool,
    in_value: bool,
}

fn function_argument_context(source: &str, cursor: usize) -> Option<FunctionArgumentContext> {
    let before_cursor = source.get(..cursor)?;
    let open = unmatched_call_parenthesis(before_cursor)?;
    let function = function_name(&before_cursor[..open])?;
    let segments = split_top_level_arguments(&before_cursor[open + 1..]);
    let current = segments.last()?.trim();
    let in_value = current.contains(':');
    let prefix = (!in_value
        && current.chars().all(|character| character.is_alphanumeric() || character == '-'))
    .then(|| current.to_owned())?;
    let used_names = segments
        .iter()
        .filter_map(|segment| segment.split_once(':').map(|(name, _)| name.trim()))
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect();
    Some(FunctionArgumentContext {
        function,
        prefix,
        used_names,
        after_comma: segments.len() > 1,
        in_value,
    })
}

fn unmatched_call_parenthesis(source: &str) -> Option<usize> {
    let mut closed = 0;
    for (offset, character) in source.char_indices().rev() {
        match character {
            ')' => closed += 1,
            '(' if closed > 0 => closed -= 1,
            '(' if is_typst_function_call(&source[..offset]) => return Some(offset),
            _ => {}
        }
    }
    None
}

fn function_name(before_parenthesis: &str) -> Option<String> {
    let trimmed = before_parenthesis.trim_end();
    let start = trimmed
        .char_indices()
        .rev()
        .take_while(|(_, character)| {
            character.is_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
        .last()?
        .0;
    trimmed[..start].ends_with('#').then(|| trimmed[start..].to_owned())
}

fn split_top_level_arguments(arguments: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    let mut quoted = false;
    for (offset, character) in arguments.char_indices() {
        match character {
            '"' => quoted = !quoted,
            '(' | '[' | '{' if !quoted => depth += 1,
            ')' | ']' | '}' if !quoted => depth -= 1,
            ',' if !quoted && depth == 0 => {
                result.push(&arguments[start..offset]);
                start = offset + 1;
            }
            _ => {}
        }
    }
    result.push(&arguments[start..]);
    result
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
    fn empty_string_argument_stub_places_caret_between_quotes() {
        let item = TinymistCompletion {
            label: "alt".into(),
            insert_text: "alt: \"${1:}\"".into(),
            range: None,
            is_snippet: true,
            description: Some("none | str".into()),
            detail: None,
        };
        let edit = tinymist_completion_edit("#image(\"x\", ", 12, &item).expect("edit");
        assert_eq!(edit.replacement, "alt: \"\"");
        assert_eq!(edit.cursor, 6);
        assert_eq!(edit.selection, None);
    }

    #[test]
    fn quoted_value_completion_replaces_existing_stub_value() {
        let item = TinymistCompletion {
            label: "\"png\"".into(),
            insert_text: "\"png".into(),
            range: Some(LspRange {
                start: LspPosition { line: 0, character: 23 },
                end: LspPosition { line: 0, character: 23 },
            }),
            is_snippet: true,
            description: None,
            detail: None,
        };
        let source = "#image(\"x\", format: \"p\")";
        let cursor = source.rfind('"').expect("closing quote");
        let edit = tinymist_completion_edit(source, cursor, &item).expect("edit");
        assert_eq!(&source[edit.range.clone()], "\"p\"");
        assert_eq!(edit.replacement, "\"png\"");

        let cursor = source.rfind('"').expect("closing quote") + 1;
        let edit = tinymist_completion_edit(source, cursor, &item).expect("edit after quote");
        assert_eq!(&source[edit.range], "\"p\"");
        assert_eq!(edit.replacement, "\"png\"");
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
    fn comma_completion_contains_only_unused_optional_arguments() {
        let function = TinymistCompletion {
            label: "image".into(),
            insert_text: "image(\"${1:image-path}\")".into(),
            range: None,
            is_snippet: true,
            description: Some(
                "([image], alt: none | str, fit: \"contain\" | \"cover\") => image".into(),
            ),
            detail: None,
        };
        let mut cache = FunctionArgumentCache::new();
        contextual_completion_items("#image", 6, vec![function], &mut cache);

        let source = "#image(\"image-path\", ";
        let items = contextual_completion_items(source, source.len(), Vec::new(), &mut cache);
        assert_eq!(
            items.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(),
            ["alt", "fit"]
        );

        let source = "#image(\"image-path\", alt: \"text\", f";
        let items = contextual_completion_items(source, source.len(), Vec::new(), &mut cache);
        assert_eq!(items.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(), ["fit"]);
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
