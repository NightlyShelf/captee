//! Platform integration boundaries for Captee.
//!
//! Concrete filesystem, bundled Typst, portal, subprocess, and trash adapters
//! belong here. The initial scaffold keeps those integrations unimplemented so
//! the core crate remains headless and independently testable.

mod typst;
mod paths;

pub use paths::{PathError, ProjectPaths};
pub use typst::TypstRunner;

/// Identifies the role of this crate for architecture checks and diagnostics.
pub const CRATE_ROLE: &str = "platform-adapters";
