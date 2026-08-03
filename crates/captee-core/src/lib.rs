//! Platform-independent Captee domain logic.
//!
//! This crate intentionally contains no GTK, Linux, process, or real-filesystem
//! dependencies. Platform adapters and the desktop shell depend on it through
//! narrow data types and traits.

/// Identifies the role of this crate for architecture checks and diagnostics.
pub const CRATE_ROLE: &str = "domain-core";

mod authoring;
mod diagnostics;
mod editor;
mod project;
mod render;
mod revision;

pub use authoring::{
    find_literal, replace_literal, request_completions, AuthoringError, CancellationToken,
    CompletionItem, CompletionProvider, Formatter, Operation, ReplaceError, ReplaceResult,
};
pub use diagnostics::{parse_diagnostics, Diagnostic, DiagnosticSeverity, SourceSpan};
pub use editor::{DocumentPersistence, EditError, SourceDocument};
pub use project::{
    CaptureSettings, ConfigError, FormattingSettings, PreviewSettings, ProjectConfig,
    ProjectSettings, RecentProjects, CONFIG_VERSION,
};
pub use render::{RenderState, RenderedPreview};
pub use revision::{DebouncedScheduler, PendingWork, WorkKind};
