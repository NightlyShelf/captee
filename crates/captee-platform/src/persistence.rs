use crate::{atomic_write, AtomicWriteError, AutosaveStore, PathError, ProjectPaths};
use captee_core::{DocumentPersistence, KeybindingSettings, RecentProjects};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub const AUTOSAVE_FILE: &str = ".captee-autosave";
pub const WORKSPACE_VIEW_FILE: &str = ".captee-view.json";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceViewState {
    pub document: String,
    pub cursor_offset: usize,
    pub editor_scroll: f64,
    pub preview_page: usize,
    pub preview_y_ratio: f64,
}

#[derive(Debug, Clone)]
pub struct WorkspaceViewStore {
    path: PathBuf,
}

impl WorkspaceViewStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn load(&self) -> Result<Option<WorkspaceViewState>, WorkspaceViewError> {
        if !self.path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&self.path).map_err(WorkspaceViewError::Io)?;
        let state = serde_json::from_slice::<WorkspaceViewState>(&bytes)
            .map_err(WorkspaceViewError::Serialization)?;
        Ok(Some(state.normalized()))
    }

    pub fn save(&self, state: &WorkspaceViewState) -> Result<(), WorkspaceViewError> {
        let parent = self.path.parent().ok_or_else(|| {
            WorkspaceViewError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "workspace view path has no parent",
            ))
        })?;
        fs::create_dir_all(parent).map_err(WorkspaceViewError::Io)?;
        let payload = serde_json::to_vec_pretty(&state.clone().normalized())
            .map_err(WorkspaceViewError::Serialization)?;
        atomic_write(&self.path, &payload).map_err(WorkspaceViewError::Atomic)
    }
}

impl WorkspaceViewState {
    fn normalized(mut self) -> Self {
        self.editor_scroll = nonnegative_value(self.editor_scroll);
        self.preview_page = self.preview_page.max(1);
        self.preview_y_ratio = normalized_value(self.preview_y_ratio);
        self
    }
}

fn nonnegative_value(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn normalized_value(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[derive(Debug)]
pub enum WorkspaceViewError {
    Io(std::io::Error),
    Serialization(serde_json::Error),
    Atomic(AtomicWriteError),
}

impl fmt::Display for WorkspaceViewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "workspace view I/O failed: {error}"),
            Self::Serialization(error) => {
                write!(formatter, "workspace view data is invalid: {error}")
            }
            Self::Atomic(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for WorkspaceViewError {}

#[derive(Debug, Clone)]
pub struct GlobalKeybindingStore {
    path: PathBuf,
}

impl GlobalKeybindingStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn exists(&self) -> bool {
        self.path.is_file()
    }

    pub fn load(&self) -> Result<KeybindingSettings, GlobalKeybindingError> {
        if !self.exists() {
            return Ok(KeybindingSettings::default());
        }
        let bytes = fs::read(&self.path).map_err(GlobalKeybindingError::Io)?;
        serde_json::from_slice(&bytes).map_err(GlobalKeybindingError::Serialization)
    }

    pub fn save(&self, keybindings: &KeybindingSettings) -> Result<(), GlobalKeybindingError> {
        let parent = self.path.parent().ok_or_else(|| {
            GlobalKeybindingError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "global keybinding path has no parent",
            ))
        })?;
        fs::create_dir_all(parent).map_err(GlobalKeybindingError::Io)?;
        let payload =
            serde_json::to_vec_pretty(keybindings).map_err(GlobalKeybindingError::Serialization)?;
        atomic_write(&self.path, &payload).map_err(GlobalKeybindingError::Atomic)
    }
}

#[derive(Debug)]
pub enum GlobalKeybindingError {
    Io(std::io::Error),
    Serialization(serde_json::Error),
    Atomic(AtomicWriteError),
}

impl fmt::Display for GlobalKeybindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "global keybinding I/O failed: {error}"),
            Self::Serialization(error) => {
                write!(formatter, "global keybindings are invalid: {error}")
            }
            Self::Atomic(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for GlobalKeybindingError {}

#[derive(Debug, Clone)]
pub struct ProjectDocumentPersistence {
    entry_path: PathBuf,
    autosave: AutosaveStore,
}

impl ProjectDocumentPersistence {
    pub fn open(
        project_root: impl AsRef<Path>,
        entry_document: impl AsRef<Path>,
    ) -> Result<Self, PersistenceError> {
        let paths = ProjectPaths::open(project_root).map_err(PersistenceError::Path)?;
        let entry_path = paths.require_file(entry_document).map_err(PersistenceError::Path)?;
        let autosave_path = paths.resolve(AUTOSAVE_FILE).map_err(PersistenceError::Path)?;
        Ok(Self { entry_path, autosave: AutosaveStore::new(autosave_path) })
    }

    pub fn autosave(&self, revision: u64, contents: &str) -> Result<(), PersistenceError> {
        self.autosave.write(revision, contents.as_bytes()).map_err(PersistenceError::Atomic)
    }

    pub fn clear_autosave(&self) -> Result<(), PersistenceError> {
        self.autosave.clear().map_err(PersistenceError::Atomic)
    }
}

impl DocumentPersistence for ProjectDocumentPersistence {
    type Error = PersistenceError;

    fn save(&self, contents: &str) -> Result<(), Self::Error> {
        atomic_write(&self.entry_path, contents.as_bytes()).map_err(PersistenceError::Atomic)
    }
}

#[derive(Debug)]
pub enum PersistenceError {
    Path(PathError),
    Atomic(AtomicWriteError),
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path(error) => error.fmt(formatter),
            Self::Atomic(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for PersistenceError {}

#[derive(Debug, Clone)]
pub struct RecentProjectStore {
    path: PathBuf,
}

impl RecentProjectStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn load(&self) -> Result<RecentProjects, RecentProjectError> {
        if !self.path.exists() {
            return Ok(RecentProjects::default());
        }
        let bytes = fs::read(&self.path).map_err(RecentProjectError::Io)?;
        let mut recent: RecentProjects =
            serde_json::from_slice(&bytes).map_err(RecentProjectError::Serialization)?;
        recent.migrate_legacy_paths();
        Ok(recent)
    }

    pub fn record(
        &self,
        name: impl Into<String>,
        path: impl Into<String>,
        last_access_unix_seconds: u64,
    ) -> Result<RecentProjects, RecentProjectError> {
        let mut recent = self.load()?;
        recent.record(name, path, last_access_unix_seconds);
        self.save(&recent)?;
        Ok(recent)
    }

    pub fn set_pinned(
        &self,
        path: &str,
        pinned: bool,
    ) -> Result<RecentProjects, RecentProjectError> {
        let mut recent = self.load()?;
        recent.set_pinned(path, pinned);
        self.save(&recent)?;
        Ok(recent)
    }

    pub fn remove(&self, path: &str) -> Result<RecentProjects, RecentProjectError> {
        let mut recent = self.load()?;
        recent.remove(path);
        self.save(&recent)?;
        Ok(recent)
    }

    fn save(&self, recent: &RecentProjects) -> Result<(), RecentProjectError> {
        let payload =
            serde_json::to_vec_pretty(&recent).map_err(RecentProjectError::Serialization)?;
        atomic_write(&self.path, &payload).map_err(RecentProjectError::Atomic)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum RecentProjectError {
    Io(std::io::Error),
    Serialization(serde_json::Error),
    Atomic(AtomicWriteError),
}

impl fmt::Display for RecentProjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "recent-project read failed: {error}"),
            Self::Serialization(error) => {
                write!(formatter, "recent-project data is invalid: {error}")
            }
            Self::Atomic(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for RecentProjectError {}

#[cfg(test)]
mod tests {
    use super::*;
    use captee_core::SourceDocument;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos();
        let root = std::env::temp_dir().join(format!("captee-persistence-{name}-{suffix}"));
        fs::create_dir_all(&root).expect("temporary root");
        root
    }

    #[test]
    fn document_save_is_atomic_and_clears_only_after_core_success() {
        let root = test_root("document");
        fs::write(root.join("main.typ"), "old").expect("entry");
        let persistence = ProjectDocumentPersistence::open(&root, "main.typ").expect("persistence");
        let mut document = SourceDocument::new("new");
        document.replace(3..3, " source").expect("edit");

        document.save(&persistence).expect("save");

        assert_eq!(fs::read_to_string(root.join("main.typ")).expect("read"), "new source");
        assert!(!document.is_dirty());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn autosave_is_project_local_and_can_be_cleared() {
        let root = test_root("autosave");
        fs::write(root.join("main.typ"), "source").expect("entry");
        let persistence = ProjectDocumentPersistence::open(&root, "main.typ").expect("persistence");

        persistence.autosave(4, "draft").expect("autosave");
        assert!(root.join(AUTOSAVE_FILE).is_file());
        persistence.clear_autosave().expect("clear");
        assert!(!root.join(AUTOSAVE_FILE).exists());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn recent_projects_are_persisted_deduplicated_and_bounded() {
        let root = test_root("recent");
        let store = RecentProjectStore::new(root.join("recent.json"));
        for index in 0..7 {
            store
                .record(format!("Project {index}"), format!("project-{index}"), index)
                .expect("record");
        }
        store.record("Project 5", "project-5", 9).expect("deduplicate");

        let recent = store.load().expect("load");
        assert_eq!(recent.entries.first().map(|entry| entry.path.as_str()), Some("project-5"));
        assert_eq!(recent.entries.len(), 5);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn recent_project_pin_and_removal_are_persisted() {
        let root = test_root("recent-actions");
        let store = RecentProjectStore::new(root.join("recent.json"));
        store.record("Notes", "/work/notes", 1).expect("record");
        store.set_pinned("/work/notes", true).expect("pin");

        let recent = store.load().expect("load pinned project");
        assert!(recent.entries[0].pinned);

        store.remove("/work/notes").expect("remove");
        assert!(store.load().expect("load removal").entries.is_empty());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn global_keybindings_are_persisted_outside_projects() {
        let root = test_root("global-keybindings");
        let store = GlobalKeybindingStore::new(root.join("settings/keybindings.json"));
        assert_eq!(store.load().expect("defaults"), KeybindingSettings::default());

        let keybindings = KeybindingSettings {
            capture: "<Primary>asciitilde".to_owned(),
            ..KeybindingSettings::default()
        };
        store.save(&keybindings).expect("save");

        assert!(store.exists());
        assert_eq!(store.load().expect("load"), keybindings);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn workspace_view_is_persisted_in_the_project() {
        let root = test_root("workspace-view");
        let store = WorkspaceViewStore::new(root.join(WORKSPACE_VIEW_FILE));
        assert_eq!(store.load().expect("empty view"), None);

        let state = WorkspaceViewState {
            document: "main.typ".to_owned(),
            cursor_offset: 42,
            editor_scroll: 640.0,
            preview_page: 3,
            preview_y_ratio: 0.75,
        };
        store.save(&state).expect("save view");

        assert_eq!(store.load().expect("load view"), Some(state));
        fs::remove_dir_all(root).expect("cleanup");
    }
}
