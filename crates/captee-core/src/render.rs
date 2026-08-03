//! Revision-aware preview state and render results.

use crate::Diagnostic;
use std::time::SystemTime;

/// A successfully rendered entry document retained for preview and export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedPreview {
    pub revision: u64,
    pub pdf: Vec<u8>,
    pub rendered_at: SystemTime,
}

/// State presented by the preview pane for the active source document.
///
/// Render results are accepted only for the current source revision. A newer
/// source revision clears diagnostics from the older attempt but deliberately
/// keeps the last successful preview available until it is replaced.
#[derive(Debug, Clone, Default)]
pub struct RenderState {
    current_revision: u64,
    last_successful_preview: Option<RenderedPreview>,
    diagnostics: Vec<Diagnostic>,
    last_render_at: Option<SystemTime>,
}

impl RenderState {
    pub fn new(current_revision: u64) -> Self {
        Self { current_revision, ..Self::default() }
    }

    pub fn current_revision(&self) -> u64 {
        self.current_revision
    }

    pub fn last_successful_preview(&self) -> Option<&RenderedPreview> {
        self.last_successful_preview.as_ref()
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Returns the completion time of the most recent accepted render attempt
    /// for the current source revision.
    pub fn last_render_at(&self) -> Option<SystemTime> {
        self.last_render_at
    }

    /// Moves the state to a newer source revision without discarding a valid preview.
    pub fn set_source_revision(&mut self, revision: u64) {
        if revision <= self.current_revision {
            return;
        }
        self.current_revision = revision;
        self.diagnostics.clear();
        self.last_render_at = None;
    }

    /// Applies a successful render when it belongs to the current source revision.
    ///
    /// Returns `false` for stale results and leaves all state unchanged.
    pub fn apply_success(
        &mut self,
        revision: u64,
        pdf: Vec<u8>,
        diagnostics: Vec<Diagnostic>,
        rendered_at: SystemTime,
    ) -> bool {
        if revision != self.current_revision {
            return false;
        }
        self.last_successful_preview = Some(RenderedPreview { revision, pdf, rendered_at });
        self.diagnostics = diagnostics;
        self.last_render_at = Some(rendered_at);
        true
    }

    /// Applies a failed render when it belongs to the current source revision.
    ///
    /// The failed attempt's diagnostics are shown, while the last successful
    /// preview remains available. Returns `false` for stale results.
    pub fn apply_failure(
        &mut self,
        revision: u64,
        diagnostics: Vec<Diagnostic>,
        rendered_at: SystemTime,
    ) -> bool {
        if revision != self.current_revision {
            return false;
        }
        self.diagnostics = diagnostics;
        self.last_render_at = Some(rendered_at);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DiagnosticSeverity, SourceSpan};
    use std::time::{Duration, UNIX_EPOCH};

    fn warning() -> Diagnostic {
        Diagnostic {
            severity: DiagnosticSeverity::Warning,
            message: "deprecated syntax".to_owned(),
            span: Some(SourceSpan { path: "main.typ".to_owned(), line: 2, column: 4 }),
        }
    }

    #[test]
    fn successful_render_records_preview_diagnostics_and_timestamp() {
        let rendered_at = UNIX_EPOCH + Duration::from_secs(10);
        let mut state = RenderState::new(3);
        assert!(state.apply_success(3, b"pdf".to_vec(), vec![warning()], rendered_at));

        let preview = state.last_successful_preview().expect("preview");
        assert_eq!(preview.revision, 3);
        assert_eq!(preview.pdf, b"pdf");
        assert_eq!(preview.rendered_at, rendered_at);
        assert_eq!(state.diagnostics(), &[warning()]);
        assert_eq!(state.last_render_at(), Some(rendered_at));
    }

    #[test]
    fn failed_render_keeps_last_successful_preview_and_updates_diagnostics() {
        let first_render = UNIX_EPOCH + Duration::from_secs(10);
        let failed_render = UNIX_EPOCH + Duration::from_secs(20);
        let mut state = RenderState::new(1);
        state.apply_success(1, b"good pdf".to_vec(), Vec::new(), first_render);
        assert!(state.apply_failure(1, vec![warning()], failed_render));

        let preview = state.last_successful_preview().expect("last preview");
        assert_eq!(preview.pdf, b"good pdf");
        assert_eq!(preview.rendered_at, first_render);
        assert_eq!(state.diagnostics(), &[warning()]);
        assert_eq!(state.last_render_at(), Some(failed_render));
    }

    #[test]
    fn newer_source_revision_clears_old_diagnostics_but_retains_preview() {
        let rendered_at = UNIX_EPOCH + Duration::from_secs(10);
        let mut state = RenderState::new(1);
        state.apply_success(1, b"pdf".to_vec(), vec![warning()], rendered_at);
        state.set_source_revision(2);

        assert_eq!(state.current_revision(), 2);
        assert!(state.diagnostics().is_empty());
        assert_eq!(state.last_render_at(), None);
        assert_eq!(state.last_successful_preview().expect("preview").revision, 1);
    }

    #[test]
    fn stale_results_cannot_replace_preview_or_diagnostics() {
        let rendered_at = UNIX_EPOCH + Duration::from_secs(10);
        let mut state = RenderState::new(2);
        state.apply_success(2, b"current".to_vec(), Vec::new(), rendered_at);
        assert!(!state.apply_success(1, b"stale".to_vec(), vec![warning()], rendered_at));
        assert!(!state.apply_failure(1, vec![warning()], rendered_at));

        let preview = state.last_successful_preview().expect("preview");
        assert_eq!(preview.pdf, b"current");
        assert!(state.diagnostics().is_empty());
        assert_eq!(state.last_render_at(), Some(rendered_at));
    }
}
