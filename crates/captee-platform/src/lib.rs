//! Platform integration boundaries for Captee.
//!
//! Concrete filesystem, bundled Typst, portal, subprocess, and trash adapters
//! belong here. The initial scaffold keeps those integrations unimplemented so
//! the core crate remains headless and independently testable.

mod assets;
mod atomic;
mod authoring;
mod capture;
mod export;
mod paths;
mod persistence;
mod shortcuts;
mod tinymist;
mod typst;
mod workspace;

pub use assets::{insert_saved_asset, AssetError, AssetStore, SavedAsset};
pub use atomic::{atomic_write, AtomicWriteError, AutosaveSnapshot, AutosaveStore};
pub use authoring::{FormattedSource, TypstCompletionProvider, TypstFormatError, TypstFormatter};
pub use capture::{
    current_capture_origin, current_desktop_prefers_fallback_capture, place_capture_review_window,
    CaptureOrigin, CaptureSelector, GrimSlurpCapture, PngAnnotationBackend, XdgPortalCapture,
};
pub use export::{export_pdf, PdfExportError};
pub use paths::{PathError, ProjectPaths};
pub use persistence::{
    GlobalKeybindingError, GlobalKeybindingStore, PersistenceError, ProjectDocumentPersistence,
    RecentProjectError, RecentProjectStore, AUTOSAVE_FILE,
};
pub use shortcuts::{register_capture_shortcut, GlobalShortcutEvent, GlobalShortcutRegistration};
pub use tinymist::{
    capture_review_uri, document_uri, LspPosition, LspRange, TinymistCompletion,
    TinymistDiagnostic, TinymistDiagnosticSeverity, TinymistError, TinymistEvent, TinymistRunner,
    TinymistSession,
};
pub use typst::{
    AsyncPreviewCompiler, PreviewArtifact, PreviewCompiler, PreviewContentEnd, PreviewError,
    PreviewHandle, PreviewOutcome, PreviewWorkerError, TypstPreviewCompiler, TypstRunner,
};
pub use workspace::{
    confirm_and_trash, create_project, create_project_item, delete_project_item, list_project_tree,
    move_project_item, open_project, rename_project_item, save_project_settings, ProjectTreeEntry,
    ProjectWorkspace, TrashBackend, TrashError, TrashOutcome, WorkspaceError, CONFIG_FILE,
    IMAGE_DIRECTORY,
};

/// Identifies the role of this crate for architecture checks and diagnostics.
pub const CRATE_ROLE: &str = "platform-adapters";
