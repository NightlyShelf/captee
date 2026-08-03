//! Project creation/opening and platform trash boundaries.

use crate::{atomic_write, PathError, ProjectPaths};
use captee_core::ProjectConfig;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub const CONFIG_FILE: &str = ".captee.json";
pub const IMAGE_DIRECTORY: &str = "img";

#[derive(Debug, Clone)]
pub struct ProjectWorkspace {
    pub root: PathBuf,
    pub config: ProjectConfig,
    pub paths: ProjectPaths,
}

pub fn create_project(
    root: impl AsRef<Path>,
    config: ProjectConfig,
) -> Result<ProjectWorkspace, WorkspaceError> {
    let root = root.as_ref();
    if root.exists() {
        if !root.is_dir() {
            return Err(WorkspaceError::RootNotDirectory(root.to_path_buf()));
        }
        if fs::read_dir(root).map_err(WorkspaceError::Io)?.next().is_some() {
            return Err(WorkspaceError::DirectoryNotEmpty(root.to_path_buf()));
        }
    } else {
        fs::create_dir_all(root).map_err(WorkspaceError::Io)?;
    }

    config.validate().map_err(WorkspaceError::Config)?;
    let entry_path = root.join(&config.entry_document);
    let image_path = root.join(IMAGE_DIRECTORY);
    fs::create_dir_all(&image_path).map_err(WorkspaceError::Io)?;
    atomic_write(
        root.join(CONFIG_FILE),
        config.to_json().map_err(WorkspaceError::Config)?.as_bytes(),
    )
    .map_err(WorkspaceError::Atomic)?;
    atomic_write(&entry_path, b"# Captee\n\n").map_err(WorkspaceError::Atomic)?;
    open_project(root)
}

pub fn open_project(root: impl AsRef<Path>) -> Result<ProjectWorkspace, WorkspaceError> {
    let root = root.as_ref();
    let paths = ProjectPaths::open(root).map_err(WorkspaceError::Path)?;
    let config_path = paths.require_file(CONFIG_FILE).map_err(WorkspaceError::Path)?;
    let config_text = fs::read_to_string(config_path).map_err(WorkspaceError::Io)?;
    let config = ProjectConfig::from_json(&config_text).map_err(WorkspaceError::Config)?;
    paths.require_file(&config.entry_document).map_err(WorkspaceError::Path)?;
    paths.require_directory(IMAGE_DIRECTORY).map_err(WorkspaceError::Path)?;
    Ok(ProjectWorkspace { root: root.to_path_buf(), config, paths })
}

pub trait TrashBackend {
    fn move_to_trash(&self, path: &Path) -> Result<(), TrashError>;
}

pub fn confirm_and_trash<B: TrashBackend>(
    backend: &B,
    path: &Path,
    confirmed: bool,
) -> Result<TrashOutcome, TrashError> {
    if !confirmed {
        return Ok(TrashOutcome::Cancelled);
    }
    backend.move_to_trash(path)?;
    Ok(TrashOutcome::Moved)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrashOutcome {
    Cancelled,
    Moved,
}

#[derive(Debug)]
pub enum WorkspaceError {
    Io(std::io::Error),
    Atomic(crate::AtomicWriteError),
    Config(captee_core::ConfigError),
    Path(PathError),
    RootNotDirectory(PathBuf),
    DirectoryNotEmpty(PathBuf),
}

impl fmt::Display for WorkspaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "workspace I/O failed: {error}"),
            Self::Atomic(error) => write!(formatter, "workspace atomic write failed: {error}"),
            Self::Config(error) => write!(formatter, "invalid project configuration: {error}"),
            Self::Path(error) => write!(formatter, "invalid project path: {error}"),
            Self::RootNotDirectory(path) => {
                write!(formatter, "project root is not a directory: {}", path.display())
            }
            Self::DirectoryNotEmpty(path) => {
                write!(formatter, "project directory is not empty: {}", path.display())
            }
        }
    }
}

impl std::error::Error for WorkspaceError {}

#[derive(Debug)]
pub enum TrashError {
    Io(std::io::Error),
    Backend(String),
}

impl fmt::Display for TrashError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "trash operation failed: {error}"),
            Self::Backend(error) => formatter.write_str(error),
        }
    }
}

impl std::error::Error for TrashError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos();
        let root = std::env::temp_dir().join(format!("captee-{name}-{suffix}"));
        fs::create_dir_all(&root).expect("temporary root");
        root
    }

    #[test]
    fn creates_and_reopens_a_project() {
        let root = test_root("workspace");
        let config = ProjectConfig::new("Notes", "main.typ").expect("config");
        let created = create_project(&root, config.clone()).expect("create");
        assert_eq!(created.config, config);
        assert!(root.join(CONFIG_FILE).is_file());
        assert!(root.join("main.typ").is_file());
        assert!(root.join(IMAGE_DIRECTORY).is_dir());
        assert_eq!(open_project(&root).expect("open").config, config);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn rejects_non_empty_creation_directory() {
        let root = test_root("non-empty");
        fs::write(root.join("existing.txt"), "keep").expect("existing file");
        let config = ProjectConfig::new("Notes", "main.typ").expect("config");
        assert!(matches!(create_project(&root, config), Err(WorkspaceError::DirectoryNotEmpty(_))));
        assert!(root.join("existing.txt").is_file());
        fs::remove_dir_all(root).expect("cleanup");
    }

    struct FakeTrash {
        calls: Arc<Mutex<Vec<PathBuf>>>,
    }

    impl TrashBackend for FakeTrash {
        fn move_to_trash(&self, path: &Path) -> Result<(), TrashError> {
            self.calls.lock().expect("lock").push(path.to_path_buf());
            Ok(())
        }
    }

    #[test]
    fn cancelled_trash_does_not_call_backend() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let backend = FakeTrash { calls: calls.clone() };
        assert_eq!(
            confirm_and_trash(&backend, Path::new("project"), false).expect("cancel"),
            TrashOutcome::Cancelled
        );
        assert!(calls.lock().expect("lock").is_empty());
    }
}
