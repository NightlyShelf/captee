use captee_core::{
    AppCommand, AppState, AppStateStore, AppView, DispatchError, OperationKind, ProjectSession,
    ProjectSettings,
};
use std::fmt;

pub mod annotation_bridge;
pub mod editor_bridge;
pub mod native;
pub mod operation;

/// The three logical regions that a GTK workspace adapter renders.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Pane {
    #[default]
    Navigation,
    Editor,
    Preview,
}

/// Keyboard focus targets exposed to the desktop adapter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FocusTarget {
    #[default]
    ProjectList,
    SourceEditor,
    Preview,
    FindInput,
    Status,
}

/// A keyboard action that can be registered by a GTK application window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutAction {
    Save,
    Format,
    FindReplace,
    Completion,
    Capture,
    Preview,
    Export,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shortcut {
    pub accelerator: &'static str,
    pub action: ShortcutAction,
}

/// Stable keyboard bindings for the shell. GTK registration is an adapter job.
pub const SHORTCUTS: &[Shortcut] = &[
    Shortcut { accelerator: "<Primary>s", action: ShortcutAction::Save },
    Shortcut { accelerator: "<Primary><Shift>f", action: ShortcutAction::Format },
    Shortcut { accelerator: "<Primary>f", action: ShortcutAction::FindReplace },
    Shortcut { accelerator: "<Primary>space", action: ShortcutAction::Completion },
    Shortcut { accelerator: "<Primary><Shift>c", action: ShortcutAction::Capture },
    Shortcut { accelerator: "<Primary>r", action: ShortcutAction::Preview },
    Shortcut { accelerator: "<Primary><Shift>e", action: ShortcutAction::Export },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    pub operation: OperationKind,
    pub cancellable: bool,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Announcement {
    pub label: String,
    pub is_error: bool,
}

/// Data needed to render the shell without giving widgets ownership of state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiSnapshot {
    pub app: AppState,
    pub focused: FocusTarget,
    pub active_pane: Pane,
    pub progress: Option<Progress>,
    pub announcement: Option<Announcement>,
    pub settings_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InteractionState {
    pub busy: bool,
    pub cancellable: bool,
    pub project_actions_enabled: bool,
    pub workspace_actions_enabled: bool,
    pub editor_enabled: bool,
}

pub fn interaction_state(snapshot: &UiSnapshot) -> InteractionState {
    let busy = snapshot.progress.is_some();
    let editor_enabled = snapshot.app.project.is_some()
        && (!busy
            || snapshot
                .progress
                .as_ref()
                .is_some_and(|progress| progress.operation == OperationKind::Preview));
    InteractionState {
        busy,
        cancellable: snapshot.progress.as_ref().is_some_and(|progress| progress.cancellable),
        project_actions_enabled: !busy,
        workspace_actions_enabled: snapshot.app.project.is_some() && !busy,
        editor_enabled,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiCommand {
    OpenProject { session: ProjectSession, settings: ProjectSettings },
    CloseProject,
    SetDirty(bool),
    Navigate(AppView),
    Focus(FocusTarget),
    Save,
    Format,
    FindReplace,
    Completion,
    Capture,
    StoreCapture,
    Preview,
    Export,
    SaveSettings,
    Complete { message: String },
    Fail { message: String },
    Warn { message: String },
    Cancel,
    ClearStatus,
    ApplySettings(ProjectSettings),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsValidationError {
    InvalidLineWidth(u16),
    InvalidZoom(u16),
    NoCaptureBackend,
    EmptyKeybinding(&'static str),
    DuplicateKeybinding(String),
}

impl fmt::Display for SettingsValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLineWidth(value) => {
                write!(formatter, "line width must be between 1 and 400 (got {value})")
            }
            Self::InvalidZoom(value) => {
                write!(formatter, "preview zoom must be between 25 and 500 (got {value})")
            }
            Self::NoCaptureBackend => {
                formatter.write_str("enable the screenshot portal or the slurp/grim fallback")
            }
            Self::EmptyKeybinding(action) => {
                write!(formatter, "{action} keybinding cannot be empty")
            }
            Self::DuplicateKeybinding(binding) => {
                write!(formatter, "keybinding {binding} is assigned more than once")
            }
        }
    }
}

impl std::error::Error for SettingsValidationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiError {
    Dispatch(DispatchError),
    InvalidSettings(SettingsValidationError),
}

impl fmt::Display for UiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dispatch(error) => error.fmt(formatter),
            Self::InvalidSettings(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for UiError {}

impl From<DispatchError> for UiError {
    fn from(error: DispatchError) -> Self {
        Self::Dispatch(error)
    }
}

/// Headless presentation adapter used by GTK and UI-state tests.
#[derive(Debug, Clone, Default)]
pub struct UiShell {
    store: AppStateStore,
    focused: FocusTarget,
    active_pane: Pane,
    progress: Option<Progress>,
    announcement: Option<Announcement>,
    settings_error: Option<String>,
}

impl UiShell {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> UiSnapshot {
        UiSnapshot {
            app: self.store.snapshot(),
            focused: self.focused,
            active_pane: self.active_pane,
            progress: self.progress.clone(),
            announcement: self.announcement.clone(),
            settings_error: self.settings_error.clone(),
        }
    }

    pub fn dispatch(&mut self, command: UiCommand) -> Result<(), UiError> {
        match command {
            UiCommand::OpenProject { session, settings } => {
                self.store.dispatch(AppCommand::OpenProject { session, settings })?;
                self.active_pane = Pane::Editor;
                self.focused = FocusTarget::SourceEditor;
                self.progress = None;
                self.settings_error = None;
                self.announce("Project opened", false);
            }
            UiCommand::CloseProject => {
                self.store.dispatch(AppCommand::CloseProject)?;
                self.active_pane = Pane::Navigation;
                self.focused = FocusTarget::ProjectList;
                self.progress = None;
                self.announce("Project closed", false);
            }
            UiCommand::SetDirty(dirty) => {
                self.store.dispatch(AppCommand::SetDirty(dirty))?;
            }
            UiCommand::Navigate(view) => {
                self.store.dispatch(AppCommand::Navigate(view))?;
                self.active_pane = pane_for_view(view);
                self.focused = focus_for_pane(self.active_pane);
            }
            UiCommand::Focus(target) => {
                self.focused = target;
                self.active_pane = pane_for_focus(target);
            }
            UiCommand::Save => self.start(OperationKind::Save, false, "Saving")?,
            UiCommand::Format => self.start(OperationKind::Format, true, "Formatting")?,
            UiCommand::FindReplace => {
                self.start(OperationKind::FindReplace, true, "Finding and replacing")?
            }
            UiCommand::Completion => {
                self.start(OperationKind::Completion, true, "Finding completions")?
            }
            UiCommand::Capture => self.start(OperationKind::Capture, true, "Capturing")?,
            UiCommand::StoreCapture => {
                self.start(OperationKind::Capture, false, "Saving capture")?
            }
            UiCommand::Preview => self.start(OperationKind::Preview, true, "Rendering preview")?,
            UiCommand::Export => self.start(OperationKind::Export, false, "Exporting PDF")?,
            UiCommand::SaveSettings => {
                self.start(OperationKind::Settings, false, "Saving settings")?
            }
            UiCommand::Complete { message } => {
                self.store.dispatch(AppCommand::CompleteOperation { message: message.clone() })?;
                self.progress = None;
                self.announce(message, false);
            }
            UiCommand::Fail { message } => {
                self.store.dispatch(AppCommand::ReportError { message: message.clone() })?;
                self.progress = None;
                self.announce(message, true);
            }
            UiCommand::Warn { message } => {
                self.store.dispatch(AppCommand::ReportWarning { message: message.clone() })?;
                self.progress = None;
                self.announce(message, false);
            }
            UiCommand::Cancel => {
                self.store.dispatch(AppCommand::CancelOperation)?;
                self.progress = None;
                self.announce("Operation cancelled", false);
            }
            UiCommand::ClearStatus => {
                self.store.dispatch(AppCommand::ClearActivity)?;
                self.announcement = None;
            }
            UiCommand::ApplySettings(settings) => {
                validate_settings(&settings).map_err(UiError::InvalidSettings)?;
                self.store.dispatch(AppCommand::ApplySettings(settings))?;
                self.settings_error = None;
                self.announce("Settings saved", false);
            }
        }
        Ok(())
    }

    fn start(
        &mut self,
        kind: OperationKind,
        cancellable: bool,
        label: &str,
    ) -> Result<(), UiError> {
        self.store.dispatch(AppCommand::StartOperation { kind, cancellable })?;
        self.progress = Some(Progress { operation: kind, cancellable, label: label.into() });
        self.announce(label, false);
        Ok(())
    }

    fn announce(&mut self, label: impl Into<String>, is_error: bool) {
        self.announcement = Some(Announcement { label: label.into(), is_error });
    }
}

fn validate_settings(settings: &ProjectSettings) -> Result<(), SettingsValidationError> {
    if !(1..=400).contains(&settings.formatting.line_width) {
        return Err(SettingsValidationError::InvalidLineWidth(settings.formatting.line_width));
    }
    if !(25..=500).contains(&settings.preview.zoom_percent) {
        return Err(SettingsValidationError::InvalidZoom(settings.preview.zoom_percent));
    }
    if !settings.capture.portal_enabled && !settings.capture.fallback_enabled {
        return Err(SettingsValidationError::NoCaptureBackend);
    }
    let mut bindings = std::collections::BTreeSet::new();
    for (action, binding) in settings.keybindings.named_bindings() {
        let binding = binding.trim();
        if binding.is_empty() {
            return Err(SettingsValidationError::EmptyKeybinding(action));
        }
        if !bindings.insert(binding) {
            return Err(SettingsValidationError::DuplicateKeybinding(binding.to_owned()));
        }
    }
    Ok(())
}

fn pane_for_view(view: AppView) -> Pane {
    match view {
        AppView::Preview => Pane::Preview,
        AppView::Home
        | AppView::Workspace
        | AppView::Editor
        | AppView::Capture
        | AppView::Settings => Pane::Editor,
    }
}

fn focus_for_pane(pane: Pane) -> FocusTarget {
    match pane {
        Pane::Navigation => FocusTarget::ProjectList,
        Pane::Editor => FocusTarget::SourceEditor,
        Pane::Preview => FocusTarget::Preview,
    }
}

fn pane_for_focus(focus: FocusTarget) -> Pane {
    match focus {
        FocusTarget::ProjectList => Pane::Navigation,
        FocusTarget::SourceEditor | FocusTarget::FindInput | FocusTarget::Status => Pane::Editor,
        FocusTarget::Preview => Pane::Preview,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use captee_core::{CaptureSettings, FormattingSettings, PreviewSettings};

    fn session() -> ProjectSession {
        ProjectSession::new("/tmp/notes", "Notes", "main.typ")
    }

    #[test]
    fn empty_home_has_navigation_focus_and_no_progress() {
        let shell = UiShell::new();
        let snapshot = shell.snapshot();
        assert_eq!(snapshot.app.view, AppView::Home);
        assert_eq!(snapshot.active_pane, Pane::Navigation);
        assert_eq!(snapshot.focused, FocusTarget::ProjectList);
        assert!(snapshot.progress.is_none());
    }

    #[test]
    fn opened_workspace_selects_editor_and_exposes_shortcuts() {
        let mut shell = UiShell::new();
        shell
            .dispatch(UiCommand::OpenProject {
                session: session(),
                settings: ProjectSettings::default(),
            })
            .expect("project opens");
        let snapshot = shell.snapshot();
        assert_eq!(snapshot.app.view, AppView::Workspace);
        assert_eq!(snapshot.active_pane, Pane::Editor);
        assert_eq!(snapshot.focused, FocusTarget::SourceEditor);
        assert!(SHORTCUTS.iter().any(|shortcut| shortcut.action == ShortcutAction::Save));
    }

    #[test]
    fn invalid_settings_are_rejected_without_mutating_previous_settings() {
        let mut shell = UiShell::new();
        shell
            .dispatch(UiCommand::OpenProject {
                session: session(),
                settings: ProjectSettings::default(),
            })
            .expect("project opens");
        let mut invalid = ProjectSettings {
            formatting: FormattingSettings { line_width: 0, format_on_save: true },
            capture: CaptureSettings::default(),
            preview: PreviewSettings::default(),
            keybindings: captee_core::KeybindingSettings::default(),
        };
        assert!(matches!(
            shell.dispatch(UiCommand::ApplySettings(invalid.clone())),
            Err(UiError::InvalidSettings(SettingsValidationError::InvalidLineWidth(0)))
        ));
        assert_eq!(shell.snapshot().app.settings, ProjectSettings::default());
        invalid.formatting.line_width = 90;
        invalid.preview.zoom_percent = 125;
        shell.dispatch(UiCommand::ApplySettings(invalid.clone())).expect("settings save");
        assert_eq!(shell.snapshot().app.settings, invalid);
    }

    #[test]
    fn duplicate_shortcuts_and_disabled_capture_are_rejected() {
        let mut shell = UiShell::new();
        shell
            .dispatch(UiCommand::OpenProject {
                session: session(),
                settings: ProjectSettings::default(),
            })
            .expect("project opens");
        let mut invalid = ProjectSettings::default();
        invalid.keybindings.capture = invalid.keybindings.save.clone();
        assert!(matches!(
            shell.dispatch(UiCommand::ApplySettings(invalid)),
            Err(UiError::InvalidSettings(SettingsValidationError::DuplicateKeybinding(_)))
        ));
        let mut invalid = ProjectSettings::default();
        invalid.capture.portal_enabled = false;
        invalid.capture.fallback_enabled = false;
        assert!(matches!(
            shell.dispatch(UiCommand::ApplySettings(invalid)),
            Err(UiError::InvalidSettings(SettingsValidationError::NoCaptureBackend))
        ));
        assert_eq!(shell.snapshot().app.settings, ProjectSettings::default());
    }

    #[test]
    fn failed_operation_is_announced_and_clears_progress() {
        let mut shell = UiShell::new();
        shell
            .dispatch(UiCommand::OpenProject {
                session: session(),
                settings: ProjectSettings::default(),
            })
            .expect("project opens");
        shell.dispatch(UiCommand::Preview).expect("preview starts");
        assert!(shell.snapshot().progress.is_some());
        shell
            .dispatch(UiCommand::Fail { message: "Typst failed".into() })
            .expect("failure reports");
        let snapshot = shell.snapshot();
        assert!(snapshot.progress.is_none());
        let announcement = snapshot.announcement.expect("announcement");
        assert_eq!(announcement.label, "Typst failed");
        assert!(announcement.is_error);
        assert_eq!(snapshot.app.activity, captee_core::Activity::Failed("Typst failed".into()));
    }

    #[test]
    fn cancellation_keeps_the_editor_context() {
        let mut shell = UiShell::new();
        shell
            .dispatch(UiCommand::OpenProject {
                session: session(),
                settings: ProjectSettings::default(),
            })
            .expect("project opens");
        shell.dispatch(UiCommand::Capture).expect("capture starts");
        shell.dispatch(UiCommand::Cancel).expect("capture cancels");
        assert_eq!(shell.snapshot().app.view, AppView::Workspace);
        assert_eq!(shell.snapshot().focused, FocusTarget::SourceEditor);
    }

    #[test]
    fn confirmed_capture_storage_is_not_cancellable_mid_write() {
        let mut shell = UiShell::new();
        shell
            .dispatch(UiCommand::OpenProject {
                session: session(),
                settings: ProjectSettings::default(),
            })
            .expect("project opens");
        shell.dispatch(UiCommand::StoreCapture).expect("capture storage starts");
        let progress = shell.snapshot().progress.expect("storage progress");
        assert_eq!(progress.operation, OperationKind::Capture);
        assert_eq!(progress.label, "Saving capture");
        assert!(!progress.cancellable);
        assert!(shell.dispatch(UiCommand::Cancel).is_err());
    }

    #[test]
    fn interaction_state_disables_conflicts_and_exposes_only_valid_cancellation() {
        let mut shell = UiShell::new();
        assert_eq!(
            interaction_state(&shell.snapshot()),
            InteractionState {
                busy: false,
                cancellable: false,
                project_actions_enabled: true,
                workspace_actions_enabled: false,
                editor_enabled: false,
            }
        );
        shell
            .dispatch(UiCommand::OpenProject {
                session: session(),
                settings: ProjectSettings::default(),
            })
            .expect("project opens");
        shell.dispatch(UiCommand::Capture).expect("capture starts");
        assert_eq!(
            interaction_state(&shell.snapshot()),
            InteractionState {
                busy: true,
                cancellable: true,
                project_actions_enabled: false,
                workspace_actions_enabled: false,
                editor_enabled: false,
            }
        );
        shell.dispatch(UiCommand::Cancel).expect("capture cancels");
        assert!(interaction_state(&shell.snapshot()).workspace_actions_enabled);
    }

    #[test]
    fn preview_keeps_the_source_editor_editable_while_disabling_conflicting_actions() {
        let mut shell = UiShell::new();
        shell
            .dispatch(UiCommand::OpenProject {
                session: session(),
                settings: ProjectSettings::default(),
            })
            .expect("project opens");
        shell.dispatch(UiCommand::Preview).expect("preview starts");

        let interaction = interaction_state(&shell.snapshot());
        assert!(interaction.busy);
        assert!(interaction.editor_enabled);
        assert!(!interaction.workspace_actions_enabled);
        assert!(interaction.cancellable);
    }

    #[test]
    fn closing_a_project_returns_to_the_home_surface() {
        let mut shell = UiShell::new();
        shell
            .dispatch(UiCommand::OpenProject {
                session: session(),
                settings: ProjectSettings::default(),
            })
            .expect("project opens");
        shell.dispatch(UiCommand::CloseProject).expect("project closes");
        let snapshot = shell.snapshot();
        assert_eq!(snapshot.app.view, AppView::Home);
        assert!(snapshot.app.project.is_none());
        assert_eq!(snapshot.active_pane, Pane::Navigation);
        assert_eq!(snapshot.focused, FocusTarget::ProjectList);
    }
}
