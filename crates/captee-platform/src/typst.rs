//! Adapter for the bundled Typst command-line compiler.

use crate::atomic_write;
use captee_core::{parse_diagnostics, Diagnostic, DiagnosticSeverity, RenderState};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::SystemTime;

static NEXT_PREVIEW_ID: AtomicU64 = AtomicU64::new(0);

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

    /// Locates the packaged compiler, a development bundle, or a PATH fallback.
    pub fn discover() -> Self {
        if let Some(executable) = std::env::var_os("CAPTEE_TYPST_BINARY") {
            return Self::new(executable);
        }
        if let Ok(current_executable) = std::env::current_exe() {
            if let Some(directory) = current_executable.parent() {
                for candidate in [
                    directory.join("typst"),
                    directory.join("../share/captee/typst/typst"),
                    directory.join("../lib/captee/typst"),
                ] {
                    if candidate.is_file() {
                        return Self::new(candidate);
                    }
                }
            }
        }
        let development = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../dist/typst/typst");
        if development.is_file() {
            return Self::new(development);
        }
        Self::new("typst")
    }

    /// Returns the compiler's version output.
    pub fn version(&self) -> io::Result<Output> {
        self.run(["--version".to_owned()])
    }

    /// Compiles a Typst source file to a PDF destination.
    pub fn compile(&self, source: &Path, output: &Path) -> io::Result<Output> {
        self.run(["compile".to_owned(), path_arg(source), path_arg(output)])
    }

    pub fn compile_png_pages(&self, source: &Path, output_template: &Path) -> io::Result<Output> {
        self.run([
            "compile".to_owned(),
            "--format".to_owned(),
            "png".to_owned(),
            path_arg(source),
            path_arg(output_template),
        ])
    }

    /// Formats a Typst source file in place.
    pub fn format(&self, source: &Path) -> io::Result<Output> {
        self.run(["fmt".to_owned(), path_arg(source)])
    }

    fn run<const N: usize>(&self, args: [String; N]) -> io::Result<Output> {
        Command::new(&self.executable).args(args).output().map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "Typst compiler not found at '{}'; run tools/fetch-typst.sh or set CAPTEE_TYPST_BINARY",
                        self.executable.display()
                    ),
                )
            } else {
                error
            }
        })
    }
}

/// The successful output of a preview compilation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewArtifact {
    pub pdf: Vec<u8>,
    pub page_pngs: Vec<Vec<u8>>,
    pub diagnostics: Vec<Diagnostic>,
}

/// A compiler failure that can be displayed without exposing process details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewError {
    pub message: String,
    pub diagnostics: Vec<Diagnostic>,
}

/// Boundary used by the asynchronous preview worker and its test doubles.
pub trait PreviewCompiler: Send + Sync + 'static {
    fn compile_preview(&self, source: &str) -> Result<PreviewArtifact, PreviewError>;
}

/// Runs the bundled Typst executable against an in-memory source snapshot.
#[derive(Debug, Clone)]
pub struct TypstPreviewCompiler {
    runner: TypstRunner,
    project_root: PathBuf,
}

impl TypstPreviewCompiler {
    pub fn new(runner: TypstRunner, project_root: impl Into<PathBuf>) -> Self {
        Self { runner, project_root: project_root.into() }
    }
}

impl PreviewCompiler for TypstPreviewCompiler {
    fn compile_preview(&self, source: &str) -> Result<PreviewArtifact, PreviewError> {
        let id = NEXT_PREVIEW_ID.fetch_add(1, Ordering::Relaxed);
        let stamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let prefix = format!(".captee-preview-{}-{stamp}-{id}", std::process::id());
        let source_path = self.project_root.join(format!("{prefix}.typ"));
        let output_path = self.project_root.join(format!("{prefix}.pdf"));
        let png_template = self.project_root.join(format!("{prefix}-page-{{p}}-of-{{t}}.png"));

        let result = (|| {
            atomic_write(&source_path, source.as_bytes()).map_err(|error| PreviewError {
                message: format!("could not stage preview source: {error}"),
                diagnostics: Vec::new(),
            })?;
            let output =
                self.runner.compile(&source_path, &output_path).map_err(|error| PreviewError {
                    message: format!("could not run Typst preview compiler: {error}"),
                    diagnostics: Vec::new(),
                })?;
            let diagnostics = diagnostics_from_output(&output);
            if !output.status.success() {
                return Err(PreviewError {
                    message: compiler_failure_message(&output),
                    diagnostics,
                });
            }
            let pdf = std::fs::read(&output_path).map_err(|error| PreviewError {
                message: format!("could not read rendered preview: {error}"),
                diagnostics: diagnostics.clone(),
            })?;
            let png_output =
                self.runner.compile_png_pages(&source_path, &png_template).map_err(|error| {
                    PreviewError {
                        message: format!("could not run Typst preview image compiler: {error}"),
                        diagnostics: diagnostics.clone(),
                    }
                })?;
            let mut image_diagnostics = diagnostics_from_output(&png_output);
            if !png_output.status.success() {
                return Err(PreviewError {
                    message: compiler_failure_message(&png_output),
                    diagnostics: image_diagnostics,
                });
            }
            let page_paths = preview_page_paths(&self.project_root, &prefix)?;
            let mut page_pngs = Vec::with_capacity(page_paths.len());
            for page_path in page_paths {
                page_pngs.push(std::fs::read(&page_path).map_err(|error| PreviewError {
                    message: format!("could not read rendered preview image: {error}"),
                    diagnostics: image_diagnostics.clone(),
                })?);
            }
            let mut diagnostics = diagnostics;
            diagnostics.append(&mut image_diagnostics);
            Ok(PreviewArtifact { pdf, page_pngs, diagnostics })
        })();

        let _ = std::fs::remove_file(&source_path);
        let _ = std::fs::remove_file(&output_path);
        remove_preview_pages(&self.project_root, &prefix);
        result
    }
}

fn preview_page_paths(root: &Path, prefix: &str) -> Result<Vec<PathBuf>, PreviewError> {
    let mut pages = BTreeMap::new();
    let entries = std::fs::read_dir(root).map_err(|error| PreviewError {
        message: format!("could not enumerate rendered preview images: {error}"),
        diagnostics: Vec::new(),
    })?;
    let marker = format!("{prefix}-page-");
    for entry in entries {
        let path = entry
            .map_err(|error| PreviewError {
                message: format!("could not enumerate rendered preview images: {error}"),
                diagnostics: Vec::new(),
            })?
            .path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(page) = name
            .strip_prefix(&marker)
            .and_then(|page| page.split('-').next())
            .and_then(|page| page.parse::<usize>().ok())
        else {
            continue;
        };
        pages.insert(page, path);
    }
    if pages.is_empty() {
        return Err(PreviewError {
            message: "Typst produced no preview pages".to_owned(),
            diagnostics: Vec::new(),
        });
    }
    Ok(pages.into_values().collect())
}

fn remove_preview_pages(root: &Path, prefix: &str) {
    let marker = format!("{prefix}-page-");
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_preview_page = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(&marker) && name.ends_with(".png"));
        if is_preview_page {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// A revision-tagged result returned by an asynchronous preview compilation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewOutcome {
    pub revision: u64,
    pub result: Result<PreviewArtifact, PreviewError>,
    pub rendered_at: SystemTime,
}

impl PreviewOutcome {
    /// Applies this result only when it belongs to the current source revision.
    pub fn apply_to(self, state: &mut RenderState) -> bool {
        self.apply_to_with_pages(state).0
    }

    /// Applies this result and transfers page images only when it is current.
    /// Keeping the page buffers in the outcome avoids cloning every page while
    /// the core render state records only the PDF and diagnostics.
    pub fn apply_to_with_pages(self, state: &mut RenderState) -> (bool, Option<Vec<Vec<u8>>>) {
        match self.result {
            Ok(artifact) => {
                let page_pngs = artifact.page_pngs;
                let accepted = state.apply_success(
                    self.revision,
                    artifact.pdf,
                    artifact.diagnostics,
                    self.rendered_at,
                );
                (accepted, accepted.then_some(page_pngs))
            }
            Err(error) => {
                let diagnostics = if error.diagnostics.is_empty() {
                    vec![Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        message: error.message,
                        span: None,
                    }]
                } else {
                    error.diagnostics
                };
                (state.apply_failure(self.revision, diagnostics, self.rendered_at), None)
            }
        }
    }
}

/// Handle for a preview compilation running outside the caller's thread.
#[derive(Debug)]
pub struct PreviewHandle {
    receiver: mpsc::Receiver<PreviewOutcome>,
}

impl PreviewHandle {
    pub fn recv(self) -> Result<PreviewOutcome, PreviewWorkerError> {
        self.receiver.recv().map_err(|_| PreviewWorkerError::Disconnected)
    }

    pub fn try_recv(&self) -> Result<Option<PreviewOutcome>, PreviewWorkerError> {
        match self.receiver.try_recv() {
            Ok(outcome) => Ok(Some(outcome)),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => Err(PreviewWorkerError::Disconnected),
        }
    }
}

/// Starts independent preview jobs using a shared compiler adapter.
#[derive(Debug, Clone)]
pub struct AsyncPreviewCompiler<C> {
    compiler: Arc<C>,
}

impl<C: PreviewCompiler> AsyncPreviewCompiler<C> {
    pub fn new(compiler: C) -> Self {
        Self { compiler: Arc::new(compiler) }
    }

    pub fn submit(&self, revision: u64, source: impl Into<String>) -> PreviewHandle {
        let compiler = Arc::clone(&self.compiler);
        let source = source.into();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = compiler.compile_preview(&source);
            let _ =
                sender.send(PreviewOutcome { revision, result, rendered_at: SystemTime::now() });
        });
        PreviewHandle { receiver }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewWorkerError {
    Disconnected,
}

impl std::fmt::Display for PreviewWorkerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disconnected => formatter.write_str("preview worker disconnected"),
        }
    }
}

impl std::error::Error for PreviewWorkerError {}

fn diagnostics_from_output(output: &Output) -> Vec<Diagnostic> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    parse_diagnostics(&format!("{stdout}\n{stderr}"))
}

fn compiler_failure_message(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if !stderr.is_empty() {
        return stderr;
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !stdout.is_empty() {
        return stdout;
    }
    format!("Typst exited with status {}", output.status)
}

fn path_arg(path: &Path) -> String {
    path.as_os_str().to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use captee_core::DiagnosticSeverity;
    use std::sync::Mutex;

    #[derive(Debug, Default)]
    struct FakeCompiler {
        sources: Mutex<Vec<String>>,
    }

    impl PreviewCompiler for FakeCompiler {
        fn compile_preview(&self, source: &str) -> Result<PreviewArtifact, PreviewError> {
            self.sources.lock().expect("sources lock").push(source.to_owned());
            Ok(PreviewArtifact {
                pdf: source.as_bytes().to_vec(),
                page_pngs: vec![b"png".to_vec()],
                diagnostics: Vec::new(),
            })
        }
    }

    #[test]
    fn async_preview_compiles_a_source_snapshot_off_thread() {
        let compiler = FakeCompiler::default();
        let worker = AsyncPreviewCompiler::new(compiler);
        let outcome = worker.submit(7, "#let answer = 42").recv().expect("preview outcome");

        assert_eq!(outcome.revision, 7);
        let artifact = outcome.result.expect("successful preview");
        assert_eq!(artifact.pdf, b"#let answer = 42");
        assert_eq!(artifact.page_pngs, vec![b"png".to_vec()]);
    }

    #[test]
    fn missing_compiler_reports_setup_instructions() {
        let runner = TypstRunner::new("/captee/test/missing-typst");
        let error = runner.version().expect_err("compiler should be missing");
        assert!(error.to_string().contains("tools/fetch-typst.sh"));
        assert!(error.to_string().contains("CAPTEE_TYPST_BINARY"));
    }

    #[test]
    fn outcome_applies_success_only_to_the_current_revision() {
        let worker = AsyncPreviewCompiler::new(FakeCompiler::default());
        let outcome = worker.submit(1, "stale").recv().expect("preview outcome");
        let mut state = RenderState::new(2);

        assert!(!outcome.apply_to(&mut state));
        assert!(state.last_successful_preview().is_none());
    }

    #[test]
    fn failed_outcome_preserves_the_previous_preview() {
        let outcome = PreviewOutcome {
            revision: 4,
            result: Err(PreviewError {
                message: "syntax error".to_owned(),
                diagnostics: vec![Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    message: "broken".to_owned(),
                    span: None,
                }],
            }),
            rendered_at: SystemTime::UNIX_EPOCH,
        };
        let mut state = RenderState::new(4);
        state.apply_success(4, b"previous".to_vec(), Vec::new(), SystemTime::UNIX_EPOCH);

        assert!(outcome.apply_to(&mut state));
        assert_eq!(state.last_successful_preview().expect("preview").pdf, b"previous");
        assert_eq!(state.diagnostics()[0].severity, DiagnosticSeverity::Error);
    }

    #[test]
    fn preview_page_paths_are_returned_in_document_order() {
        let root = std::env::temp_dir().join(format!(
            "captee-preview-pages-{}",
            NEXT_PREVIEW_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).expect("create preview test directory");
        std::fs::write(root.join(".preview-page-10-of-10.png"), b"10").expect("page 10");
        std::fs::write(root.join(".preview-page-2-of-10.png"), b"2").expect("page 2");
        std::fs::write(root.join(".preview-page-1-of-10.png"), b"1").expect("page 1");

        let paths = preview_page_paths(&root, ".preview").expect("preview pages");
        let names = paths
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                ".preview-page-1-of-10.png",
                ".preview-page-2-of-10.png",
                ".preview-page-10-of-10.png"
            ]
        );
        remove_preview_pages(&root, ".preview");
        std::fs::remove_dir(&root).expect("remove preview test directory");
    }
}
