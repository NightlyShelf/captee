//! Project creation/opening and platform trash boundaries.

use crate::{atomic_write, PathError, ProjectPaths};
use captee_core::{ProjectConfig, ProjectSettings};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectTreeEntry {
    pub relative_path: PathBuf,
    pub is_directory: bool,
}

pub fn list_project_tree(root: impl AsRef<Path>) -> Result<Vec<ProjectTreeEntry>, WorkspaceError> {
    let paths = ProjectPaths::open(root).map_err(WorkspaceError::Path)?;
    let mut entries = Vec::new();
    collect_tree(&paths, Path::new(""), &mut entries)?;
    Ok(entries)
}

fn collect_tree(
    paths: &ProjectPaths,
    relative: &Path,
    entries: &mut Vec<ProjectTreeEntry>,
) -> Result<(), WorkspaceError> {
    let directory = paths.resolve(relative).map_err(WorkspaceError::Path)?;
    let mut children = fs::read_dir(directory)
        .map_err(WorkspaceError::Io)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(WorkspaceError::Io)?;
    children.sort_by_key(|item| item.file_name());
    for item in children {
        let file_type = item.file_type().map_err(WorkspaceError::Io)?;
        if file_type.is_symlink() {
            continue;
        }
        if relative.as_os_str().is_empty()
            && matches!(item.file_name().to_str(), Some(name) if name == CONFIG_FILE || name == ".captee-autosave")
        {
            continue;
        }
        let child = if relative.as_os_str().is_empty() {
            PathBuf::from(item.file_name())
        } else {
            relative.join(item.file_name())
        };
        let is_directory = file_type.is_dir();
        entries.push(ProjectTreeEntry { relative_path: child.clone(), is_directory });
        if is_directory {
            collect_tree(paths, &child, entries)?;
        }
    }
    Ok(())
}

pub fn create_project_item(
    root: impl AsRef<Path>,
    parent: impl AsRef<Path>,
    name: &str,
    directory: bool,
) -> Result<PathBuf, WorkspaceError> {
    validate_item_name(name)?;
    let paths = ProjectPaths::open(root).map_err(WorkspaceError::Path)?;
    let parent = paths.resolve(parent).map_err(WorkspaceError::Path)?;
    if !parent.is_dir() {
        return Err(WorkspaceError::Path(PathError::ExpectedDirectory(parent)));
    }
    let target = parent.join(name);
    paths
        .resolve(target.strip_prefix(paths.root()).unwrap_or(Path::new(name)))
        .map_err(WorkspaceError::Path)?;
    if target.exists() {
        return Err(WorkspaceError::ItemExists(target));
    }
    if directory {
        fs::create_dir(&target).map_err(WorkspaceError::Io)?;
    } else {
        fs::File::create(&target).map_err(WorkspaceError::Io)?;
    }
    Ok(target)
}

pub fn move_project_item(
    root: impl AsRef<Path>,
    source: impl AsRef<Path>,
    destination_directory: impl AsRef<Path>,
) -> Result<PathBuf, WorkspaceError> {
    let paths = ProjectPaths::open(root).map_err(WorkspaceError::Path)?;
    let source_relative = source.as_ref();
    let destination_relative = destination_directory.as_ref();
    let source_path = paths.resolve(source_relative).map_err(WorkspaceError::Path)?;
    let destination = paths.resolve(destination_relative).map_err(WorkspaceError::Path)?;
    if !source_path.exists() {
        return Err(WorkspaceError::Path(PathError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "source item does not exist",
        ))));
    }
    if !destination.is_dir() {
        return Err(WorkspaceError::Path(PathError::ExpectedDirectory(destination)));
    }
    if source_path.is_dir() && destination.starts_with(&source_path) {
        return Err(WorkspaceError::MoveIntoDescendant);
    }
    let target = destination.join(source_path.file_name().ok_or(WorkspaceError::InvalidItemName)?);
    if target.exists() {
        return Err(WorkspaceError::ItemExists(target));
    }
    fs::rename(&source_path, &target).map_err(WorkspaceError::Io)?;
    Ok(target)
}

pub fn rename_project_item(
    root: impl AsRef<Path>,
    source: impl AsRef<Path>,
    new_name: &str,
) -> Result<PathBuf, WorkspaceError> {
    validate_item_name(new_name)?;
    let paths = ProjectPaths::open(root).map_err(WorkspaceError::Path)?;
    let source_path = paths.resolve(source.as_ref()).map_err(WorkspaceError::Path)?;
    if source_path == paths.root() {
        return Err(WorkspaceError::InvalidItemName);
    }
    if !source_path.exists() {
        return Err(WorkspaceError::Path(PathError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "source item does not exist",
        ))));
    }
    let target = source_path.parent().ok_or(WorkspaceError::InvalidItemName)?.join(new_name.trim());
    if target.exists() {
        return Err(WorkspaceError::ItemExists(target));
    }
    fs::rename(source_path, &target).map_err(WorkspaceError::Io)?;
    Ok(target)
}

pub fn delete_project_item(
    root: impl AsRef<Path>,
    relative: impl AsRef<Path>,
) -> Result<(), WorkspaceError> {
    let paths = ProjectPaths::open(root).map_err(WorkspaceError::Path)?;
    let target = paths.resolve(relative).map_err(WorkspaceError::Path)?;
    if target == paths.root() {
        return Err(WorkspaceError::InvalidItemName);
    }
    let metadata = fs::symlink_metadata(&target).map_err(WorkspaceError::Io)?;
    if metadata.is_dir() {
        fs::remove_dir_all(target).map_err(WorkspaceError::Io)
    } else {
        fs::remove_file(target).map_err(WorkspaceError::Io)
    }
}

fn validate_item_name(name: &str) -> Result<(), WorkspaceError> {
    let path = Path::new(name.trim());
    if name.trim().is_empty()
        || !path.is_relative()
        || path.components().count() != 1
        || path == Path::new(".")
        || path == Path::new("..")
    {
        return Err(WorkspaceError::InvalidItemName);
    }
    Ok(())
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
    atomic_write(&entry_path, b"= Captee\n\n").map_err(WorkspaceError::Atomic)?;
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

/// Replaces only the validated settings section of an existing project config.
pub fn save_project_settings(
    root: impl AsRef<Path>,
    settings: ProjectSettings,
) -> Result<ProjectConfig, WorkspaceError> {
    let mut workspace = open_project(root)?;
    workspace.config.settings = settings;
    workspace.config.validate().map_err(WorkspaceError::Config)?;
    let config_path = workspace.paths.require_file(CONFIG_FILE).map_err(WorkspaceError::Path)?;
    let json = workspace.config.to_json().map_err(WorkspaceError::Config)?;
    atomic_write(config_path, json.as_bytes()).map_err(WorkspaceError::Atomic)?;
    Ok(workspace.config)
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
    ItemExists(PathBuf),
    InvalidItemName,
    MoveIntoDescendant,
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
            Self::ItemExists(path) => {
                write!(formatter, "project item already exists: {}", path.display())
            }
            Self::InvalidItemName => {
                formatter.write_str("project item name must be one safe path component")
            }
            Self::MoveIntoDescendant => {
                formatter.write_str("cannot move a folder into itself or a descendant")
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
        assert_eq!(
            fs::read_to_string(root.join("main.typ")).expect("entry source"),
            "= Captee\n\n"
        );
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

    #[test]
    fn settings_are_atomically_persisted_without_changing_project_identity() {
        let root = test_root("settings");
        let config = ProjectConfig::new("Notes", "main.typ").expect("config");
        create_project(&root, config).expect("create");
        let mut settings = ProjectSettings::default();
        settings.capture.fallback_enabled = false;
        settings.preview.zoom_percent = 150;
        settings.keybindings.capture = "<Primary><Alt>c".to_owned();

        let saved = save_project_settings(&root, settings.clone()).expect("save settings");
        assert_eq!(saved.name, "Notes");
        assert_eq!(saved.entry_document, "main.typ");
        assert_eq!(open_project(&root).expect("reopen").config.settings, settings);
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

    #[test]
    fn project_tree_supports_safe_create_list_and_move() {
        let root = test_root("tree");
        create_project_item(&root, "", "notes", true).expect("folder");
        create_project_item(&root, "notes", "draft.typ", false).expect("file");
        let entries = list_project_tree(&root).expect("tree");
        assert!(entries.iter().any(|entry| entry.relative_path == Path::new("notes")));
        assert!(entries.iter().any(|entry| entry.relative_path == Path::new("notes/draft.typ")));
        move_project_item(&root, "notes/draft.typ", "").expect("move file");
        assert!(root.join("draft.typ").is_file());
        assert!(matches!(
            move_project_item(&root, "notes", "notes"),
            Err(WorkspaceError::MoveIntoDescendant)
        ));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn project_tree_supports_rename() {
        let root = test_root("rename");
        create_project_item(&root, "", "draft.typ", false).expect("file");
        rename_project_item(&root, "draft.typ", "main.typ").expect("rename");
        assert!(root.join("main.typ").is_file());
        assert!(!root.join("draft.typ").exists());
        fs::remove_dir_all(root).expect("cleanup");
    }
}
