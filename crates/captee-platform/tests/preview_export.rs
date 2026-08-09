use captee_core::{Diagnostic, DiagnosticSeverity, RenderState, SourceSpan};
use captee_platform::{
    export_pdf, AsyncPreviewCompiler, PdfExportError, PreviewArtifact, PreviewCompiler,
    PreviewError,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const SUCCESS_SOURCE: &str = include_str!("fixtures/preview-success.typ");
const FAILURE_SOURCE: &str = include_str!("fixtures/preview-failure.typ");

struct FixtureCompiler;

impl PreviewCompiler for FixtureCompiler {
    fn compile_preview(&self, source: &str) -> Result<PreviewArtifact, PreviewError> {
        if source == SUCCESS_SOURCE {
            return Ok(PreviewArtifact {
                pdf: b"fixture-success-pdf".to_vec(),
                page_pngs: vec![b"fixture-success-png".to_vec()],
                content_end: None,
                diagnostics: Vec::new(),
            });
        }
        if source == FAILURE_SOURCE {
            return Err(PreviewError {
                message: "fixture compilation failed".to_owned(),
                diagnostics: vec![Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    message: "expected expression".to_owned(),
                    span: Some(SourceSpan {
                        path: "preview-failure.typ".to_owned(),
                        line: 3,
                        column: 15,
                    }),
                }],
            });
        }
        Err(PreviewError { message: "unknown fixture".to_owned(), diagnostics: Vec::new() })
    }
}

fn test_root(name: &str) -> PathBuf {
    let suffix = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos();
    let root = std::env::temp_dir().join(format!("captee-preview-fixture-{name}-{suffix}"));
    fs::create_dir_all(&root).expect("temporary root");
    root
}

#[test]
fn successful_fixture_preview_is_applied_to_current_state() {
    let worker = AsyncPreviewCompiler::new(FixtureCompiler);
    let outcome = worker.submit(5, SUCCESS_SOURCE).recv().expect("preview outcome");
    let mut state = RenderState::new(5);

    assert!(outcome.apply_to(&mut state));
    assert_eq!(state.last_successful_preview().expect("preview").pdf, b"fixture-success-pdf");
}

#[test]
fn failed_fixture_render_keeps_the_last_successful_preview() {
    let worker = AsyncPreviewCompiler::new(FixtureCompiler);
    let successful = worker.submit(1, SUCCESS_SOURCE).recv().expect("success outcome");
    let failed = worker.submit(1, FAILURE_SOURCE).recv().expect("failure outcome");
    let mut state = RenderState::new(1);

    assert!(successful.apply_to(&mut state));
    assert!(failed.apply_to(&mut state));
    assert_eq!(state.last_successful_preview().expect("preview").pdf, b"fixture-success-pdf");
    assert_eq!(state.diagnostics()[0].message, "expected expression");
}

#[test]
fn stale_fixture_render_is_rejected() {
    let worker = AsyncPreviewCompiler::new(FixtureCompiler);
    let outcome = worker.submit(1, SUCCESS_SOURCE).recv().expect("preview outcome");
    let mut state = RenderState::new(2);

    assert!(!outcome.apply_to(&mut state));
    assert!(state.last_successful_preview().is_none());
}

#[test]
fn export_refuses_a_fixture_render_for_a_newer_revision() {
    let root = test_root("export");
    let destination = root.join("notes.pdf");
    fs::write(&destination, b"previous export").expect("existing destination");
    let worker = AsyncPreviewCompiler::new(FixtureCompiler);
    let outcome = worker.submit(1, SUCCESS_SOURCE).recv().expect("preview outcome");
    let mut state = RenderState::new(1);
    assert!(outcome.apply_to(&mut state));
    state.set_source_revision(2);

    assert!(matches!(export_pdf(&state, &destination), Err(PdfExportError::StalePreview { .. })));
    assert_eq!(fs::read(&destination).expect("destination"), b"previous export");
    fs::remove_dir_all(root).expect("cleanup");
}
