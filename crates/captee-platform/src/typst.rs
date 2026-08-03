//! Adapter for the bundled Typst command-line compiler.

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Runs a pinned Typst executable without exposing process details to the UI.
#[derive(Debug, Clone)]
pub struct TypstRunner {
    executable: PathBuf,
}

impl TypstRunner {
    /// Creates a runner for an already-installed bundled executable.
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self { executable: executable.into() }
    }

    /// Returns the compiler's version output.
    pub fn version(&self) -> io::Result<Output> {
        self.run(["--version".to_owned()])
    }

    /// Compiles a Typst source file to a PDF destination.
    pub fn compile(&self, source: &Path, output: &Path) -> io::Result<Output> {
        self.run(["compile".to_owned(), path_arg(source), path_arg(output)])
    }

    /// Formats a Typst source file in place.
    pub fn format(&self, source: &Path) -> io::Result<Output> {
        self.run(["fmt".to_owned(), path_arg(source)])
    }

    fn run<const N: usize>(&self, args: [String; N]) -> io::Result<Output> {
        Command::new(&self.executable).args(args).output()
    }
}

fn path_arg(path: &Path) -> String {
    path.as_os_str().to_string_lossy().into_owned()
}
