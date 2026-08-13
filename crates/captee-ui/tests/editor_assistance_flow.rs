use captee_platform::{LspPosition, LspRange, TinymistCompletion};
use captee_ui::editor_assistance::{lsp_position, tinymist_completion_edit};
use captee_ui::editor_bridge::EditorBridge;

#[test]
fn tinymist_completion_replaces_prefix_in_editor() {
    let source = "#im";
    let item = TinymistCompletion {
        label: "image".into(),
        insert_text: "image(\"img/\")".into(),
        range: Some(LspRange {
            start: LspPosition { line: 0, character: 1 },
            end: LspPosition { line: 0, character: 3 },
        }),
    };
    let edit = tinymist_completion_edit(source, source.len(), &item).expect("completion edit");
    let mut editor = EditorBridge::new("main.typ", source);
    editor.replace_range(edit.range, &edit.replacement).expect("replace prefix");
    assert_eq!(editor.state().text, "#image(\"img/\")");
}

#[test]
fn lsp_cursor_position_handles_unicode() {
    assert_eq!(lsp_position("😀#im", 7), Some(LspPosition { line: 0, character: 5 }));
}
