//! Platform integration boundaries for Captee.
//!
//! Concrete filesystem, bundled Typst, portal, subprocess, and trash adapters
//! belong here. The initial scaffold keeps those integrations unimplemented so
//! the core crate remains headless and independently testable.

mod atomic;
mod paths;
mod typst;
mod workspace;

pub use atomic::{atomic_write, AtomicWriteError, AutosaveSnapshot, AutosaveStore};
pub use paths::{PathError, ProjectPaths};
pub use typst::TypstRunner;
pub use workspace::{
    confirm_and_trash, create_project, open_project, ProjectWorkspace, TrashBackend, TrashError,
    TrashOutcome, WorkspaceError, CONFIG_FILE, IMAGE_DIRECTORY,
};

/// Identifies the role of this crate for architecture checks and diagnostics.
pub const CRATE_ROLE: &str = "platform-adapters";
