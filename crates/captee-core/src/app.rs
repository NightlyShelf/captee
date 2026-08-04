use crate::ProjectSettings;
use std::fmt;

/// Top-level surfaces exposed by the desktop shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppView {
    Home,
    Workspace,
    Editor,
    Preview,
    Capture,
    Settings,
}

/// Work that can make the application busy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Save,
    Format,
    FindReplace,
    Completion,
    Capture,
    Preview,
    Export,
    Settings,
    LoadingProject,
}

impl OperationKind {
    fn requires_project(self) -> bool {
        !matches!(self, Self::LoadingProject)
    }
}

/// User-visible status for the current command or background operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Activity {
    Idle,
    Running { kind: OperationKind, cancellable: bool },
    Succeeded(String),
    Warning(String),
    Failed(String),
}

impl Activity {
    fn is_running(&self) -> bool {
        matches!(self, Self::Running { .. })
    }
}

/// Project context needed by the shell to label and route workspace actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSession {
    pub root: String,
    pub name: String,
    pub entry_document: String,
}

impl ProjectSession {
    pub fn new(
        root: impl Into<String>,
        name: impl Into<String>,
        entry_document: impl Into<String>,
    ) -> Self {
        Self { root: root.into(), name: name.into(), entry_document: entry_document.into() }
    }
}

/// Immutable application snapshot consumed by UI adapters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppState {
    pub view: AppView,
    pub project: Option<ProjectSession>,
    pub dirty: bool,
    pub activity: Activity,
    pub settings: ProjectSettings,
    pub settings_error: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            view: AppView::Home,
            project: None,
            dirty: false,
            activity: Activity::Idle,
            settings: ProjectSettings::default(),
            settings_error: None,
        }
    }
}

/// Commands accepted by the headless application state boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppCommand {
    OpenProject { session: ProjectSession, settings: ProjectSettings },
    CloseProject,
    Navigate(AppView),
    SetDirty(bool),
    StartOperation { kind: OperationKind, cancellable: bool },
    CompleteOperation { message: String },
    ReportWarning { message: String },
    ReportError { message: String },
    CancelOperation,
    ClearActivity,
    ApplySettings(ProjectSettings),
}

/// Rejection returned when a command cannot be applied to the current state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchError {
    ProjectRequired,
    Busy,
    InvalidTransition(&'static str),
    NotCancellable,
    NoOperation,
}

impl fmt::Display for DispatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProjectRequired => formatter.write_str("a project is required"),
            Self::Busy => formatter.write_str("another operation is already running"),
            Self::InvalidTransition(message) => formatter.write_str(message),
            Self::NotCancellable => {
                formatter.write_str("the current operation cannot be cancelled")
            }
            Self::NoOperation => formatter.write_str("no operation is running"),
        }
    }
}

impl std::error::Error for DispatchError {}

/// Small, deterministic state machine shared by GTK and headless tests.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppStateStore {
    state: AppState,
    version: u64,
}

impl AppStateStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn snapshot(&self) -> AppState {
        self.state.clone()
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn dispatch(&mut self, command: AppCommand) -> Result<(), DispatchError> {
        if self.state.activity.is_running()
            && !matches!(
                command,
                AppCommand::CancelOperation
                    | AppCommand::CompleteOperation { .. }
                    | AppCommand::ReportWarning { .. }
                    | AppCommand::ReportError { .. }
                    | AppCommand::SetDirty(_)
            )
        {
            return Err(DispatchError::Busy);
        }

        match command {
            AppCommand::OpenProject { session, settings } => {
                self.state.project = Some(session);
                self.state.settings = settings;
                self.state.settings_error = None;
                self.state.dirty = false;
                self.state.view = AppView::Workspace;
                self.state.activity = Activity::Idle;
            }
            AppCommand::CloseProject => {
                self.state.project = None;
                self.state.settings = ProjectSettings::default();
                self.state.settings_error = None;
                self.state.dirty = false;
                self.state.view = AppView::Home;
                self.state.activity = Activity::Idle;
            }
            AppCommand::Navigate(view) => {
                if view != AppView::Home && self.state.project.is_none() {
                    return Err(DispatchError::ProjectRequired);
                }
                self.state.view = view;
            }
            AppCommand::SetDirty(dirty) => {
                if self.state.project.is_none() {
                    return Err(DispatchError::ProjectRequired);
                }
                self.state.dirty = dirty;
            }
            AppCommand::StartOperation { kind, cancellable } => {
                if kind.requires_project() && self.state.project.is_none() {
                    return Err(DispatchError::ProjectRequired);
                }
                self.state.activity = Activity::Running { kind, cancellable };
            }
            AppCommand::CompleteOperation { message } => {
                if !self.state.activity.is_running() {
                    return Err(DispatchError::NoOperation);
                }
                self.state.activity = Activity::Succeeded(message);
            }
            AppCommand::ReportWarning { message } => {
                self.state.activity = Activity::Warning(message);
            }
            AppCommand::ReportError { message } => {
                self.state.activity = Activity::Failed(message);
            }
            AppCommand::CancelOperation => match self.state.activity {
                Activity::Running { cancellable: true, .. } => self.state.activity = Activity::Idle,
                Activity::Running { cancellable: false, .. } => {
                    return Err(DispatchError::NotCancellable)
                }
                _ => return Err(DispatchError::NoOperation),
            },
            AppCommand::ClearActivity => self.state.activity = Activity::Idle,
            AppCommand::ApplySettings(settings) => {
                if self.state.project.is_none() {
                    return Err(DispatchError::ProjectRequired);
                }
                self.state.settings = settings;
                self.state.settings_error = None;
            }
        }

        self.version = self.version.saturating_add(1);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opened_store() -> AppStateStore {
        let mut store = AppStateStore::new();
        store
            .dispatch(AppCommand::OpenProject {
                session: ProjectSession::new("/tmp/notes", "Notes", "main.typ"),
                settings: ProjectSettings::default(),
            })
            .expect("project opens");
        store
    }

    #[test]
    fn new_store_starts_on_empty_home() {
        let store = AppStateStore::new();
        assert_eq!(store.state().view, AppView::Home);
        assert!(store.state().project.is_none());
        assert_eq!(store.state().activity, Activity::Idle);
        assert_eq!(store.version(), 0);
    }

    #[test]
    fn opening_project_exposes_workspace_context() {
        let mut store = AppStateStore::new();
        let settings = ProjectSettings::default();
        store
            .dispatch(AppCommand::OpenProject {
                session: ProjectSession::new("/tmp/notes", "Notes", "main.typ"),
                settings: settings.clone(),
            })
            .expect("project opens");

        assert_eq!(store.state().view, AppView::Workspace);
        assert_eq!(store.state().project.as_ref().expect("session").name, "Notes");
        assert_eq!(store.state().settings, settings);
        assert!(!store.state().dirty);
        assert_eq!(store.version(), 1);
    }

    #[test]
    fn project_only_actions_are_rejected_from_home() {
        let mut store = AppStateStore::new();
        assert_eq!(
            store.dispatch(AppCommand::Navigate(AppView::Editor)),
            Err(DispatchError::ProjectRequired)
        );
        assert_eq!(
            store.dispatch(AppCommand::StartOperation {
                kind: OperationKind::Save,
                cancellable: false,
            }),
            Err(DispatchError::ProjectRequired)
        );
        assert_eq!(store.version(), 0);
    }

    #[test]
    fn busy_operations_block_navigation_and_close() {
        let mut store = opened_store();
        store
            .dispatch(AppCommand::StartOperation {
                kind: OperationKind::Capture,
                cancellable: true,
            })
            .expect("capture starts");

        assert_eq!(
            store.dispatch(AppCommand::Navigate(AppView::Preview)),
            Err(DispatchError::Busy)
        );
        assert_eq!(store.dispatch(AppCommand::CloseProject), Err(DispatchError::Busy));
        store.dispatch(AppCommand::CancelOperation).expect("capture cancels");
        store.dispatch(AppCommand::CloseProject).expect("project closes");
        assert_eq!(store.state().view, AppView::Home);
    }

    #[test]
    fn cancellation_does_not_mutate_document_context() {
        let mut store = opened_store();
        store.dispatch(AppCommand::SetDirty(true)).expect("document is dirty");
        let project = store.state().project.clone();
        store
            .dispatch(AppCommand::StartOperation {
                kind: OperationKind::Capture,
                cancellable: true,
            })
            .expect("capture starts");
        store.dispatch(AppCommand::CancelOperation).expect("capture cancels");

        assert_eq!(store.state().project, project);
        assert!(store.state().dirty);
        assert_eq!(store.state().activity, Activity::Idle);
    }

    #[test]
    fn completion_preserves_project_state() {
        let mut store = opened_store();
        let project = store.state().project.clone();
        store
            .dispatch(AppCommand::StartOperation {
                kind: OperationKind::Preview,
                cancellable: false,
            })
            .expect("preview starts");
        store
            .dispatch(AppCommand::CompleteOperation { message: "Rendered".into() })
            .expect("preview completes");

        assert_eq!(store.state().project, project);
        assert_eq!(store.state().activity, Activity::Succeeded("Rendered".into()));
    }

    #[test]
    fn edits_can_mark_the_document_dirty_while_revision_work_is_running() {
        let mut store = opened_store();
        store
            .dispatch(AppCommand::StartOperation {
                kind: OperationKind::Preview,
                cancellable: true,
            })
            .expect("preview starts");

        store.dispatch(AppCommand::SetDirty(true)).expect("edit remains available");

        assert!(store.state().dirty);
        assert!(matches!(store.state().activity, Activity::Running { .. }));
    }
}
