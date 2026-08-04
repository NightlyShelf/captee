use crate::editor_bridge::{EditorBridge, EditorState};
use crate::operation::{
    OperationCoordinator, OperationOutcome, ProjectIdentity, ResultDisposition, SourceIdentity,
};
use crate::{UiCommand, UiShell};
use captee_core::{
    replace_literal, request_completions, CaptureBackend, CaptureResult, CapturedImage,
    CompletionItem, Diagnostic, DiagnosticSeverity, Operation, OperationKind, ProjectConfig,
    ProjectSession, RenderState, SourceDocument,
};
use captee_platform::{
    create_project, export_pdf, open_project, AsyncPreviewCompiler, AutosaveSnapshot,
    AutosaveStore, CaptureSelector, FormattedSource, GrimSlurpCapture, PreviewOutcome,
    ProjectDocumentPersistence, RecentProjectStore, TypstCompletionProvider, TypstFormatter,
    TypstPreviewCompiler, TypstRunner, XdgPortalCapture, AUTOSAVE_FILE,
};
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, Dialog, Entry, Label, MenuButton,
    Orientation, Paned, ResponseType, ScrolledWindow, Stack,
};
use gtk4 as gtk;
use sourceview::prelude::*;
use sourceview5 as sourceview;
use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

const APPLICATION_ID: &str = "com.nightlyshelf.Captee";

#[derive(Debug)]
enum WorkspaceOperationResult {
    Saved(SourceDocument),
    Formatted(FormattedSource),
    Completions { items: Vec<CompletionItem>, cursor: usize },
    AuthoringFailure { message: String, diagnostics: Vec<Diagnostic> },
    Preview(PreviewOutcome),
    Exported(PathBuf),
    Captured(CapturedImage),
}

#[derive(Debug)]
enum BackgroundResult {
    Autosave { source: SourceIdentity, result: Result<(), String> },
    RecentProject { project: ProjectIdentity, result: Result<(), String> },
}

/// Starts the GTK application with a project home screen and workspace shell.
pub fn run() -> glib::ExitCode {
    let application = Application::builder().application_id(APPLICATION_ID).build();
    application.connect_activate(build_ui);
    application.run()
}

fn build_ui(application: &Application) {
    let menus = install_actions(application);
    let shell = Rc::new(RefCell::new(UiShell::new()));
    let editor = Rc::new(RefCell::new(None));
    let coordinator = Rc::new(RefCell::new(OperationCoordinator::new()));
    let syncing_buffer = Rc::new(Cell::new(false));
    let autosave_sequence = Rc::new(Cell::new(0));
    let preview_sequence = Rc::new(Cell::new(0));
    let render_state = Rc::new(RefCell::new(RenderState::new(0)));
    let pending_capture = Rc::new(RefCell::new(None));
    let (background_sender, background_receiver) = mpsc::channel();
    let window = ApplicationWindow::builder()
        .application(application)
        .title("Captee")
        .default_width(1280)
        .default_height(800)
        .build();

    let source_buffer = sourceview::Buffer::builder().highlight_matching_brackets(true).build();
    let source_view = sourceview::View::with_buffer(&source_buffer);
    source_view.set_show_line_numbers(true);
    source_view.set_monospace(true);
    source_view.set_hexpand(true);
    source_view.set_vexpand(true);
    source_view.set_tooltip_text(Some("Typst source editor"));

    let status = Label::new(Some("Ready. Create or open a project to begin."));
    status.set_xalign(0.0);
    status.set_margin_start(16);
    status.set_margin_end(16);
    status.set_margin_top(8);
    status.set_margin_bottom(8);
    status.set_tooltip_text(Some("Accessible operation status"));

    let project_label = Label::new(Some("No project open"));
    project_label.set_xalign(0.0);
    project_label.set_hexpand(true);
    let diagnostics_label = Label::new(Some("No diagnostics."));
    diagnostics_label.set_xalign(0.0);
    diagnostics_label.set_wrap(true);
    diagnostics_label.set_selectable(true);
    let preview_picture = gtk::Picture::new();
    preview_picture.set_hexpand(true);
    preview_picture.set_vexpand(true);
    preview_picture.set_can_shrink(true);
    let preview_status = Label::new(Some("Render a document to see its preview."));
    preview_status.set_xalign(0.0);
    preview_status.set_wrap(true);

    let home_new_button = Button::with_label("New project");
    home_new_button.add_css_class("suggested-action");
    let home_open_button = Button::with_label("Open project");
    home_open_button.add_css_class("suggested-action");
    let stack = Stack::builder().hexpand(true).vexpand(true).build();
    stack.add_named(&build_home(&home_new_button, &home_open_button), Some("home"));
    stack.add_named(
        &build_workspace(
            &source_view,
            &preview_picture,
            &preview_status,
            &diagnostics_label,
            &menus,
        ),
        Some("workspace"),
    );
    stack.set_visible_child_name("home");

    let project_ui = ProjectUi {
        window: window.downgrade(),
        shell: Rc::clone(&shell),
        status: status.clone(),
        stack: stack.clone(),
        source_buffer: source_buffer.clone(),
        project_label: project_label.clone(),
        diagnostics_label,
        preview_picture,
        preview_status,
        editor,
        coordinator,
        syncing_buffer,
        autosave_sequence,
        preview_sequence,
        render_state,
        pending_capture,
        background_sender,
        background_receiver: Rc::new(RefCell::new(background_receiver)),
    };

    let header = GtkBox::new(Orientation::Horizontal, 8);
    header.set_margin_top(8);
    header.set_margin_bottom(8);
    header.set_margin_start(12);
    header.set_margin_end(12);
    let title = Label::new(Some("Captee"));
    title.add_css_class("title-3");
    header.append(&title);
    header.append(&project_label);

    let root = GtkBox::new(Orientation::Vertical, 0);
    root.append(&header);
    root.append(&stack);
    root.append(&status);
    window.set_child(Some(&root));

    connect_ui_actions(&project_ui, application);
    connect_project_button(&home_new_button, true, &project_ui);
    connect_project_button(&home_open_button, false, &project_ui);
    connect_editor_buffer(&project_ui);
    connect_runtime_results(&project_ui);
    window.present();
}

fn build_home(new_button: &Button, open_button: &Button) -> GtkBox {
    let home = GtkBox::new(Orientation::Vertical, 12);
    home.set_halign(Align::Center);
    home.set_valign(Align::Center);
    home.set_margin_top(48);
    home.set_margin_bottom(48);
    home.set_margin_start(48);
    home.set_margin_end(48);

    let title = Label::new(Some("Welcome to Captee"));
    title.add_css_class("title-1");
    let description = Label::new(Some(
        "Create a new Typst workspace or open an existing project to start writing.",
    ));
    description.set_wrap(true);
    description.set_justify(gtk::Justification::Center);
    home.append(&title);
    home.append(&description);
    home.append(&Label::new(Some("Your projects stay local on this computer.")));
    let actions = GtkBox::new(Orientation::Horizontal, 8);
    actions.set_halign(Align::Center);
    actions.append(new_button);
    actions.append(open_button);
    home.append(&actions);
    home
}

#[derive(Clone)]
struct WorkspaceMenus {
    file: gio::Menu,
    edit: gio::Menu,
    capture: gio::Menu,
    view: gio::Menu,
}

fn build_workspace(
    source_view: &sourceview::View,
    preview_picture: &gtk::Picture,
    preview_status: &Label,
    diagnostics_label: &Label,
    menus: &WorkspaceMenus,
) -> GtkBox {
    let navigation = GtkBox::new(Orientation::Vertical, 12);
    navigation.set_margin_top(16);
    navigation.set_margin_bottom(16);
    navigation.set_margin_start(16);
    navigation.set_margin_end(16);
    navigation.set_width_request(220);
    navigation.append(&Label::new(Some("Project")));
    navigation.append(&Label::new(Some("Entry document")));
    navigation.append(&Label::new(Some("Images")));

    let editor_scroll =
        ScrolledWindow::builder().child(source_view).hexpand(true).vexpand(true).build();

    let preview = GtkBox::new(Orientation::Vertical, 12);
    preview.set_margin_top(16);
    preview.set_margin_bottom(16);
    preview.set_margin_start(16);
    preview.set_margin_end(16);
    preview.append(&Label::new(Some("Preview")));
    preview.append(preview_status);
    preview.append(
        &ScrolledWindow::builder()
            .child(preview_picture)
            .hexpand(true)
            .vexpand(true)
            .min_content_height(240)
            .build(),
    );
    let diagnostics_title = Label::new(Some("Diagnostics"));
    diagnostics_title.set_xalign(0.0);
    diagnostics_title.add_css_class("heading");
    preview.append(&diagnostics_title);
    preview.append(diagnostics_label);

    let editor_preview = Paned::new(Orientation::Horizontal);
    editor_preview.set_start_child(Some(&editor_scroll));
    editor_preview.set_end_child(Some(&preview));
    editor_preview.set_resize_start_child(true);
    editor_preview.set_shrink_start_child(false);

    let workspace = Paned::new(Orientation::Horizontal);
    workspace.set_start_child(Some(&navigation));
    workspace.set_end_child(Some(&editor_preview));
    workspace.set_resize_start_child(false);
    workspace.set_shrink_start_child(false);
    workspace.set_position(220);

    let menu_strip = GtkBox::new(Orientation::Horizontal, 4);
    menu_strip.set_halign(Align::Start);
    menu_strip.set_margin_start(12);
    menu_strip.set_margin_end(12);
    menu_strip.set_margin_top(8);
    menu_strip.set_margin_bottom(8);
    for (label, menu, tooltip) in [
        ("File", &menus.file, "Project and document actions"),
        ("Edit", &menus.edit, "Editing actions"),
        ("Capture", &menus.capture, "Capture actions"),
        ("View", &menus.view, "Preview and export actions"),
    ] {
        let button = MenuButton::new();
        button.set_label(label);
        button.set_menu_model(Some(menu));
        button.add_css_class("flat");
        button.set_tooltip_text(Some(tooltip));
        menu_strip.append(&button);
    }

    let root = GtkBox::new(Orientation::Vertical, 0);
    root.append(&menu_strip);
    root.append(&workspace);
    root
}

fn install_actions(application: &Application) -> WorkspaceMenus {
    let file = gio::Menu::new();
    file.append(Some("New project"), Some("app.new-project"));
    file.append(Some("Open project"), Some("app.open-project"));
    file.append(Some("Close project"), Some("app.close-project"));
    file.append(Some("Save"), Some("app.save"));
    let edit = gio::Menu::new();
    edit.append(Some("Format"), Some("app.format"));
    edit.append(Some("Find and Replace"), Some("app.find-replace"));
    edit.append(Some("Completion"), Some("app.completion"));
    edit.append(Some("Undo"), Some("app.undo"));
    edit.append(Some("Redo"), Some("app.redo"));
    let capture = gio::Menu::new();
    capture.append(Some("Capture"), Some("app.capture"));
    let view = gio::Menu::new();
    view.append(Some("Preview"), Some("app.preview"));
    view.append(Some("Export PDF"), Some("app.export"));

    for (name, accelerator) in [
        ("new-project", "<Primary>n"),
        ("open-project", "<Primary>o"),
        ("close-project", "<Primary>w"),
        ("save", "<Primary>s"),
        ("format", "<Primary><Shift>f"),
        ("find-replace", "<Primary>f"),
        ("completion", "<Primary>space"),
        ("undo", "<Primary>z"),
        ("redo", "<Primary><Shift>z"),
        ("capture", "<Primary><Shift>c"),
        ("preview", "<Primary>r"),
        ("export", "<Primary><Shift>e"),
    ] {
        let action = gio::SimpleAction::new(name, None);
        application.add_action(&action);
        application.set_accels_for_action(&format!("app.{name}"), &[accelerator]);
    }
    WorkspaceMenus { file, edit, capture, view }
}

fn connect_ui_actions(project_ui: &ProjectUi, application: &Application) {
    let action = application.lookup_action("capture").expect("installed capture action");
    let action = action.downcast::<gio::SimpleAction>().expect("simple action");
    let capture_ui = project_ui.clone();
    action.connect_activate(move |_, _| start_capture(&capture_ui));

    for (name, create) in [("new-project", true), ("open-project", false)] {
        let action = application.lookup_action(name).expect("installed project action");
        let action = action.downcast::<gio::SimpleAction>().expect("simple action");
        connect_project_action(&action, create, project_ui);
    }

    let action = application.lookup_action("close-project").expect("installed close action");
    let action = action.downcast::<gio::SimpleAction>().expect("simple action");
    let close_ui = project_ui.clone();
    action.connect_activate(move |_, _| {
        close_project(&close_ui);
    });

    for (name, redo) in [("undo", false), ("redo", true)] {
        let action = application.lookup_action(name).expect("installed edit action");
        let action = action.downcast::<gio::SimpleAction>().expect("simple action");
        let project_ui = project_ui.clone();
        action.connect_activate(move |_, _| undo_or_redo(&project_ui, redo));
    }

    let action = application.lookup_action("save").expect("installed save action");
    let action = action.downcast::<gio::SimpleAction>().expect("simple action");
    let save_ui = project_ui.clone();
    action.connect_activate(move |_, _| start_save(&save_ui));

    let action = application.lookup_action("format").expect("installed format action");
    let action = action.downcast::<gio::SimpleAction>().expect("simple action");
    let format_ui = project_ui.clone();
    action.connect_activate(move |_, _| start_format(&format_ui));

    let action = application.lookup_action("find-replace").expect("installed find action");
    let action = action.downcast::<gio::SimpleAction>().expect("simple action");
    let find_ui = project_ui.clone();
    action.connect_activate(move |_, _| show_find_replace_dialog(&find_ui));

    let action = application.lookup_action("completion").expect("installed completion action");
    let action = action.downcast::<gio::SimpleAction>().expect("simple action");
    let completion_ui = project_ui.clone();
    action.connect_activate(move |_, _| start_completion(&completion_ui));

    let action = application.lookup_action("preview").expect("installed preview action");
    let action = action.downcast::<gio::SimpleAction>().expect("simple action");
    let preview_ui = project_ui.clone();
    action.connect_activate(move |_, _| start_preview(&preview_ui));

    let action = application.lookup_action("export").expect("installed export action");
    let action = action.downcast::<gio::SimpleAction>().expect("simple action");
    let export_ui = project_ui.clone();
    action.connect_activate(move |_, _| show_export_dialog(&export_ui));
}

fn connect_project_button(button: &Button, create: bool, project_ui: &ProjectUi) {
    let project_ui = project_ui.clone();
    button.connect_clicked(move |_| {
        choose_project_action(create, &project_ui);
    });
}

fn connect_project_action(action: &gio::SimpleAction, create: bool, project_ui: &ProjectUi) {
    let project_ui = project_ui.clone();
    action.connect_activate(move |_, _| {
        choose_project_action(create, &project_ui);
    });
}

#[derive(Clone)]
struct ProjectUi {
    window: glib::WeakRef<ApplicationWindow>,
    shell: Rc<RefCell<UiShell>>,
    status: Label,
    stack: Stack,
    source_buffer: sourceview::Buffer,
    project_label: Label,
    diagnostics_label: Label,
    preview_picture: gtk::Picture,
    preview_status: Label,
    editor: Rc<RefCell<Option<EditorBridge>>>,
    coordinator: Rc<RefCell<OperationCoordinator<WorkspaceOperationResult>>>,
    syncing_buffer: Rc<Cell<bool>>,
    autosave_sequence: Rc<Cell<u64>>,
    preview_sequence: Rc<Cell<u64>>,
    render_state: Rc<RefCell<RenderState>>,
    pending_capture: Rc<RefCell<Option<CapturedImage>>>,
    background_sender: Sender<BackgroundResult>,
    background_receiver: Rc<RefCell<Receiver<BackgroundResult>>>,
}

impl ProjectUi {
    fn window(&self) -> Option<ApplicationWindow> {
        self.window.upgrade()
    }
}

fn connect_editor_buffer(project_ui: &ProjectUi) {
    let project_ui = project_ui.clone();
    let buffer = project_ui.source_buffer.clone();
    buffer.connect_changed(move |buffer| {
        if project_ui.syncing_buffer.get() {
            return;
        }
        let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
        let update = project_ui
            .editor
            .borrow_mut()
            .as_mut()
            .map(|editor| editor.update_from_buffer(text.as_str()));
        match update {
            Some(Ok(Some(state))) => apply_editor_state(&project_ui, &state, false),
            Some(Err(_)) => {
                project_ui.status.set_text("The editor produced an invalid text range.")
            }
            Some(Ok(None)) | None => {}
        }
    });
}

fn undo_or_redo(project_ui: &ProjectUi, redo: bool) {
    let state = project_ui.editor.borrow_mut().as_mut().and_then(|editor| {
        if redo {
            editor.redo()
        } else {
            editor.undo()
        }
    });
    if let Some(state) = state {
        apply_editor_state(project_ui, &state, true);
        project_ui.status.set_text(if redo { "Redo applied." } else { "Undo applied." });
    }
}

fn apply_editor_state(project_ui: &ProjectUi, state: &EditorState, update_buffer: bool) {
    if update_buffer {
        project_ui.syncing_buffer.set(true);
        project_ui.source_buffer.set_text(&state.text);
        project_ui.syncing_buffer.set(false);
    }
    if let Err(error) = project_ui.coordinator.borrow_mut().set_source_revision(state.revision) {
        project_ui.status.set_text(&format!("Error: {error}"));
        return;
    }
    project_ui.render_state.borrow_mut().set_source_revision(state.revision);
    project_ui.preview_status.set_text("Preview is out of date.");
    if let Err(error) = project_ui.shell.borrow_mut().dispatch(UiCommand::SetDirty(state.dirty)) {
        project_ui.status.set_text(&format!("Error: {error}"));
        return;
    }
    refresh_project_label(project_ui);
    schedule_autosave(project_ui, state);
    schedule_preview(project_ui, state);
}

fn schedule_preview(project_ui: &ProjectUi, state: &EditorState) {
    let sequence = project_ui.preview_sequence.get().saturating_add(1);
    project_ui.preview_sequence.set(sequence);
    if !project_ui.shell.borrow().snapshot().app.settings.preview.auto_render {
        return;
    }
    let revision = state.revision;
    let project_ui = project_ui.clone();
    glib::timeout_add_local_once(Duration::from_millis(600), move || {
        if project_ui.preview_sequence.get() != sequence || project_ui.window().is_none() {
            return;
        }
        let current = project_ui.coordinator.borrow().active_source();
        if current.as_ref().is_none_or(|source| source.revision() != revision)
            || project_ui.shell.borrow().snapshot().progress.is_some()
        {
            return;
        }
        start_preview(&project_ui);
    });
}

fn schedule_autosave(project_ui: &ProjectUi, state: &EditorState) {
    let sequence = project_ui.autosave_sequence.get().saturating_add(1);
    project_ui.autosave_sequence.set(sequence);
    if !state.dirty {
        return;
    }
    let state = state.clone();
    let project_ui = project_ui.clone();
    glib::timeout_add_local_once(Duration::from_millis(750), move || {
        if project_ui.autosave_sequence.get() != sequence || project_ui.window().is_none() {
            return;
        }
        let Some(source) = project_ui.coordinator.borrow().active_source() else {
            return;
        };
        if source.revision() != state.revision {
            return;
        }
        let snapshot = project_ui.shell.borrow().snapshot();
        let Some(project) = snapshot.app.project else {
            return;
        };
        let entry = {
            let editor = project_ui.editor.borrow();
            let Some(editor) = editor.as_ref() else {
                return;
            };
            editor.entry_document().to_path_buf()
        };
        let root = PathBuf::from(project.root);
        let sender = project_ui.background_sender.clone();
        let _ = thread::Builder::new().name("captee-autosave".to_owned()).spawn(move || {
            let result = ProjectDocumentPersistence::open(root, entry)
                .and_then(|persistence| persistence.autosave(state.revision, &state.text))
                .map_err(|error| error.to_string());
            let _ = sender.send(BackgroundResult::Autosave { source, result });
        });
    });
}

fn start_save(project_ui: &ProjectUi) {
    let snapshot = project_ui.shell.borrow().snapshot();
    let Some(project) = snapshot.app.project else {
        project_ui.status.set_text("Open a project before saving.");
        return;
    };
    let Some(editor) = project_ui.editor.borrow().as_ref().cloned() else {
        project_ui.status.set_text("No entry document is active.");
        return;
    };
    if !editor.state().dirty {
        project_ui.status.set_text("Document is already saved.");
        return;
    }
    if let Err(error) = project_ui.shell.borrow_mut().dispatch(UiCommand::Save) {
        project_ui.status.set_text(&format!("Error: {error}"));
        return;
    }
    let task = match project_ui.coordinator.borrow_mut().begin(OperationKind::Save, false) {
        Ok(task) => task,
        Err(error) => {
            let _ = project_ui
                .shell
                .borrow_mut()
                .dispatch(UiCommand::Fail { message: error.to_string() });
            project_ui.status.set_text(&format!("Error: {error}"));
            return;
        }
    };
    project_ui.status.set_text("Saving…");
    let root = PathBuf::from(project.root);
    let entry = editor.entry_document().to_path_buf();
    let mut document = editor.document_snapshot();
    let _ = thread::Builder::new().name("captee-save".to_owned()).spawn(move || {
        let result = ProjectDocumentPersistence::open(root, entry).and_then(|persistence| {
            document.save(&persistence)?;
            persistence.clear_autosave()?;
            Ok(document)
        });
        let outcome = match result {
            Ok(document) => OperationOutcome::Completed(WorkspaceOperationResult::Saved(document)),
            Err(error) => OperationOutcome::Failed(error.to_string()),
        };
        let _ = task.finish(outcome);
    });
}

fn start_format(project_ui: &ProjectUi) {
    let snapshot = project_ui.shell.borrow().snapshot();
    let Some(project) = snapshot.app.project else {
        project_ui.status.set_text("Open a project before formatting.");
        return;
    };
    let Some(source) = project_ui.editor.borrow().as_ref().map(EditorBridge::state) else {
        project_ui.status.set_text("No entry document is active.");
        return;
    };
    if let Err(error) = project_ui.shell.borrow_mut().dispatch(UiCommand::Format) {
        project_ui.status.set_text(&format!("Error: {error}"));
        return;
    }
    let task = match project_ui.coordinator.borrow_mut().begin(OperationKind::Format, true) {
        Ok(task) => task,
        Err(error) => {
            let _ = project_ui
                .shell
                .borrow_mut()
                .dispatch(UiCommand::Fail { message: error.to_string() });
            project_ui.status.set_text(&format!("Error: {error}"));
            return;
        }
    };
    project_ui.status.set_text("Formatting…");
    let cancellation = task.cancellation();
    let root = PathBuf::from(project.root);
    let _ = thread::Builder::new().name("captee-format".to_owned()).spawn(move || {
        let outcome = if cancellation.is_cancelled() {
            OperationOutcome::Cancelled
        } else {
            let formatter = TypstFormatter::new(TypstRunner::discover(), root);
            match formatter.format_with_diagnostics(&source.text) {
                Ok(_) if cancellation.is_cancelled() => OperationOutcome::Cancelled,
                Ok(formatted) => {
                    OperationOutcome::Completed(WorkspaceOperationResult::Formatted(formatted))
                }
                Err(error) => {
                    OperationOutcome::Completed(WorkspaceOperationResult::AuthoringFailure {
                        message: error.message,
                        diagnostics: error.diagnostics,
                    })
                }
            }
        };
        let _ = task.finish(outcome);
    });
}

fn start_completion(project_ui: &ProjectUi) {
    let Some(source) = project_ui.editor.borrow().as_ref().map(EditorBridge::state) else {
        project_ui.status.set_text("No entry document is active.");
        return;
    };
    let character_offset = project_ui.source_buffer.cursor_position().max(0) as usize;
    let cursor = byte_offset_for_character(&source.text, character_offset);
    if let Err(error) = project_ui.shell.borrow_mut().dispatch(UiCommand::Completion) {
        project_ui.status.set_text(&format!("Error: {error}"));
        return;
    }
    let task = match project_ui.coordinator.borrow_mut().begin(OperationKind::Completion, true) {
        Ok(task) => task,
        Err(error) => {
            let _ = project_ui
                .shell
                .borrow_mut()
                .dispatch(UiCommand::Fail { message: error.to_string() });
            project_ui.status.set_text(&format!("Error: {error}"));
            return;
        }
    };
    let cancellation = task.cancellation();
    project_ui.status.set_text("Finding completions…");
    let _ = thread::Builder::new().name("captee-completion".to_owned()).spawn(move || {
        let outcome = match request_completions(
            &TypstCompletionProvider,
            &source.text,
            cursor,
            &cancellation,
        ) {
            Ok(Operation::Completed(items)) => {
                OperationOutcome::Completed(WorkspaceOperationResult::Completions { items, cursor })
            }
            Ok(Operation::Cancelled) => OperationOutcome::Cancelled,
            Err(error) => match error {},
        };
        let _ = task.finish(outcome);
    });
}

fn start_preview(project_ui: &ProjectUi) {
    let snapshot = project_ui.shell.borrow().snapshot();
    let Some(project) = snapshot.app.project else {
        project_ui.status.set_text("Open a project before rendering a preview.");
        return;
    };
    let Some(source) = project_ui.editor.borrow().as_ref().map(EditorBridge::state) else {
        project_ui.status.set_text("No entry document is active.");
        return;
    };
    if let Err(error) = project_ui.shell.borrow_mut().dispatch(UiCommand::Preview) {
        project_ui.status.set_text(&format!("Error: {error}"));
        return;
    }
    let task = match project_ui.coordinator.borrow_mut().begin(OperationKind::Preview, true) {
        Ok(task) => task,
        Err(error) => {
            let _ = project_ui
                .shell
                .borrow_mut()
                .dispatch(UiCommand::Fail { message: error.to_string() });
            project_ui.status.set_text(&format!("Error: {error}"));
            return;
        }
    };
    project_ui.status.set_text("Rendering preview…");
    project_ui.preview_status.set_text("Rendering current source…");
    let root = PathBuf::from(project.root);
    let cancellation = task.cancellation();
    let _ = thread::Builder::new().name("captee-preview-result".to_owned()).spawn(move || {
        let compiler =
            AsyncPreviewCompiler::new(TypstPreviewCompiler::new(TypstRunner::discover(), root));
        let handle = compiler.submit(source.revision, source.text);
        let outcome = match handle.recv() {
            Ok(_) if cancellation.is_cancelled() => OperationOutcome::Cancelled,
            Ok(outcome) => OperationOutcome::Completed(WorkspaceOperationResult::Preview(outcome)),
            Err(error) => OperationOutcome::Failed(error.to_string()),
        };
        let _ = task.finish(outcome);
    });
}

fn start_capture(project_ui: &ProjectUi) {
    let snapshot = project_ui.shell.borrow().snapshot();
    if snapshot.app.project.is_none() {
        project_ui.status.set_text("Open a project before capturing an image.");
        return;
    }
    let settings = snapshot.app.settings.capture;
    if let Err(error) = project_ui.shell.borrow_mut().dispatch(UiCommand::Capture) {
        project_ui.status.set_text(&format!("Error: {error}"));
        return;
    }
    let task = match project_ui.coordinator.borrow_mut().begin(OperationKind::Capture, true) {
        Ok(task) => task,
        Err(error) => {
            let _ = project_ui
                .shell
                .borrow_mut()
                .dispatch(UiCommand::Fail { message: error.to_string() });
            project_ui.status.set_text(&format!("Error: {error}"));
            return;
        }
    };
    project_ui.status.set_text("Choose a screen, window, or region to capture…");
    let cancellation = task.cancellation();
    let _ = thread::Builder::new().name("captee-capture".to_owned()).spawn(move || {
        let selector = CaptureSelector::new(
            XdgPortalCapture::new(),
            GrimSlurpCapture::new(Duration::from_secs(120)),
            settings,
        );
        let result =
            if cancellation.is_cancelled() { CaptureResult::Cancelled } else { selector.capture() };
        let outcome = match result {
            CaptureResult::Completed(_) if cancellation.is_cancelled() => {
                OperationOutcome::Cancelled
            }
            CaptureResult::Completed(image) => {
                OperationOutcome::Completed(WorkspaceOperationResult::Captured(image))
            }
            CaptureResult::Cancelled => OperationOutcome::Cancelled,
            CaptureResult::Failed(error) => OperationOutcome::Failed(error.to_string()),
        };
        let _ = task.finish(outcome);
    });
}

#[allow(deprecated)]
fn show_export_dialog(project_ui: &ProjectUi) {
    let state = project_ui.render_state.borrow();
    let has_current_preview = state
        .last_successful_preview()
        .is_some_and(|preview| preview.revision == state.current_revision());
    drop(state);
    if !has_current_preview {
        project_ui.status.set_text("Render the current source successfully before exporting PDF.");
        return;
    }
    let Some(window) = project_ui.window() else {
        return;
    };
    let snapshot = project_ui.shell.borrow().snapshot();
    let project_name =
        snapshot.app.project.map(|project| project.name).unwrap_or_else(|| "document".to_owned());
    let dialog = gtk::FileChooserNative::builder()
        .title("Export PDF")
        .accept_label("Export")
        .cancel_label("Cancel")
        .action(gtk::FileChooserAction::Save)
        .transient_for(&window)
        .modal(true)
        .build();
    dialog.set_current_name(&format!("{project_name}.pdf"));
    let project_ui = project_ui.clone();
    dialog.run_async(move |dialog, response| {
        if response == ResponseType::Accept {
            if let Some(path) = dialog.file().and_then(|file| file.path()) {
                start_export(&project_ui, path);
            } else {
                project_ui.status.set_text("Choose a local filesystem destination.");
            }
        }
        dialog.destroy();
    });
}

fn start_export(project_ui: &ProjectUi, destination: PathBuf) {
    let state = project_ui.render_state.borrow().clone();
    if let Err(error) = project_ui.shell.borrow_mut().dispatch(UiCommand::Export) {
        project_ui.status.set_text(&format!("Error: {error}"));
        return;
    }
    let task = match project_ui.coordinator.borrow_mut().begin(OperationKind::Export, false) {
        Ok(task) => task,
        Err(error) => {
            let _ = project_ui
                .shell
                .borrow_mut()
                .dispatch(UiCommand::Fail { message: error.to_string() });
            project_ui.status.set_text(&format!("Error: {error}"));
            return;
        }
    };
    project_ui.status.set_text("Exporting PDF…");
    let _ = thread::Builder::new().name("captee-export".to_owned()).spawn(move || {
        let outcome = match export_pdf(&state, &destination) {
            Ok(()) => OperationOutcome::Completed(WorkspaceOperationResult::Exported(destination)),
            Err(error) => OperationOutcome::Failed(error.to_string()),
        };
        let _ = task.finish(outcome);
    });
}

fn byte_offset_for_character(source: &str, character_offset: usize) -> usize {
    source.char_indices().nth(character_offset).map(|(offset, _)| offset).unwrap_or(source.len())
}

fn show_find_replace_dialog(project_ui: &ProjectUi) {
    let Some(window) = project_ui.window() else {
        return;
    };
    if project_ui.editor.borrow().is_none() {
        project_ui.status.set_text("No entry document is active.");
        return;
    }
    let dialog =
        Dialog::builder().title("Find and Replace").transient_for(&window).modal(true).build();
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Replace all", ResponseType::Accept);
    dialog.set_default_response(ResponseType::Accept);
    let form = GtkBox::new(Orientation::Vertical, 8);
    form.set_margin_top(16);
    form.set_margin_bottom(16);
    form.set_margin_start(16);
    form.set_margin_end(16);
    let query = Entry::new();
    query.set_placeholder_text(Some("Literal text to find"));
    let replacement = Entry::new();
    replacement.set_placeholder_text(Some("Replacement text"));
    let error_label = Label::new(None);
    error_label.add_css_class("error");
    error_label.set_xalign(0.0);
    form.append(&query);
    form.append(&replacement);
    form.append(&error_label);
    dialog.content_area().append(&form);

    let project_ui = project_ui.clone();
    let query_for_response = query.clone();
    dialog.connect_response(move |dialog, response| {
        if response != ResponseType::Accept {
            dialog.close();
            return;
        }
        let query_text = query_for_response.text().to_string();
        if query_text.is_empty() {
            error_label.set_text("Enter text to find.");
            return;
        }
        if let Err(error) = project_ui.shell.borrow_mut().dispatch(UiCommand::FindReplace) {
            error_label.set_text(&error.to_string());
            return;
        }
        let source = project_ui.editor.borrow().as_ref().map(EditorBridge::state);
        let Some(source) = source else {
            let _ = project_ui
                .shell
                .borrow_mut()
                .dispatch(UiCommand::Fail { message: "No entry document is active.".to_owned() });
            dialog.close();
            return;
        };
        match replace_literal(&source.text, &query_text, &replacement.text(), true) {
            Ok(Operation::Completed(replaced)) => {
                let update =
                    project_ui.editor.borrow_mut().as_mut().and_then(|editor| {
                        editor.update_from_buffer(&replaced.text).ok().flatten()
                    });
                if let Some(state) = update {
                    apply_editor_state(&project_ui, &state, true);
                }
                let message = format!("Replaced {} match(es).", replaced.replacements);
                let _ = project_ui
                    .shell
                    .borrow_mut()
                    .dispatch(UiCommand::Complete { message: message.clone() });
                project_ui.status.set_text(&message);
                dialog.close();
            }
            Ok(Operation::Cancelled) => {
                let _ = project_ui.shell.borrow_mut().dispatch(UiCommand::Cancel);
                dialog.close();
            }
            Err(error) => {
                let message = format!("Replace failed: {error:?}");
                let _ = project_ui
                    .shell
                    .borrow_mut()
                    .dispatch(UiCommand::Fail { message: message.clone() });
                error_label.set_text(&message);
            }
        }
    });
    dialog.present();
    query.grab_focus();
}

fn connect_runtime_results(project_ui: &ProjectUi) {
    let project_ui = project_ui.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        if project_ui.window().is_none() {
            return glib::ControlFlow::Break;
        }
        while let Some(result) = project_ui.coordinator.borrow_mut().try_next_result() {
            apply_operation_result(&project_ui, result);
        }
        loop {
            let result = project_ui.background_receiver.borrow().try_recv();
            match result {
                Ok(result) => apply_background_result(&project_ui, result),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        glib::ControlFlow::Continue
    });
}

fn apply_operation_result(
    project_ui: &ProjectUi,
    disposition: ResultDisposition<WorkspaceOperationResult>,
) {
    match disposition {
        ResultDisposition::Current(result) => {
            let source_identity = result.context.source().clone();
            match result.outcome {
                OperationOutcome::Completed(WorkspaceOperationResult::Saved(document)) => {
                    let state = project_ui
                        .editor
                        .borrow_mut()
                        .as_mut()
                        .and_then(|editor| editor.apply_saved_document(document));
                    if let Some(state) = state {
                        let _ = project_ui
                            .shell
                            .borrow_mut()
                            .dispatch(UiCommand::Complete { message: "Document saved".to_owned() });
                        let _ = project_ui
                            .shell
                            .borrow_mut()
                            .dispatch(UiCommand::SetDirty(state.dirty));
                        project_ui.status.set_text("Document saved.");
                        refresh_project_label(project_ui);
                        schedule_autosave(project_ui, &state);
                    } else {
                        let message =
                            "Save completed for an older source revision; current edits remain unsaved.";
                        let _ = project_ui
                            .shell
                            .borrow_mut()
                            .dispatch(UiCommand::Warn { message: message.to_owned() });
                        project_ui.status.set_text(message);
                    }
                }
                OperationOutcome::Completed(WorkspaceOperationResult::Formatted(formatted)) => {
                    show_diagnostics(project_ui, &formatted.diagnostics);
                    let state = project_ui.editor.borrow_mut().as_mut().and_then(|editor| {
                        editor.update_from_buffer(&formatted.source).ok().flatten()
                    });
                    if let Some(state) = state {
                        apply_editor_state(project_ui, &state, true);
                    }
                    let _ = project_ui.shell.borrow_mut().dispatch(UiCommand::Complete {
                        message: "Formatting complete".to_owned(),
                    });
                    project_ui.status.set_text("Formatting complete.");
                }
                OperationOutcome::Completed(WorkspaceOperationResult::Completions {
                    items,
                    cursor,
                }) => {
                    let _ = project_ui
                        .shell
                        .borrow_mut()
                        .dispatch(UiCommand::Complete { message: "Completions ready".to_owned() });
                    show_completion_dialog(project_ui, source_identity, items, cursor);
                }
                OperationOutcome::Completed(WorkspaceOperationResult::Preview(outcome)) => {
                    let preview = outcome.result.as_ref().ok().map(|artifact| {
                        (artifact.first_page_png.clone(), artifact.diagnostics.clone())
                    });
                    let failure = outcome
                        .result
                        .as_ref()
                        .err()
                        .map(|error| (error.message.clone(), error.diagnostics.clone()));
                    let accepted = outcome.apply_to(&mut project_ui.render_state.borrow_mut());
                    if !accepted {
                        let message = "Preview result ignored because its revision is stale.";
                        let _ = project_ui
                            .shell
                            .borrow_mut()
                            .dispatch(UiCommand::Warn { message: message.to_owned() });
                        project_ui.status.set_text(message);
                    } else if let Some((png, diagnostics)) = preview {
                        show_diagnostics(project_ui, &diagnostics);
                        let bytes = glib::Bytes::from_owned(png);
                        match gtk::gdk::Texture::from_bytes(&bytes) {
                            Ok(texture) => {
                                project_ui.preview_picture.set_paintable(Some(&texture));
                                project_ui.preview_status.set_text("Showing current preview.");
                                let _ =
                                    project_ui.shell.borrow_mut().dispatch(UiCommand::Complete {
                                        message: "Preview rendered".to_owned(),
                                    });
                                project_ui.status.set_text("Preview rendered.");
                            }
                            Err(error) => {
                                let message = format!("Could not display preview image: {error}");
                                let _ = project_ui
                                    .shell
                                    .borrow_mut()
                                    .dispatch(UiCommand::Fail { message: message.clone() });
                                project_ui.preview_status.set_text(
                                    "Preview compiled, but its image could not be displayed.",
                                );
                                project_ui.status.set_text(&format!("Error: {message}"));
                            }
                        }
                    } else if let Some((message, diagnostics)) = failure {
                        show_diagnostics(project_ui, &diagnostics);
                        let _ = project_ui
                            .shell
                            .borrow_mut()
                            .dispatch(UiCommand::Fail { message: message.clone() });
                        project_ui
                            .preview_status
                            .set_text("Preview failed; the last valid preview is retained.");
                        project_ui.status.set_text(&format!("Preview error: {message}"));
                    }
                }
                OperationOutcome::Completed(WorkspaceOperationResult::Exported(path)) => {
                    let message = format!("PDF exported to {}", path.display());
                    let _ = project_ui
                        .shell
                        .borrow_mut()
                        .dispatch(UiCommand::Complete { message: message.clone() });
                    project_ui.status.set_text(&message);
                }
                OperationOutcome::Completed(WorkspaceOperationResult::Captured(image)) => {
                    *project_ui.pending_capture.borrow_mut() = Some(image);
                    let _ = project_ui
                        .shell
                        .borrow_mut()
                        .dispatch(UiCommand::Complete { message: "Capture ready".to_owned() });
                    project_ui.status.set_text("Capture ready for annotation.");
                }
                OperationOutcome::Completed(WorkspaceOperationResult::AuthoringFailure {
                    message,
                    diagnostics,
                }) => {
                    show_diagnostics(project_ui, &diagnostics);
                    let _ = project_ui
                        .shell
                        .borrow_mut()
                        .dispatch(UiCommand::Fail { message: message.clone() });
                    project_ui.status.set_text(&format!("Error: {message}"));
                }
                OperationOutcome::Cancelled => {
                    let _ = project_ui.shell.borrow_mut().dispatch(UiCommand::Cancel);
                    project_ui.status.set_text("Operation cancelled.");
                }
                OperationOutcome::Failed(message) => {
                    let _ = project_ui
                        .shell
                        .borrow_mut()
                        .dispatch(UiCommand::Fail { message: message.clone() });
                    project_ui.status.set_text(&format!("Error: {message}"));
                }
            }
        }
        ResultDisposition::Stale(_) => {
            let message = "Background result ignored because the project or source changed.";
            let _ = project_ui
                .shell
                .borrow_mut()
                .dispatch(UiCommand::Warn { message: message.to_owned() });
            project_ui.status.set_text(message);
        }
    }
    refresh_project_label(project_ui);
}

fn show_completion_dialog(
    project_ui: &ProjectUi,
    source_identity: SourceIdentity,
    items: Vec<CompletionItem>,
    cursor: usize,
) {
    if items.is_empty() {
        project_ui.status.set_text("No completions found.");
        return;
    }
    let Some(window) = project_ui.window() else {
        return;
    };
    let dialog =
        Dialog::builder().title("Insert completion").transient_for(&window).modal(true).build();
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Insert", ResponseType::Accept);
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    let choices = gtk::DropDown::from_strings(&labels);
    choices.set_margin_top(16);
    choices.set_margin_bottom(16);
    choices.set_margin_start(16);
    choices.set_margin_end(16);
    dialog.content_area().append(&choices);

    let project_ui = project_ui.clone();
    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Accept
            && project_ui.coordinator.borrow().active_source() == Some(source_identity.clone())
        {
            let selected = choices.selected() as usize;
            if let Some(item) = items.get(selected) {
                let state = project_ui.editor.borrow_mut().as_mut().and_then(|editor| {
                    editor.replace_range(cursor..cursor, &item.insert_text).ok()
                });
                if let Some(state) = state {
                    apply_editor_state(&project_ui, &state, true);
                    project_ui.status.set_text(&format!("Inserted {}.", item.label));
                }
            }
        } else if response == ResponseType::Accept {
            project_ui.status.set_text("Completion ignored because the source changed.");
        }
        dialog.close();
    });
    dialog.present();
}

fn show_diagnostics(project_ui: &ProjectUi, diagnostics: &[Diagnostic]) {
    if diagnostics.is_empty() {
        project_ui.diagnostics_label.set_text("No diagnostics.");
        return;
    }
    let text = diagnostics
        .iter()
        .take(20)
        .map(|diagnostic| {
            let severity = match diagnostic.severity {
                DiagnosticSeverity::Error => "Error",
                DiagnosticSeverity::Warning => "Warning",
                DiagnosticSeverity::Info => "Info",
            };
            match &diagnostic.span {
                Some(span) => format!(
                    "{severity}: {}:{}:{}: {}",
                    span.path, span.line, span.column, diagnostic.message
                ),
                None => format!("{severity}: {}", diagnostic.message),
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    project_ui.diagnostics_label.set_text(&text);
}

fn apply_background_result(project_ui: &ProjectUi, background: BackgroundResult) {
    match background {
        BackgroundResult::Autosave { source, result } => {
            if project_ui.coordinator.borrow().active_source().as_ref() != Some(&source) {
                return;
            }
            match result {
                Ok(()) => project_ui.status.set_text("Draft autosaved locally."),
                Err(message) => {
                    project_ui.status.set_text(&format!("Autosave warning: {message}"));
                }
            }
        }
        BackgroundResult::RecentProject { project, result } => {
            let is_current = project_ui
                .coordinator
                .borrow()
                .active_source()
                .is_some_and(|source| source.project() == &project);
            if is_current {
                if let Err(message) = result {
                    project_ui.status.set_text(&format!("Recent-project warning: {message}"));
                }
            }
        }
    }
}

fn refresh_project_label(project_ui: &ProjectUi) {
    let snapshot = project_ui.shell.borrow().snapshot();
    let Some(project) = snapshot.app.project else {
        project_ui.project_label.set_text("No project open");
        return;
    };
    let modified = if snapshot.app.dirty { " • Modified" } else { "" };
    project_ui.project_label.set_text(&format!("{} · {}{modified}", project.name, project.root));
}

fn choose_project_action(create: bool, project_ui: &ProjectUi) {
    if create {
        show_new_project_dialog(project_ui);
    } else {
        show_open_project_dialog(project_ui);
    }
}

fn show_new_project_dialog(project_ui: &ProjectUi) {
    let Some(window) = project_ui.window() else {
        return;
    };
    let dialog =
        Dialog::builder().title("New Captee project").transient_for(&window).modal(true).build();
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Create", ResponseType::Accept);
    dialog.set_default_response(ResponseType::Accept);

    let content = dialog.content_area();
    let form = GtkBox::new(Orientation::Vertical, 10);
    form.set_margin_top(16);
    form.set_margin_bottom(16);
    form.set_margin_start(16);
    form.set_margin_end(16);

    let name_label = Label::new(Some("Project name"));
    name_label.set_xalign(0.0);
    let name_entry = Entry::new();
    name_entry.set_placeholder_text(Some("e.g. Meeting notes"));
    name_entry.set_activates_default(true);
    name_entry.set_hexpand(true);

    let location_label = Label::new(Some("Parent location"));
    location_label.set_xalign(0.0);
    let location = Rc::new(RefCell::new(None::<PathBuf>));
    let selected_location = Label::new(Some("No location selected"));
    selected_location.set_xalign(0.0);
    selected_location.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    let choose_location = Button::with_label("Choose location…");
    let location_for_button = Rc::clone(&location);
    let selected_location_for_button = selected_location.clone();
    let window = window.clone();
    choose_location.connect_clicked(move |_| {
        let chooser = gtk::FileChooserNative::builder()
            .title("Choose parent location")
            .accept_label("Select folder")
            .cancel_label("Cancel")
            .action(gtk::FileChooserAction::SelectFolder)
            .transient_for(&window)
            .modal(true)
            .build();
        let location_for_chooser = Rc::clone(&location_for_button);
        let selected_location_for_chooser = selected_location_for_button.clone();
        chooser.run_async(move |chooser, response| {
            if response == ResponseType::Accept {
                if let Some(file) = chooser.file() {
                    if let Some(path) = file.path() {
                        selected_location_for_chooser.set_text(&path.display().to_string());
                        *location_for_chooser.borrow_mut() = Some(path);
                    }
                }
            }
            chooser.destroy();
        });
    });

    let dialog_status = Label::new(None);
    dialog_status.set_xalign(0.0);
    dialog_status.add_css_class("error");
    form.append(&name_label);
    form.append(&name_entry);
    form.append(&location_label);
    form.append(&choose_location);
    form.append(&selected_location);
    form.append(&dialog_status);
    content.append(&form);

    let project_ui_for_response = project_ui.clone();
    let location_for_response = Rc::clone(&location);
    let dialog_status_for_response = dialog_status.clone();
    let name_entry_for_response = name_entry.clone();
    dialog.connect_response(move |dialog, response| {
        if response != ResponseType::Accept {
            dialog.close();
            return;
        }

        let name = name_entry_for_response.text().trim().to_owned();
        let Some(parent) = location_for_response.borrow().clone() else {
            dialog_status_for_response.set_text("Choose a parent location first.");
            return;
        };
        if let Err(error) = validate_project_name(&name) {
            dialog_status_for_response.set_text(&error);
            return;
        }

        match create_loaded_project(&parent, &name) {
            Ok(project) => {
                if open_loaded_project(
                    project,
                    true,
                    &parent.join(name.trim()),
                    &project_ui_for_response,
                ) {
                    dialog.close();
                }
            }
            Err(error) => {
                dialog_status_for_response.set_text(&format!("Could not create project: {error}"))
            }
        }
    });
    dialog.present();
    name_entry.grab_focus();
}

#[allow(deprecated)]
fn show_open_project_dialog(project_ui: &ProjectUi) {
    let Some(window) = project_ui.window() else {
        return;
    };
    let dialog = gtk::FileChooserNative::builder()
        .title("Open project folder")
        .accept_label("Open")
        .cancel_label("Cancel")
        .action(gtk::FileChooserAction::SelectFolder)
        .transient_for(&window)
        .modal(true)
        .build();
    let project_ui = project_ui.clone();
    dialog.run_async(move |dialog, response| {
        if response != gtk::ResponseType::Accept {
            dialog.destroy();
            return;
        }
        let Some(file) = dialog.file() else {
            dialog.destroy();
            return;
        };
        let Some(path) = file.path() else {
            project_ui.status.set_text("The selected location is not a local filesystem path.");
            dialog.destroy();
            return;
        };
        match load_project(&path) {
            Ok(project) => {
                open_loaded_project(project, false, &path, &project_ui);
            }
            Err(error) => project_ui.status.set_text(&format!("Could not open project: {error}")),
        }
        dialog.destroy();
    });
}

struct LoadedProject {
    session: ProjectSession,
    settings: captee_core::ProjectSettings,
    source: String,
    recovery: Option<RecoveryDraft>,
    recovery_warning: Option<String>,
}

struct RecoveryDraft {
    revision: u64,
    source: String,
}

fn validate_project_name(name: &str) -> Result<(), String> {
    let path = Path::new(name);
    if name.trim().is_empty()
        || !path.is_relative()
        || path == Path::new(".")
        || path == Path::new("..")
        || path.components().count() != 1
    {
        return Err("Enter a simple project name without path separators.".into());
    }
    Ok(())
}

fn create_loaded_project(parent: &Path, name: &str) -> Result<LoadedProject, String> {
    validate_project_name(name)?;
    let path = parent.join(name.trim());
    let config = ProjectConfig::new(name.trim(), "main.typ").map_err(|error| error.to_string())?;
    create_project(&path, config).map_err(|error| error.to_string())?;
    load_project(&path)
}

fn load_project(path: &Path) -> Result<LoadedProject, String> {
    let workspace = open_project(path).map_err(|error| error.to_string())?;
    let source_path = workspace
        .paths
        .require_file(&workspace.config.entry_document)
        .map_err(|error| error.to_string())?;
    let source = fs::read_to_string(&source_path)
        .map_err(|error| format!("could not read {}: {error}", source_path.display()))?;
    let (recovery, recovery_warning) = match AutosaveStore::new(path.join(AUTOSAVE_FILE)).recover()
    {
        Ok(Some(snapshot)) => match recovery_draft(snapshot, &source) {
            Ok(recovery) => (recovery, None),
            Err(error) => (None, Some(error)),
        },
        Ok(None) => (None, None),
        Err(error) => (None, Some(format!("Could not read the autosave: {error}"))),
    };
    Ok(LoadedProject {
        session: ProjectSession::new(
            path.to_string_lossy(),
            workspace.config.name.clone(),
            workspace.config.entry_document.clone(),
        ),
        settings: workspace.config.settings,
        source,
        recovery,
        recovery_warning,
    })
}

fn recovery_draft(
    snapshot: AutosaveSnapshot,
    disk_source: &str,
) -> Result<Option<RecoveryDraft>, String> {
    let source = String::from_utf8(snapshot.contents)
        .map_err(|_| "The autosave is not valid UTF-8 and was not applied.".to_owned())?;
    if source == disk_source {
        return Ok(None);
    }
    Ok(Some(RecoveryDraft { revision: snapshot.revision, source }))
}

fn open_loaded_project(
    project: LoadedProject,
    created: bool,
    path: &Path,
    project_ui: &ProjectUi,
) -> bool {
    if project_ui.shell.borrow().snapshot().progress.is_some() {
        project_ui
            .status
            .set_text("Wait for the active operation to finish before opening another project.");
        return false;
    }
    let project_identity = match project_ui.coordinator.borrow_mut().activate_project(path) {
        Ok(identity) => identity,
        Err(error) => {
            project_ui.status.set_text(&format!("Error: {error}"));
            return false;
        }
    };
    let result = project_ui.shell.borrow_mut().dispatch(UiCommand::OpenProject {
        session: project.session.clone(),
        settings: project.settings,
    });
    match result {
        Ok(()) => {
            project_ui.autosave_sequence.set(project_ui.autosave_sequence.get().saturating_add(1));
            *project_ui.editor.borrow_mut() = Some(EditorBridge::new(
                project.session.entry_document.clone(),
                project.source.clone(),
            ));
            project_ui.syncing_buffer.set(true);
            project_ui.source_buffer.set_text(&project.source);
            project_ui.syncing_buffer.set(false);
            project_ui.diagnostics_label.set_text("No diagnostics.");
            *project_ui.render_state.borrow_mut() = RenderState::new(0);
            *project_ui.pending_capture.borrow_mut() = None;
            project_ui.preview_picture.set_paintable(Option::<&gtk::gdk::Texture>::None);
            project_ui.preview_status.set_text("Preview has not been rendered yet.");
            refresh_project_label(project_ui);
            project_ui.stack.set_visible_child_name("workspace");
            project_ui.status.set_text(if created {
                "Project created. Ready to edit."
            } else {
                "Project opened. Ready to edit."
            });
            record_recent_project(project_ui, project_identity);
            if let Some(warning) = project.recovery_warning {
                project_ui.status.set_text(&format!("Recovery warning: {warning}"));
            }
            if let Some(recovery) = project.recovery {
                show_recovery_dialog(project_ui, recovery);
            } else if let Some(state) = project_ui.editor.borrow().as_ref().map(EditorBridge::state)
            {
                schedule_preview(project_ui, &state);
            }
            true
        }
        Err(error) => {
            project_ui.coordinator.borrow_mut().deactivate_project();
            project_ui.status.set_text(&format!("Error: {error}"));
            false
        }
    }
}

fn record_recent_project(project_ui: &ProjectUi, project: ProjectIdentity) {
    let store_path = glib::user_data_dir().join("captee/recent-projects.json");
    let project_path = project.root().to_string_lossy().into_owned();
    let sender = project_ui.background_sender.clone();
    let _ = thread::Builder::new().name("captee-recent-project".to_owned()).spawn(move || {
        let result = RecentProjectStore::new(store_path)
            .record(project_path)
            .map(|_| ())
            .map_err(|error| error.to_string());
        let _ = sender.send(BackgroundResult::RecentProject { project, result });
    });
}

fn show_recovery_dialog(project_ui: &ProjectUi, recovery: RecoveryDraft) {
    let Some(window) = project_ui.window() else {
        return;
    };
    let dialog = Dialog::builder()
        .title("Recover autosaved draft?")
        .transient_for(&window)
        .modal(true)
        .build();
    dialog.add_button("Keep disk version", ResponseType::Cancel);
    dialog.add_button("Recover draft", ResponseType::Accept);
    dialog.set_default_response(ResponseType::Accept);
    let message = Label::new(Some(&format!(
        "A different autosaved draft (revision {}) was found. Recover it as unsaved editor content?",
        recovery.revision
    )));
    message.set_wrap(true);
    message.set_margin_top(16);
    message.set_margin_bottom(16);
    message.set_margin_start(16);
    message.set_margin_end(16);
    dialog.content_area().append(&message);

    let project_ui = project_ui.clone();
    let source_identity = project_ui.coordinator.borrow().active_source();
    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Accept
            && project_ui.coordinator.borrow().active_source() == source_identity
        {
            let state = project_ui
                .editor
                .borrow_mut()
                .as_mut()
                .and_then(|editor| editor.update_from_buffer(&recovery.source).ok().flatten());
            if let Some(state) = state {
                apply_editor_state(&project_ui, &state, true);
                project_ui.status.set_text("Autosaved draft recovered. Save to keep it.");
            }
        }
        if let Some(state) = project_ui.editor.borrow().as_ref().map(EditorBridge::state) {
            schedule_preview(&project_ui, &state);
        }
        dialog.close();
    });
    dialog.present();
}

fn close_project(project_ui: &ProjectUi) {
    match project_ui.shell.borrow_mut().dispatch(UiCommand::CloseProject) {
        Ok(()) => {
            project_ui.autosave_sequence.set(project_ui.autosave_sequence.get().saturating_add(1));
            project_ui.coordinator.borrow_mut().deactivate_project();
            *project_ui.editor.borrow_mut() = None;
            project_ui.stack.set_visible_child_name("home");
            project_ui.syncing_buffer.set(true);
            project_ui.source_buffer.set_text("");
            project_ui.syncing_buffer.set(false);
            project_ui.diagnostics_label.set_text("No diagnostics.");
            *project_ui.render_state.borrow_mut() = RenderState::new(0);
            *project_ui.pending_capture.borrow_mut() = None;
            project_ui.preview_picture.set_paintable(Option::<&gtk::gdk::Texture>::None);
            project_ui.preview_status.set_text("Render a document to see its preview.");
            refresh_project_label(project_ui);
            project_ui.status.set_text("Project closed. Create or open a project to begin.");
        }
        Err(error) => project_ui.status.set_text(&format!("Error: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{byte_offset_for_character, recovery_draft, validate_project_name};
    use captee_platform::AutosaveSnapshot;

    #[test]
    fn project_name_accepts_a_single_directory_name() {
        assert!(validate_project_name("Meeting notes").is_ok());
    }

    #[test]
    fn project_name_rejects_paths_and_empty_input() {
        assert!(validate_project_name("").is_err());
        assert!(validate_project_name("nested/project").is_err());
        assert!(validate_project_name("..").is_err());
    }

    #[test]
    fn recovery_is_offered_only_for_different_valid_utf8_source() {
        let same = AutosaveSnapshot { revision: 2, contents: b"disk".to_vec() };
        assert!(recovery_draft(same, "disk").expect("valid autosave").is_none());

        let changed = AutosaveSnapshot { revision: 3, contents: b"draft".to_vec() };
        let recovery = recovery_draft(changed, "disk").expect("valid autosave").expect("draft");
        assert_eq!(recovery.revision, 3);
        assert_eq!(recovery.source, "draft");

        let invalid = AutosaveSnapshot { revision: 4, contents: vec![0xff] };
        assert!(recovery_draft(invalid, "disk").is_err());
    }

    #[test]
    fn character_offsets_are_converted_to_utf8_byte_offsets() {
        assert_eq!(byte_offset_for_character("aéz", 0), 0);
        assert_eq!(byte_offset_for_character("aéz", 1), 1);
        assert_eq!(byte_offset_for_character("aéz", 2), 3);
        assert_eq!(byte_offset_for_character("aéz", 99), 4);
    }
}
