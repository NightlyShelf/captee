//! Platform-independent Captee domain logic.
//!
//! This crate intentionally contains no GTK, Linux, process, or real-filesystem
//! dependencies. Platform adapters and the desktop shell depend on it through
//! narrow data types and traits.

/// Identifies the role of this crate for architecture checks and diagnostics.
pub const CRATE_ROLE: &str = "domain-core";

mod project;
mod editor;
mod diagnostics;

pub use project::{
    CaptureSettings, ConfigError, FormattingSettings, PreviewSettings, ProjectConfig,
    ProjectSettings, RecentProjects, CONFIG_VERSION,
};
pub use editor::{DocumentPersistence, EditError, SourceDocument};
pub use diagnostics::{parse_diagnostics, Diagnostic, DiagnosticSeverity, SourceSpan};
