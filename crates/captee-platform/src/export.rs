//! Atomic export of the current successful preview.

use crate::{atomic_write, AtomicWriteError};
use captee_core::RenderState;
use std::fmt;
use std::path::{Path, PathBuf};

/// Writes the successful preview only when it belongs to the current source.
pub fn export_pdf(
    state: &RenderState,
    destination: impl AsRef<Path>,
) -> Result<(), PdfExportError> {
    let preview = state.last_successful_preview().ok_or(PdfExportError::NoSuccessfulPreview)?;
    if preview.revision != state.current_revision() {
        return Err(PdfExportError::StalePreview {
            preview_revision: preview.revision,
            current_revision: state.current_revision(),
        });
    }

    let destination = destination.as_ref();
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if !parent.is_dir() {
        return Err(PdfExportError::DestinationParentNotDirectory(parent.to_path_buf()));
    }
    if destination.exists() && !destination.is_file() {
        return Err(PdfExportError::DestinationNotFile(destination.to_path_buf()));
    }

    atomic_write(destination, &preview.pdf).map_err(PdfExportError::Atomic)
}

#[derive(Debug)]
pub enum PdfExportError {
    NoSuccessfulPreview,
    StalePreview { preview_revision: u64, current_revision: u64 },
    DestinationParentNotDirectory(PathBuf),
    DestinationNotFile(PathBuf),
    Atomic(AtomicWriteError),
}

impl fmt::Display for PdfExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSuccessfulPreview => {
                formatter.write_str("no successful preview is available for export")
            }
            Self::StalePreview { preview_revision, current_revision } => write!(
                formatter,
                "preview revision {preview_revision} is stale; current source revision is {current_revision}"
            ),
            Self::DestinationParentNotDirectory(path) => {
                write!(formatter, "export destination parent is not a directory: {}", path.display())
            }
            Self::DestinationNotFile(path) => {
                write!(formatter, "export destination is not a file: {}", path.display())
            }
            Self::Atomic(error) => write!(formatter, "PDF export failed: {error}"),
        }
    }
}

impl std::error::Error for PdfExportError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos();
        let root = std::env::temp_dir().join(format!("captee-export-{name}-{suffix}"));
        fs::create_dir_all(&root).expect("temporary root");
        root
    }

    #[test]
    fn exports_the_successful_preview_for_the_current_revision() {
        let root = test_root("success");
        let destination = root.join("notes.pdf");
        let mut state = RenderState::new(4);
        state.apply_success(4, b"rendered pdf".to_vec(), Vec::new(), UNIX_EPOCH);

        export_pdf(&state, &destination).expect("export");
        assert_eq!(fs::read(&destination).expect("read export"), b"rendered pdf");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn refuses_missing_or_stale_previews_without_changing_destination() {
        let root = test_root("stale");
        let destination = root.join("notes.pdf");
        fs::write(&destination, b"existing").expect("existing destination");

        let empty = RenderState::new(1);
        assert!(matches!(
            export_pdf(&empty, &destination),
            Err(PdfExportError::NoSuccessfulPreview)
        ));
        assert_eq!(fs::read(&destination).expect("read existing"), b"existing");

        let mut stale = RenderState::new(1);
        stale.apply_success(1, b"old render".to_vec(), Vec::new(), UNIX_EPOCH);
        stale.set_source_revision(2);
        assert!(matches!(
            export_pdf(&stale, &destination),
            Err(PdfExportError::StalePreview { .. })
        ));
        assert_eq!(fs::read(&destination).expect("read existing"), b"existing");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn failed_destination_validation_preserves_existing_directory() {
        let root = test_root("failure");
        let destination = root.join("notes.pdf");
        fs::create_dir(&destination).expect("destination directory");
        let mut state = RenderState::new(1);
        state.apply_success(1, b"pdf".to_vec(), Vec::new(), UNIX_EPOCH);

        assert!(matches!(
            export_pdf(&state, &destination),
            Err(PdfExportError::DestinationNotFile(_))
        ));
        assert!(destination.is_dir());
        fs::remove_dir_all(root).expect("cleanup");
    }
}
