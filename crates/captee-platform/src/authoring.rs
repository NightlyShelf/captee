use crate::{atomic_write, TypstRunner};
use captee_core::{parse_diagnostics, CompletionItem, CompletionProvider, Diagnostic, Formatter};
use std::convert::Infallible;
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::process::Output;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_FORMAT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct TypstFormatter {
    runner: TypstRunner,
    project_root: PathBuf,
}

impl TypstFormatter {
    pub fn new(runner: TypstRunner, project_root: impl Into<PathBuf>) -> Self {
        Self { runner, project_root: project_root.into() }
    }

    pub fn format_with_diagnostics(
        &self,
        source: &str,
    ) -> Result<FormattedSource, TypstFormatError> {
        let id = NEXT_FORMAT_ID.fetch_add(1, Ordering::Relaxed);
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let path = self
            .project_root
            .join(format!(".captee-format-{}-{stamp}-{id}.typ", std::process::id()));
        let result = (|| {
            atomic_write(&path, source.as_bytes()).map_err(|error| TypstFormatError {
                message: format!("could not stage source for formatting: {error}"),
                diagnostics: Vec::new(),
            })?;
            let output = self.runner.format(&path).map_err(|error| TypstFormatError {
                message: format!("could not run Typst formatter: {error}"),
                diagnostics: Vec::new(),
            })?;
            let diagnostics = diagnostics_from_output(&output);
            if !output.status.success() {
                return Err(TypstFormatError {
                    message: process_failure_message(&output),
                    diagnostics,
                });
            }
            let source = fs::read_to_string(&path).map_err(|error| TypstFormatError {
                message: format!("could not read formatted source: {error}"),
                diagnostics: diagnostics.clone(),
            })?;
            Ok(FormattedSource { source, diagnostics })
        })();
        let _ = fs::remove_file(path);
        result
    }
}

impl Formatter for TypstFormatter {
    type Error = TypstFormatError;

    fn format(&self, source: &str) -> Result<String, Self::Error> {
        self.format_with_diagnostics(source).map(|formatted| formatted.source)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormattedSource {
    pub source: String,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypstFormatError {
    pub message: String,
    pub diagnostics: Vec<Diagnostic>,
}

impl fmt::Display for TypstFormatError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TypstFormatError {}

#[derive(Debug, Clone, Copy, Default)]
pub struct TypstCompletionProvider;

impl CompletionProvider for TypstCompletionProvider {
    type Error = Infallible;

    fn complete(&self, source: &str, cursor: usize) -> Result<Vec<CompletionItem>, Self::Error> {
        if cursor > source.len() || !source.is_char_boundary(cursor) {
            return Ok(Vec::new());
        }
        let prefix = source[..cursor]
            .split(|character: char| {
                !(character.is_alphanumeric() || character == '#' || character == '-')
            })
            .next_back()
            .unwrap_or_default();
        let items = [
            ("#figure", "#figure(\n  ,\n  caption: [],\n)"),
            ("#heading", "#heading[]"),
            ("#image", "#image(\"img/\")"),
            ("#let", "#let name = "),
            ("#set", "#set "),
            ("#show", "#show: "),
            ("#table", "#table(\n  columns: (),\n)"),
            ("#text", "#text[]"),
        ];
        Ok(items
            .into_iter()
            .filter(|(label, _)| prefix.is_empty() || label.starts_with(prefix))
            .map(|(label, insert_text)| CompletionItem {
                label: label.to_owned(),
                insert_text: insert_text.to_owned(),
            })
            .collect())
    }
}

fn diagnostics_from_output(output: &Output) -> Vec<Diagnostic> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    parse_diagnostics(&format!("{stdout}\n{stderr}"))
}

fn process_failure_message(output: &Output) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn completion_filters_known_typst_constructs_by_prefix() {
        let completions = TypstCompletionProvider.complete("#im", 3).expect("completion");
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].label, "#image");
        assert!(TypstCompletionProvider.complete("#im", 99).expect("boundary").is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn formatter_stages_reads_and_cleans_a_source_snapshot() {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos();
        let root = std::env::temp_dir().join(format!("captee-formatter-{stamp}"));
        fs::create_dir_all(&root).expect("temporary root");
        let executable = root.join("fake-typst");
        fs::write(
            &executable,
            "#!/bin/sh\nprintf '= formatted\\n' > \"$2\"\nprintf 'warning: main.typ:1:1: adjusted\\n' >&2\n",
        )
        .expect("fake formatter");
        let mut permissions = fs::metadata(&executable).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).expect("executable permissions");

        let formatter = TypstFormatter::new(TypstRunner::new(&executable), &root);
        let formatted = formatter.format_with_diagnostics("=unformatted").expect("format");

        assert_eq!(formatted.source, "= formatted\n");
        assert_eq!(formatted.diagnostics.len(), 1);
        assert_eq!(fs::read_dir(&root).expect("read root").count(), 1);
        fs::remove_dir_all(root).expect("cleanup");
    }
}
