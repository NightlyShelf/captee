use captee_core::{Diagnostic, DiagnosticSeverity, OperationKind, RenderState};
use captee_platform::{export_pdf, PreviewArtifact, PreviewError, PreviewOutcome};
use captee_ui::operation::{OperationCoordinator, OperationOutcome, ResultDisposition};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn test_root(name: &str) -> PathBuf {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos();
    let root = std::env::temp_dir().join(format!("captee-ui-preview-{name}-{stamp}"));
    fs::create_dir_all(&root).expect("temporary root");
    root
}

fn coordinator() -> OperationCoordinator<PreviewOutcome> {
    let mut coordinator = OperationCoordinator::new();
    coordinator.activate_project("/tmp/notes").expect("project");
    coordinator
}

fn success(revision: u64, pdf: &[u8]) -> PreviewOutcome {
    PreviewOutcome {
        revision,
        result: Ok(PreviewArtifact {
            pdf: pdf.to_vec(),
            first_page_png: b"png".to_vec(),
            diagnostics: Vec::new(),
        }),
        rendered_at: UNIX_EPOCH,
    }
}

#[test]
fn current_success_flows_from_ui_result_channel_to_atomic_export() {
    let root = test_root("success");
    let destination = root.join("notes.pdf");
    let mut coordinator = coordinator();
    let task = coordinator.begin(OperationKind::Preview, true).expect("preview");
    task.finish(OperationOutcome::Completed(success(0, b"current pdf"))).expect("result");
    let Some(ResultDisposition::Current(result)) = coordinator.try_next_result() else {
        panic!("current preview result");
    };
    let OperationOutcome::Completed(outcome) = result.outcome else {
        panic!("successful preview outcome");
    };
    let mut state = RenderState::new(0);
    assert!(outcome.apply_to(&mut state));

    export_pdf(&state, &destination).expect("export");

    assert_eq!(fs::read(&destination).expect("read export"), b"current pdf");
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn failed_current_preview_retains_last_valid_export() {
    let mut state = RenderState::new(1);
    state.apply_success(1, b"last valid".to_vec(), Vec::new(), UNIX_EPOCH);
    let failure = PreviewOutcome {
        revision: 1,
        result: Err(PreviewError {
            message: "compile failed".to_owned(),
            diagnostics: vec![Diagnostic {
                severity: DiagnosticSeverity::Error,
                message: "broken source".to_owned(),
                span: None,
            }],
        }),
        rendered_at: UNIX_EPOCH,
    };

    assert!(failure.apply_to(&mut state));

    assert_eq!(state.last_successful_preview().expect("retained").pdf, b"last valid");
    assert_eq!(state.diagnostics()[0].message, "broken source");
}

#[test]
fn cancelled_or_stale_preview_cannot_replace_render_state() {
    let mut coordinator = coordinator();
    let task = coordinator.begin(OperationKind::Preview, true).expect("preview");
    coordinator.cancel_active().expect("cancel");
    task.finish(OperationOutcome::Completed(success(0, b"cancelled"))).expect("late result");
    assert!(matches!(coordinator.try_next_result(), Some(ResultDisposition::Stale(_))));

    let task = coordinator.begin(OperationKind::Preview, true).expect("next preview");
    coordinator.set_source_revision(2).expect("new source");
    task.finish(OperationOutcome::Completed(success(0, b"stale"))).expect("stale result");
    assert!(matches!(coordinator.try_next_result(), Some(ResultDisposition::Stale(_))));

    let state = RenderState::new(2);
    assert!(state.last_successful_preview().is_none());
}

#[test]
fn cancelled_destination_selection_performs_no_export() {
    fn export_selection(
        state: &RenderState,
        destination: Option<&Path>,
    ) -> Result<bool, captee_platform::PdfExportError> {
        let Some(destination) = destination else {
            return Ok(false);
        };
        export_pdf(state, destination)?;
        Ok(true)
    }

    let root = test_root("cancelled-export");
    let destination = root.join("notes.pdf");
    fs::write(&destination, b"existing").expect("existing");
    let mut state = RenderState::new(0);
    state.apply_success(0, b"new".to_vec(), Vec::new(), UNIX_EPOCH);

    assert!(!export_selection(&state, None).expect("cancel"));
    assert_eq!(fs::read(&destination).expect("unchanged"), b"existing");
    fs::remove_dir_all(root).expect("cleanup");
}
