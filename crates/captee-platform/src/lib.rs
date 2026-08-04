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
mod typst;
mod workspace;

pub use assets::{insert_saved_asset, AssetError, AssetStore, SavedAsset};
pub use atomic::{atomic_write, AtomicWriteError, AutosaveSnapshot, AutosaveStore};
pub use authoring::{FormattedSource, TypstCompletionProvider, TypstFormatError, TypstFormatter};
pub use capture::{CaptureSelector, GrimSlurpCapture, PngAnnotationBackend, XdgPortalCapture};
pub use export::{export_pdf, PdfExportError};
pub use paths::{PathError, ProjectPaths};
pub use persistence::{
    PersistenceError, ProjectDocumentPersistence, RecentProjectError, RecentProjectStore,
    AUTOSAVE_FILE,
};
pub use typst::{
    AsyncPreviewCompiler, PreviewArtifact, PreviewCompiler, PreviewError, PreviewHandle,
    PreviewOutcome, PreviewWorkerError, TypstPreviewCompiler, TypstRunner,
};
pub use workspace::{
    confirm_and_trash, create_project, open_project, save_project_settings, ProjectWorkspace,
    TrashBackend, TrashError, TrashOutcome, WorkspaceError, CONFIG_FILE, IMAGE_DIRECTORY,
};

/// Identifies the role of this crate for architecture checks and diagnostics.
pub const CRATE_ROLE: &str = "platform-adapters";
