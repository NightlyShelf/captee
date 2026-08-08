use captee_core::{
    request_completions, CancellationToken, CompletionItem, Operation as CoreOperation,
    OperationKind,
};
use captee_platform::TypstCompletionProvider;
use captee_ui::editor_assistance::{completion_edit, typst_completions};
use captee_ui::editor_bridge::EditorBridge;
use captee_ui::operation::{OperationCoordinator, OperationOutcome, ResultDisposition};

#[test]
fn both_typst_editors_share_prefix_filtering_and_cursor_replacement() {
    let source = "#im";
    let items = typst_completions(source, source.len());
    assert_eq!(items.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(), vec!["#image"]);

    let edit = completion_edit(source, source.len(), &items[0]).expect("completion edit");
    let mut editor = EditorBridge::new("main.typ", source);
    editor.replace_range(edit.range, &edit.replacement).expect("replace prefix");
    assert_eq!(editor.state().text, "#image(\"img/\")");
}

#[test]
fn dismissed_completion_preserves_source_and_cancelled_request_returns_no_items() {
    let source = "text";
    let before = source.to_owned();
    let cancellation = CancellationToken::default();
    cancellation.cancel();
    assert_eq!(
        request_completions(&TypstCompletionProvider, source, source.len(), &cancellation)
            .expect("cancelled request"),
        CoreOperation::Cancelled
    );
    assert_eq!(source, before);
}

#[test]
fn stale_completion_result_cannot_apply_after_source_revision_changes() {
    let mut coordinator = OperationCoordinator::new();
    coordinator.activate_project("/tmp/editor-assistance").expect("project");
    let task = coordinator.begin(OperationKind::Completion, true).expect("completion");
    coordinator.set_source_revision(1).expect("new revision");
    task.finish(OperationOutcome::Completed(vec![CompletionItem {
        label: "#image".into(),
        insert_text: "#image(\"img/\")".into(),
    }]))
    .expect("late completion");
    assert!(matches!(coordinator.try_next_result(), Some(ResultDisposition::Stale(_))));
}
