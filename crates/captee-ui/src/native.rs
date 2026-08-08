use crate::annotation_bridge::AnnotationDraft;
use crate::capture_review::CaptureReview;
use crate::editor_assistance::{completion_edit, typst_completions};
use crate::editor_bridge::{EditorBridge, EditorInsertionBridge, EditorState};
use crate::operation::{
    drain_ready_results, OperationCoordinator, OperationOutcome, ProjectIdentity,
    ResultDisposition, SourceIdentity,
};
use crate::{
    initial_editor_preview_position, initial_navigation_position, status_bar_action_label,
    UiCommand, UiShell,
};
use captee_core::{
    replace_literal, request_completions, Activity, AnnotatedImage, Annotation, AnnotationBackend,
    AnnotationResult, CaptureBackend, CaptureResult, CapturedImage, CompletionItem, EditorInserter,
    InsertionResult, KeybindingSettings, Operation, OperationKind, ProjectConfig, ProjectSession,
    ProjectSettings, RenderState, SelectionGeometry, SourceDocument,
};
use captee_platform::{
    confirm_and_trash, create_project, create_project_item, current_capture_origin,
    current_desktop_prefers_fallback_capture, export_pdf, list_project_tree, move_project_item,
    open_project, place_capture_review_window, register_capture_shortcut, rename_project_item,
    save_project_settings, AssetStore, AsyncPreviewCompiler, AutosaveSnapshot, AutosaveStore,
    CaptureOrigin, CaptureSelector, FormattedSource, GlobalShortcutEvent, GrimSlurpCapture,
    PngAnnotationBackend, PreviewOutcome, ProjectDocumentPersistence, ProjectTreeEntry,
    RecentProjectStore, SavedAsset, TrashBackend, TrashError, TypstCompletionProvider,
    TypstFormatter, TypstPreviewCompiler, TypstRunner, XdgPortalCapture, AUTOSAVE_FILE,
};
use glib::value::ToValue;
use glib::variant::ToVariant;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, CheckButton, Dialog, DragSource,
    DropTarget, Entry, GestureClick, Label, ListBox, ListBoxRow, MenuButton, Orientation, Paned,
    Popover, ResponseType, ScrolledWindow, Spinner, Stack, ToggleButton, Window,
};
use gtk4 as gtk;
use sourceview::prelude::*;
use sourceview5 as sourceview;
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

const APPLICATION_ID: &str = "com.nightlyshelf.captee";

#[derive(Debug)]
enum WorkspaceOperationResult {
    Saved { document: SourceDocument, formatted: bool },
    Formatted(FormattedSource),
    Completions { items: Vec<CompletionItem>, cursor: usize },
    AuthoringFailure { message: String },
    Preview(PreviewOutcome),
    Exported(PathBuf),
    Captured(CapturedImage),
    CaptureStored { asset: SavedAsset, annotation: String, before_image: bool },
    SettingsSaved(ProjectSettings),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewScale {
    FitPage,
    FitPageWidth,
    Percent(u16),
}

const STATUS_BAR_VISIBLE_BY_DEFAULT: bool = false;

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
    let pending_annotation = Rc::new(RefCell::new(None));
    let global_capture_receiver = register_capture_shortcut("CTRL+SHIFT+C");
    let project_tree = ListBox::new();
    project_tree.set_selection_mode(gtk::SelectionMode::None);
    project_tree.set_hexpand(true);
    project_tree.set_vexpand(true);
    let project_name_label = Label::new(Some("Project"));
    let project_panel_title = Label::new(Some("Project"));
    let (background_sender, background_receiver) = mpsc::channel();
    let window = ApplicationWindow::builder()
        .application(application)
        .title("Captee")
        .default_width(1280)
        .default_height(800)
        .build();
    let style = gtk::CssProvider::new();
    style.load_from_data(
        ".capture-review-window, .capture-review-surface { background-color: transparent; }\
         .capture-review-window, .capture-review-window.background,\
         .capture-review-surface, .capture-review-surface.background {\
           background-color: transparent; background-image: none; box-shadow: none;\
         }\
         .capture-review-panel { background-color: #202124; border-radius: 4px; }\
         .capture-context { color: #9aa0a6; }\
         .typst-editor, .typst-editor.view, .typst-editor text {\
           background-color: #202124; color: #e8eaed; caret-color: #ffffff;\
         }\
         .typst-editor gutter, .typst-editor gutter.left {\
           background-color: #292a2d; color: #9aa0a6;\
         }\
         .typst-editor border { background-color: #3c4043; }\
         .workspace-header { background-color: #0a0705; }\
         .compact-menu-button, .compact-menu-button > button {\
           margin: 0; padding: 0 2px; min-height: 0; min-width: 0; font-size: 12px;\
         }\
         .compact-menu-text { font-size: 12px; }\
         .project-tree-action { padding: 0; min-height: 22px; min-width: 22px; }",
    );
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &style,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    let source_buffer = sourceview::Buffer::builder().highlight_matching_brackets(true).build();
    configure_typst_buffer(&source_buffer);
    let source_view = sourceview::View::with_buffer(&source_buffer);
    source_view.set_show_line_numbers(true);
    source_view.set_monospace(true);
    source_view.set_hexpand(true);
    source_view.set_vexpand(true);
    source_view.add_css_class("typst-editor");
    source_view.set_tooltip_text(Some("Typst source editor"));

    let status = Label::new(Some("Ready. Create or open a project to begin."));
    status.set_xalign(0.0);
    status.set_tooltip_text(Some("Accessible operation status"));
    status.set_hexpand(true);
    let progress_spinner = Spinner::new();
    progress_spinner.set_visible(false);
    progress_spinner.set_tooltip_text(Some("An operation is running"));
    let cancel_button = Button::with_label("Cancel");
    cancel_button.set_visible(false);
    cancel_button.set_tooltip_text(Some("Cancel the active operation"));
    let status_row = GtkBox::new(Orientation::Horizontal, 8);
    status_row.set_margin_start(16);
    status_row.set_margin_end(16);
    status_row.set_margin_top(8);
    status_row.set_margin_bottom(8);
    status_row.append(&progress_spinner);
    status_row.append(&status);
    status_row.append(&cancel_button);

    let project_label = Label::new(Some("Captee"));
    project_label.set_xalign(0.0);
    project_label.set_hexpand(true);
    let preview_pages = GtkBox::new(Orientation::Vertical, 16);
    preview_pages.set_hexpand(true);
    preview_pages.set_vexpand(true);
    preview_pages.set_halign(Align::Center);
    let preview_scale = gtk::DropDown::from_strings(&[
        "Fit page",
        "Fit page width",
        "50%",
        "75%",
        "100%",
        "125%",
        "150%",
        "200%",
        "300%",
    ]);
    preview_scale.set_selected(0);
    preview_scale.set_tooltip_text(Some("Choose how preview pages are scaled"));
    let preview_scroller = ScrolledWindow::builder()
        .child(&preview_pages)
        .hexpand(true)
        .vexpand(true)
        .min_content_height(240)
        .build();

    let home_new_button = Button::with_label("New project");
    home_new_button.add_css_class("suggested-action");
    let home_open_button = Button::with_label("Open project");
    home_open_button.add_css_class("suggested-action");
    let stack = Stack::builder().hexpand(true).vexpand(true).build();
    stack.add_named(&build_home(&home_new_button, &home_open_button), Some("home"));
    stack.add_named(
        &build_workspace(
            &source_view,
            PreviewWidgets { scroller: &preview_scroller, scale: &preview_scale },
            &project_tree,
            &project_name_label,
            &project_panel_title,
        ),
        Some("workspace"),
    );
    stack.set_visible_child_name("home");

    let project_ui = ProjectUi {
        application: application.downgrade(),
        window: window.downgrade(),
        shell: Rc::clone(&shell),
        status: status.clone(),
        stack: stack.clone(),
        source_buffer: source_buffer.clone(),
        source_view: source_view.clone(),
        project_label: project_label.clone(),
        project_tree: project_tree.clone(),
        project_name_label: project_name_label.clone(),
        project_panel_title: project_panel_title.clone(),
        workspace_overlay: gtk::Overlay::new(),
        expanded_tree: Rc::new(RefCell::new(BTreeSet::new())),
        tree_initialized: Rc::new(Cell::new(false)),
        status_row: status_row.clone(),
        status_bar_item: menus.status_bar_item.clone(),
        preview_pages,
        preview_scroller,
        preview_scale,
        preview_scale_mode: Rc::new(Cell::new(PreviewScale::FitPage)),
        progress_spinner,
        cancel_button: cancel_button.clone(),
        editor,
        coordinator,
        syncing_buffer,
        autosave_sequence,
        preview_sequence,
        render_state,
        pending_capture,
        pending_annotation,
        capture_origin: Rc::new(RefCell::new(None)),
        global_capture_receiver: Rc::new(RefCell::new(global_capture_receiver)),
        background_sender,
        background_receiver: Rc::new(RefCell::new(background_receiver)),
    };

    let header = build_menu_header(&menus, &project_name_label, &project_label);
    header.set_visible(false);
    let header_visibility = header.clone();
    stack.connect_visible_child_name_notify(move |stack| {
        let visible = stack.visible_child_name().is_some_and(|name| name == "workspace");
        header_visibility.set_visible(visible);
    });

    let root = GtkBox::new(Orientation::Vertical, 0);
    root.append(&header);
    root.append(&stack);
    status_row.set_visible(STATUS_BAR_VISIBLE_BY_DEFAULT);
    root.append(&status_row);
    project_ui.workspace_overlay.set_child(Some(&root));
    window.set_child(Some(&project_ui.workspace_overlay));

    connect_ui_actions(&project_ui, application);
    connect_project_button(&home_new_button, true, &project_ui);
    connect_project_button(&home_open_button, false, &project_ui);
    connect_editor_buffer(&project_ui);
    connect_preview_scale(&project_ui);
    connect_project_tree(&project_ui);
    connect_runtime_results(&project_ui);
    connect_global_capture_shortcut(&project_ui);
    connect_cancel_button(&cancel_button, &project_ui);
    sync_operation_feedback(&project_ui);
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
    status_bar_item: gio::MenuItem,
}

struct PreviewWidgets<'a> {
    scroller: &'a ScrolledWindow,
    scale: &'a gtk::DropDown,
}

fn build_menu_header(
    menus: &WorkspaceMenus,
    project_name: &Label,
    project_label: &Label,
) -> gtk::CenterBox {
    let header = gtk::CenterBox::new();
    header.add_css_class("workspace-header");
    header.set_margin_top(4);
    header.set_margin_bottom(4);
    header.set_margin_start(4);
    header.set_margin_end(4);
    header.set_valign(Align::Start);
    let menu_box = GtkBox::new(Orientation::Horizontal, 0);
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
        button.add_css_class("compact-menu-button");
        button.set_tooltip_text(Some(tooltip));
        button.set_size_request(-1, 20);
        button.set_valign(Align::Start);
        menu_box.append(&button);
    }
    let metadata = GtkBox::new(Orientation::Horizontal, 4);
    metadata.set_halign(Align::Center);
    metadata.set_hexpand(true);
    project_name.set_xalign(0.0);
    project_name.set_hexpand(false);
    project_name.set_margin_start(8);
    project_name.set_margin_end(4);
    project_name.set_valign(Align::Center);
    metadata.append(project_name);
    project_label.set_xalign(0.0);
    project_label.set_hexpand(false);
    project_label.set_margin_start(4);
    project_label.set_valign(Align::Center);
    metadata.append(project_label);
    header.set_start_widget(Some(&menu_box));
    header.set_center_widget(Some(&metadata));
    header
}

fn build_workspace(
    source_view: &sourceview::View,
    preview_widgets: PreviewWidgets<'_>,
    project_tree: &ListBox,
    project_name_label: &Label,
    project_panel_title: &Label,
) -> GtkBox {
    let PreviewWidgets { scroller: preview_scroller, scale: preview_scale } = preview_widgets;
    let navigation = GtkBox::new(Orientation::Vertical, 12);
    navigation.set_margin_top(4);
    navigation.set_margin_bottom(4);
    navigation.set_margin_start(4);
    navigation.set_margin_end(4);
    navigation.set_spacing(4);
    navigation.set_width_request(0);
    let tree_header = GtkBox::new(Orientation::Horizontal, 2);
    tree_header.set_valign(Align::Center);
    let project_name = project_name_label.clone();
    project_name.set_xalign(0.0);
    project_name.set_hexpand(false);
    project_name.set_ellipsize(gtk::pango::EllipsizeMode::End);
    project_name.set_max_width_chars(16);
    project_name.set_valign(Align::Center);
    project_name.add_css_class("compact-menu-text");
    project_panel_title.set_xalign(0.0);
    project_panel_title.set_hexpand(true);
    project_panel_title.set_margin_start(27);
    project_panel_title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    project_panel_title.set_max_width_chars(16);
    project_panel_title.set_valign(Align::Center);
    project_panel_title.add_css_class("compact-menu-text");
    let add_file = Button::from_icon_name("document-new-symbolic");
    add_file.set_action_name(Some("app.new-file"));
    add_file.add_css_class("flat");
    add_file.add_css_class("project-tree-action");
    add_file.set_size_request(22, 22);
    add_file.set_valign(Align::Center);
    add_file.set_tooltip_text(Some("Add file"));
    let add_folder = Button::from_icon_name("folder-new-symbolic");
    add_folder.set_action_name(Some("app.new-folder"));
    add_folder.add_css_class("flat");
    add_folder.add_css_class("project-tree-action");
    add_folder.set_size_request(22, 22);
    add_folder.set_valign(Align::Center);
    add_folder.set_tooltip_text(Some("Add folder"));
    tree_header.append(project_panel_title);
    let tree_spacer = GtkBox::new(Orientation::Horizontal, 0);
    tree_spacer.set_hexpand(true);
    tree_header.append(&tree_spacer);
    tree_header.append(&add_file);
    tree_header.append(&add_folder);
    navigation.append(&tree_header);
    navigation.append(project_tree);

    let editor_scroll =
        ScrolledWindow::builder().child(source_view).hexpand(true).vexpand(true).build();

    let preview = GtkBox::new(Orientation::Vertical, 12);
    preview.set_margin_top(16);
    preview.set_margin_bottom(16);
    preview.set_margin_start(16);
    preview.set_margin_end(16);
    preview.append(preview_scroller);
    let scale_row = GtkBox::new(Orientation::Horizontal, 8);
    let scale_label = Label::new(Some("Scale"));
    scale_label.set_xalign(0.0);
    scale_row.append(&scale_label);
    scale_row.append(preview_scale);
    preview.append(&scale_row);
    let editor_preview = Paned::new(Orientation::Horizontal);
    editor_preview.set_start_child(Some(&editor_scroll));
    editor_preview.set_end_child(Some(&preview));
    editor_preview.set_resize_start_child(true);
    editor_preview.set_shrink_start_child(false);
    editor_preview.set_resize_end_child(true);
    editor_preview.set_wide_handle(false);
    editor_preview.connect_map(|paned| {
        let paned = paned.clone();
        glib::idle_add_local_once(move || {
            let width = paned.width();
            if width > 0 {
                paned.set_position(initial_editor_preview_position(width));
            }
        });
    });

    let workspace = Paned::new(Orientation::Horizontal);
    workspace.set_start_child(Some(&navigation));
    workspace.set_end_child(Some(&editor_preview));
    workspace.set_resize_start_child(true);
    workspace.set_shrink_start_child(true);
    workspace.set_resize_end_child(true);
    workspace.set_wide_handle(false);
    workspace.connect_map(|paned| {
        let paned = paned.clone();
        glib::idle_add_local_once(move || {
            let width = paned.width();
            if width > 0 {
                paned.set_position(initial_navigation_position(width));
            }
        });
    });

    let root = GtkBox::new(Orientation::Vertical, 0);
    root.append(&workspace);
    root
}

fn install_actions(application: &Application) -> WorkspaceMenus {
    let file = gio::Menu::new();
    append_menu_action(&file, "New project", "app.new-project");
    append_menu_action(&file, "Open project", "app.open-project");
    append_menu_action(&file, "Close project", "app.close-project");
    append_menu_action(&file, "Save", "app.save");
    let edit = gio::Menu::new();
    append_menu_action(&edit, "Format", "app.format");
    append_menu_action(&edit, "Find and Replace", "app.find-replace");
    append_menu_action(&edit, "Completion", "app.completion");
    append_menu_action(&edit, "Undo", "app.undo");
    append_menu_action(&edit, "Redo", "app.redo");
    let capture = gio::Menu::new();
    append_menu_action(&capture, "Capture", "app.capture");
    let view = gio::Menu::new();
    append_menu_action(&view, "Preview", "app.preview");
    append_menu_action(&view, "Export PDF", "app.export");
    let status_bar_item = gio::MenuItem::new(Some("Show status bar"), Some("app.status-bar"));
    status_bar_item.set_attribute_value("accel", Some(&"".to_variant()));
    view.append_item(&status_bar_item);
    append_menu_action(&view, "Settings", "app.settings");

    for (name, accelerator) in [
        ("new-project", "<Primary>n"),
        ("open-project", "<Primary>o"),
        ("new-file", ""),
        ("new-folder", ""),
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
        ("status-bar", ""),
        ("settings", "<Primary>comma"),
    ] {
        let action = gio::SimpleAction::new(name, None);
        application.add_action(&action);
        application.set_accels_for_action(&format!("app.{name}"), &[accelerator]);
    }
    WorkspaceMenus { file, edit, capture, view, status_bar_item }
}

fn append_menu_action(menu: &gio::Menu, label: &str, action: &str) {
    let item = gio::MenuItem::new(Some(label), Some(action));
    item.set_attribute_value("accel", Some(&"".to_variant()));
    menu.append_item(&item);
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
    for (name, directory) in [("new-file", false), ("new-folder", true)] {
        let action = application.lookup_action(name).expect("installed tree action");
        let action = action.downcast::<gio::SimpleAction>().expect("simple action");
        let tree_ui = project_ui.clone();
        action.connect_activate(move |_, _| {
            show_create_project_item_dialog(&tree_ui, Path::new(""), directory);
        });
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

    let action = application.lookup_action("settings").expect("installed settings action");
    let action = action.downcast::<gio::SimpleAction>().expect("simple action");
    let settings_ui = project_ui.clone();
    action.connect_activate(move |_, _| show_settings_dialog(&settings_ui));

    let action = application.lookup_action("status-bar").expect("installed status action");
    let action = action.downcast::<gio::SimpleAction>().expect("simple action");
    let status_ui = project_ui.clone();
    let status_item = project_ui.status_bar_item.clone();
    action.connect_activate(move |_, _| {
        let visible = !status_ui.status_row.is_visible();
        status_ui.status_row.set_visible(visible);
        status_item.set_label(Some(status_bar_action_label(visible)));
    });
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
    application: glib::WeakRef<Application>,
    window: glib::WeakRef<ApplicationWindow>,
    shell: Rc<RefCell<UiShell>>,
    status: Label,
    stack: Stack,
    source_buffer: sourceview::Buffer,
    source_view: sourceview::View,
    project_label: Label,
    project_tree: ListBox,
    project_name_label: Label,
    project_panel_title: Label,
    workspace_overlay: gtk::Overlay,
    expanded_tree: Rc<RefCell<BTreeSet<PathBuf>>>,
    tree_initialized: Rc<Cell<bool>>,
    status_row: GtkBox,
    status_bar_item: gio::MenuItem,
    preview_pages: GtkBox,
    preview_scroller: ScrolledWindow,
    preview_scale: gtk::DropDown,
    preview_scale_mode: Rc<Cell<PreviewScale>>,
    progress_spinner: Spinner,
    cancel_button: Button,
    editor: Rc<RefCell<Option<EditorBridge>>>,
    coordinator: Rc<RefCell<OperationCoordinator<WorkspaceOperationResult>>>,
    syncing_buffer: Rc<Cell<bool>>,
    autosave_sequence: Rc<Cell<u64>>,
    preview_sequence: Rc<Cell<u64>>,
    render_state: Rc<RefCell<RenderState>>,
    pending_capture: Rc<RefCell<Option<CapturedImage>>>,
    pending_annotation: Rc<RefCell<Option<AnnotatedImage>>>,
    capture_origin: Rc<RefCell<Option<CaptureOrigin>>>,
    global_capture_receiver: Rc<RefCell<Receiver<GlobalShortcutEvent>>>,
    background_sender: Sender<BackgroundResult>,
    background_receiver: Rc<RefCell<Receiver<BackgroundResult>>>,
}

impl ProjectUi {
    fn application(&self) -> Option<Application> {
        self.application.upgrade()
    }

    fn window(&self) -> Option<ApplicationWindow> {
        self.window.upgrade()
    }
}

fn connect_cancel_button(button: &Button, project_ui: &ProjectUi) {
    let project_ui = project_ui.clone();
    button.connect_clicked(move |_| match project_ui.coordinator.borrow_mut().cancel_active() {
        Ok(_) => {
            let _ = project_ui.shell.borrow_mut().dispatch(UiCommand::Cancel);
            project_ui.status.set_text("Operation cancelled; late results will be ignored.");
            sync_operation_feedback(&project_ui);
        }
        Err(error) => project_ui.status.set_text(&format!("Could not cancel operation: {error}")),
    });
}

fn connect_global_capture_shortcut(project_ui: &ProjectUi) {
    let project_ui = project_ui.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        loop {
            let event = project_ui.global_capture_receiver.borrow_mut().try_recv();
            match event {
                Ok(GlobalShortcutEvent::Activated) => start_capture(&project_ui),
                Ok(GlobalShortcutEvent::Failed(error)) => {
                    project_ui
                        .status
                        .set_text(&format!("Global capture shortcut unavailable: {error}"));
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return glib::ControlFlow::Break,
            }
        }
        glib::ControlFlow::Continue
    });
}

fn sync_operation_feedback(project_ui: &ProjectUi) {
    let snapshot = project_ui.shell.borrow().snapshot();
    let interaction = crate::interaction_state(&snapshot);
    let progress = snapshot.progress.as_ref();
    let busy = interaction.busy;
    if busy {
        project_ui.progress_spinner.start();
    } else {
        project_ui.progress_spinner.stop();
    }
    project_ui.progress_spinner.set_visible(busy);
    let cancellable = interaction.cancellable;
    project_ui.cancel_button.set_visible(cancellable);
    project_ui.cancel_button.set_sensitive(cancellable);
    if let Some(progress) = progress {
        project_ui
            .cancel_button
            .set_tooltip_text(Some(&format!("Cancel {}", progress.label.to_lowercase())));
    }
    project_ui.source_view.set_editable(interaction.editor_enabled);

    for class in ["success", "warning", "error"] {
        project_ui.status.remove_css_class(class);
    }
    match snapshot.app.activity {
        Activity::Succeeded(_) => project_ui.status.add_css_class("success"),
        Activity::Warning(_) => project_ui.status.add_css_class("warning"),
        Activity::Failed(_) => project_ui.status.add_css_class("error"),
        Activity::Idle | Activity::Running { .. } => {}
    }
    let status_text = project_ui.status.text();
    project_ui.status.set_tooltip_text(Some(&format!("Status: {status_text}")));

    let Some(application) = project_ui.application() else {
        return;
    };
    for name in ["new-project", "open-project", "new-file", "new-folder"] {
        set_action_enabled(&application, name, interaction.project_actions_enabled);
    }
    for name in [
        "close-project",
        "save",
        "format",
        "find-replace",
        "completion",
        "undo",
        "redo",
        "capture",
        "preview",
        "export",
        "settings",
    ] {
        set_action_enabled(&application, name, interaction.workspace_actions_enabled);
    }
}

fn set_action_enabled(application: &Application, name: &str, enabled: bool) {
    if let Some(action) = application
        .lookup_action(name)
        .and_then(|action| action.downcast::<gio::SimpleAction>().ok())
    {
        action.set_enabled(enabled);
    }
}

fn connect_project_tree(project_ui: &ProjectUi) {
    let target = DropTarget::new(String::static_type(), gtk::gdk::DragAction::MOVE);
    let project_ui_for_drop = project_ui.clone();
    target.connect_drop(move |_, value, _, _| {
        let Ok(source) = value.get::<String>() else {
            return false;
        };
        let Some(project) = project_ui_for_drop.shell.borrow().snapshot().app.project else {
            return false;
        };
        match move_project_item(&project.root, source, Path::new("")) {
            Ok(_) => {
                refresh_project_tree(&project_ui_for_drop);
                project_ui_for_drop.status.set_text("Project item moved to the project root.");
                true
            }
            Err(error) => {
                project_ui_for_drop.status.set_text(&format!("Could not move item: {error}"));
                false
            }
        }
    });
    project_ui.project_tree.add_controller(target);
    refresh_project_tree(project_ui);
}

fn refresh_project_tree(project_ui: &ProjectUi) {
    while let Some(child) = project_ui.project_tree.first_child() {
        project_ui.project_tree.remove(&child);
    }
    let Some(project) = project_ui.shell.borrow().snapshot().app.project else {
        return;
    };
    let entries = match list_project_tree(&project.root) {
        Ok(entries) => entries,
        Err(error) => {
            project_ui.status.set_text(&format!("Could not read project tree: {error}"));
            return;
        }
    };
    if !project_ui.tree_initialized.get() {
        project_ui.expanded_tree.borrow_mut().extend(
            entries
                .iter()
                .filter(|entry| entry.is_directory)
                .map(|entry| entry.relative_path.clone()),
        );
        project_ui.tree_initialized.set(true);
    }
    for entry in entries {
        if tree_entry_visible(&project_ui.expanded_tree.borrow(), &entry.relative_path) {
            append_project_tree_row(project_ui, entry);
        }
    }
}

fn tree_entry_visible(expanded: &BTreeSet<PathBuf>, path: &Path) -> bool {
    let mut ancestor = PathBuf::new();
    let components = path.components().collect::<Vec<_>>();
    for component in components.iter().take(components.len().saturating_sub(1)) {
        ancestor.push(component.as_os_str());
        if !expanded.contains(&ancestor) {
            return false;
        }
    }
    true
}

fn toggle_tree_path(project_ui: &ProjectUi, path: &Path) {
    let mut expanded = project_ui.expanded_tree.borrow_mut();
    if !expanded.insert(path.to_path_buf()) {
        expanded.remove(path);
    }
    drop(expanded);
    refresh_project_tree(project_ui);
}

fn append_project_tree_row(project_ui: &ProjectUi, entry: ProjectTreeEntry) {
    let row = ListBoxRow::new();
    row.set_activatable(true);
    row.set_selectable(false);
    let depth = entry.relative_path.components().count().saturating_sub(1);
    let name = entry.relative_path.file_name().and_then(|name| name.to_str()).unwrap_or("item");
    let expanded = project_ui.expanded_tree.borrow().contains(&entry.relative_path);
    let icon_name = if entry.is_directory {
        if expanded {
            "folder-open-symbolic"
        } else {
            "folder-symbolic"
        }
    } else if Path::new(name).extension().is_some_and(|extension| extension == "typ") {
        "text-x-generic-symbolic"
    } else if Path::new(name).extension().is_some_and(|extension| {
        matches!(extension.to_str(), Some("png" | "jpg" | "jpeg" | "svg" | "webp"))
    }) {
        "image-x-generic-symbolic"
    } else {
        "text-x-generic-symbolic"
    };
    let label = Label::new(Some(name));
    label.set_xalign(0.0);
    label.set_hexpand(true);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    label.set_max_width_chars(24);
    label.set_margin_start(4);
    label.set_margin_top(3);
    label.set_margin_bottom(3);
    let content = GtkBox::new(Orientation::Horizontal, 6);
    content.set_margin_start(8 + (depth as i32 * 16));
    content.append(&gtk::Image::from_icon_name(icon_name));
    content.append(&label);
    row.set_child(Some(&content));

    let is_directory = entry.is_directory;
    let path_for_click = entry.relative_path.clone();
    let project_ui_for_click = project_ui.clone();
    let row_for_click = row.clone();
    let click = GestureClick::new();
    click.set_button(1);
    click.connect_pressed(move |_, n_press, _, _| {
        if n_press == 3 {
            begin_inline_rename(&project_ui_for_click, &row_for_click, &path_for_click);
        } else if n_press == 1 {
            if is_directory {
                toggle_tree_path(&project_ui_for_click, &path_for_click);
            } else {
                open_project_tree_file(&project_ui_for_click, &path_for_click);
            }
        }
    });
    row.add_controller(click);

    let source_path = entry.relative_path.to_string_lossy().into_owned();
    let drag = DragSource::new();
    drag.set_actions(gtk::gdk::DragAction::MOVE);
    drag.connect_prepare(move |_, _, _| {
        Some(gtk::gdk::ContentProvider::for_value(&source_path.to_value()))
    });
    row.add_controller(drag);

    if is_directory {
        let destination = entry.relative_path.clone();
        let project_ui_for_drop = project_ui.clone();
        let target = DropTarget::new(String::static_type(), gtk::gdk::DragAction::MOVE);
        target.connect_drop(move |_, value, _, _| {
            let Ok(source) = value.get::<String>() else {
                return false;
            };
            let Some(project) = project_ui_for_drop.shell.borrow().snapshot().app.project else {
                return false;
            };
            match move_project_item(&project.root, source, &destination) {
                Ok(_) => {
                    refresh_project_tree(&project_ui_for_drop);
                    project_ui_for_drop.status.set_text("Project item moved.");
                    true
                }
                Err(error) => {
                    project_ui_for_drop.status.set_text(&format!("Could not move item: {error}"));
                    false
                }
            }
        });
        row.add_controller(target);
    }

    let context_path = entry.relative_path.clone();
    let context_ui = project_ui.clone();
    let row_for_context = row.clone();
    let gesture = GestureClick::new();
    gesture.set_button(3);
    gesture.connect_pressed(move |_, _, _, _| {
        show_project_tree_context_menu(&context_ui, &row_for_context, &context_path);
    });
    row.add_controller(gesture);
    project_ui.project_tree.append(&row);
}

fn open_project_tree_file(project_ui: &ProjectUi, relative: &Path) {
    let snapshot = project_ui.shell.borrow().snapshot();
    let Some(project) = snapshot.app.project else {
        return;
    };
    let path = PathBuf::from(&project.root).join(relative);
    match fs::read_to_string(&path) {
        Ok(source) => {
            *project_ui.editor.borrow_mut() = Some(EditorBridge::new(relative, source.clone()));
            project_ui.syncing_buffer.set(true);
            project_ui.source_buffer.set_text(&source);
            project_ui.syncing_buffer.set(false);
            project_ui.status.set_text(&format!("Opened {}.", relative.display()));
            if let Some(state) = project_ui.editor.borrow().as_ref().map(EditorBridge::state) {
                let _ = project_ui.coordinator.borrow_mut().set_source_revision(state.revision);
                project_ui.render_state.borrow_mut().set_source_revision(state.revision);
                schedule_preview(project_ui, &state);
            }
        }
        Err(error) => project_ui.status.set_text(&format!("Could not open file: {error}")),
    }
}

fn show_project_tree_context_menu(project_ui: &ProjectUi, row: &ListBoxRow, path: &Path) {
    let popover = Popover::new();
    popover.set_parent(row);
    let actions = GtkBox::new(Orientation::Vertical, 2);
    for (label, action) in
        [("New file", 0_u8), ("New folder", 1), ("Rename", 2), ("Move", 3), ("Delete", 4)]
    {
        let button = Button::with_label(label);
        button.set_halign(Align::Fill);
        let project_ui = project_ui.clone();
        let path = path.to_path_buf();
        let popover_for_action = popover.clone();
        let row_for_action = row.clone();
        button.connect_clicked(move |_| {
            popover_for_action.popdown();
            match action {
                0 => show_create_project_item_dialog(&project_ui, &path, false),
                1 => show_create_project_item_dialog(&project_ui, &path, true),
                2 => begin_inline_rename(&project_ui, &row_for_action, &path),
                3 => show_move_project_item_dialog(&project_ui, &path),
                _ => show_delete_project_item_dialog(&project_ui, &path),
            }
        });
        actions.append(&button);
    }
    popover.set_child(Some(&actions));
    popover.popup();
}

fn show_create_project_item_dialog(project_ui: &ProjectUi, selected: &Path, directory: bool) {
    let Some(window) = project_ui.window() else {
        return;
    };
    let dialog = Dialog::builder()
        .title(if directory { "New folder" } else { "New file" })
        .transient_for(&window)
        .modal(true)
        .default_width(360)
        .build();
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Create", ResponseType::Accept);
    dialog.set_default_response(ResponseType::Accept);
    let form = GtkBox::new(Orientation::Vertical, 8);
    form.set_margin_top(12);
    form.set_margin_bottom(12);
    form.set_margin_start(12);
    form.set_margin_end(12);
    let entry = Entry::new();
    entry.set_placeholder_text(Some(if directory { "Folder name" } else { "File name" }));
    entry.set_activates_default(true);
    let error = Label::new(None);
    error.add_css_class("error");
    error.set_xalign(0.0);
    form.append(&entry);
    form.append(&error);
    dialog.content_area().append(&form);
    let project_ui = project_ui.clone();
    let selected = selected.to_path_buf();
    let entry_for_response = entry.clone();
    dialog.connect_response(move |dialog, response| {
        if response != ResponseType::Accept {
            dialog.close();
            return;
        }
        let snapshot = project_ui.shell.borrow().snapshot();
        let Some(project) = snapshot.app.project else {
            dialog.close();
            return;
        };
        let selected_absolute = PathBuf::from(&project.root).join(&selected);
        let parent = if selected_absolute.is_dir() {
            selected.clone()
        } else {
            selected.parent().unwrap_or(Path::new("")).to_path_buf()
        };
        match create_project_item(
            &project.root,
            parent,
            entry_for_response.text().trim(),
            directory,
        ) {
            Ok(_) => {
                refresh_project_tree(&project_ui);
                project_ui.status.set_text("Project item created.");
                dialog.close();
            }
            Err(item_error) => error.set_text(&item_error.to_string()),
        }
    });
    dialog.present();
    entry.grab_focus();
}

fn show_move_project_item_dialog(project_ui: &ProjectUi, source: &Path) {
    let Some(window) = project_ui.window() else {
        return;
    };
    let dialog = Dialog::builder()
        .title("Move project item")
        .transient_for(&window)
        .modal(true)
        .default_width(420)
        .build();
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Move", ResponseType::Accept);
    dialog.set_default_response(ResponseType::Accept);
    let form = GtkBox::new(Orientation::Vertical, 8);
    form.set_margin_top(12);
    form.set_margin_bottom(12);
    form.set_margin_start(12);
    form.set_margin_end(12);
    let entry = Entry::new();
    entry.set_placeholder_text(Some("Destination folder; empty = project root"));
    entry.set_activates_default(true);
    let error = Label::new(None);
    error.add_css_class("error");
    error.set_xalign(0.0);
    form.append(&entry);
    form.append(&error);
    dialog.content_area().append(&form);
    let project_ui = project_ui.clone();
    let source = source.to_path_buf();
    let entry_for_response = entry.clone();
    dialog.connect_response(move |dialog, response| {
        if response != ResponseType::Accept {
            dialog.close();
            return;
        }
        let snapshot = project_ui.shell.borrow().snapshot();
        let Some(project) = snapshot.app.project else {
            dialog.close();
            return;
        };
        match move_project_item(&project.root, &source, entry_for_response.text().trim()) {
            Ok(_) => {
                refresh_project_tree(&project_ui);
                project_ui.status.set_text("Project item moved.");
                dialog.close();
            }
            Err(item_error) => error.set_text(&item_error.to_string()),
        }
    });
    dialog.present();
    entry.grab_focus();
}

fn begin_inline_rename(project_ui: &ProjectUi, row: &ListBoxRow, source: &Path) {
    let name = source.file_name().and_then(|name| name.to_str()).unwrap_or_default();
    let entry = Entry::new();
    entry.set_text(name);
    entry.set_hexpand(true);
    entry.set_margin_start(8);
    entry.set_margin_end(8);
    entry.set_margin_top(2);
    entry.set_margin_bottom(2);
    entry.set_tooltip_text(Some("Rename project item; press Enter to confirm or Escape to cancel"));
    row.set_child(Some(&entry));
    entry.grab_focus();
    entry.select_region(0, -1);

    let finished = Rc::new(Cell::new(false));
    let commit = Rc::new({
        let project_ui = project_ui.clone();
        let row = row.clone();
        let source = source.to_path_buf();
        let entry = entry.clone();
        let finished = finished.clone();
        move |cancelled: bool| {
            if finished.get() {
                return;
            }
            if cancelled {
                finished.set(true);
                refresh_project_tree(&project_ui);
                return;
            }
            let snapshot = project_ui.shell.borrow().snapshot();
            let Some(project) = snapshot.app.project else {
                finished.set(true);
                refresh_project_tree(&project_ui);
                return;
            };
            match rename_project_item(&project.root, &source, entry.text().trim()) {
                Ok(_) => {
                    finished.set(true);
                    refresh_project_tree(&project_ui);
                    project_ui.status.set_text("Project item renamed.");
                }
                Err(error) => {
                    project_ui.status.set_text(&format!("Could not rename item: {error}"));
                    row.set_child(Some(&entry));
                    entry.grab_focus();
                }
            }
        }
    });

    let commit_for_activate = commit.clone();
    entry.connect_activate(move |_| commit_for_activate(false));

    let key = gtk::EventControllerKey::new();
    let commit_for_key = commit.clone();
    key.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            commit_for_key(true);
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    entry.add_controller(key);

    let focus = gtk::EventControllerFocus::new();
    let commit_for_focus = commit.clone();
    focus.connect_leave(move |_| commit_for_focus(false));
    entry.add_controller(focus);
}

struct GioTrashBackend;

impl TrashBackend for GioTrashBackend {
    fn move_to_trash(&self, path: &Path) -> Result<(), TrashError> {
        gio::File::for_path(path)
            .trash(gio::Cancellable::NONE)
            .map_err(|error| TrashError::Backend(error.to_string()))
    }
}

fn show_delete_project_item_dialog(project_ui: &ProjectUi, path: &Path) {
    let Some(window) = project_ui.window() else {
        return;
    };
    let dialog = Dialog::builder()
        .title("Delete project item?")
        .transient_for(&window)
        .modal(true)
        .default_width(380)
        .build();
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Delete", ResponseType::Accept);
    dialog.set_default_response(ResponseType::Cancel);
    let message = Label::new(Some(&format!(
        "Move {} to the project trash? This action removes it from the workspace.",
        path.display()
    )));
    message.set_wrap(true);
    message.set_margin_top(12);
    message.set_margin_bottom(12);
    message.set_margin_start(12);
    message.set_margin_end(12);
    dialog.content_area().append(&message);
    let project_ui = project_ui.clone();
    let path = path.to_path_buf();
    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Accept {
            let snapshot = project_ui.shell.borrow().snapshot();
            if let Some(project) = snapshot.app.project {
                let absolute = PathBuf::from(&project.root).join(&path);
                match confirm_and_trash(&GioTrashBackend, &absolute, true) {
                    Ok(_) => {
                        refresh_project_tree(&project_ui);
                        project_ui.status.set_text("Project item moved to trash.");
                    }
                    Err(error) => {
                        project_ui.status.set_text(&format!("Could not delete item: {error}"))
                    }
                }
            }
        }
        dialog.close();
    });
    dialog.present();
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
    let preview_was_cancelled = project_ui
        .shell
        .borrow()
        .snapshot()
        .progress
        .as_ref()
        .is_some_and(|progress| progress.operation == OperationKind::Preview);
    if preview_was_cancelled {
        let _ = project_ui.shell.borrow_mut().dispatch(UiCommand::Cancel);
    }
    project_ui.render_state.borrow_mut().set_source_revision(state.revision);
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
    let format_on_save = snapshot.app.settings.formatting.format_on_save;
    let _ = thread::Builder::new().name("captee-save".to_owned()).spawn(move || {
        let result: Result<SourceDocument, String> = (|| {
            if format_on_save {
                let formatted = TypstFormatter::new(TypstRunner::discover(), &root)
                    .format_with_diagnostics(document.text())
                    .map_err(|error| error.to_string())?;
                if formatted.source != document.text() {
                    let previous_len = document.text().len();
                    document.replace(0..previous_len, &formatted.source).map_err(|error| {
                        format!("formatted source could not be applied: {error:?}")
                    })?;
                }
            }
            let persistence =
                ProjectDocumentPersistence::open(root, entry).map_err(|error| error.to_string())?;
            document.save(&persistence).map_err(|error| error.to_string())?;
            persistence.clear_autosave().map_err(|error| error.to_string())?;
            Ok(document)
        })();
        let outcome = match result {
            Ok(document) => OperationOutcome::Completed(WorkspaceOperationResult::Saved {
                document,
                formatted: format_on_save,
            }),
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
    *project_ui.capture_origin.borrow_mut() = current_capture_origin();
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
        )
        .with_fallback_first(current_desktop_prefers_fallback_capture());
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

#[allow(dead_code)]
fn show_annotation_dialog(project_ui: &ProjectUi, image: CapturedImage) -> Result<(), String> {
    let Some(window) = project_ui.window() else {
        return Err("The workspace window is no longer available.".to_owned());
    };
    let picture = gtk::Picture::new();
    picture.set_can_shrink(true);
    picture.set_hexpand(true);
    picture.set_vexpand(true);
    let (image_width, image_height) = display_annotation_image(&picture, image.bytes())?;

    let dialog = Dialog::builder()
        .title("Annotate capture")
        .transient_for(&window)
        .modal(true)
        .default_width(960)
        .default_height(720)
        .build();
    dialog.add_button("Cancel", ResponseType::Cancel);
    let confirm_button = dialog.add_button("Use image", ResponseType::Accept);
    dialog.set_default_response(ResponseType::Accept);

    let content = GtkBox::new(Orientation::Vertical, 10);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.append(
        &ScrolledWindow::builder()
            .child(&picture)
            .hexpand(true)
            .vexpand(true)
            .min_content_height(420)
            .build(),
    );

    let controls = gtk::Grid::builder().column_spacing(8).row_spacing(8).build();
    let tool_label = Label::new(Some("Mark"));
    tool_label.set_xalign(0.0);
    let tool = gtk::DropDown::from_strings(&["Pointer", "Rectangle", "Text"]);
    tool.set_tooltip_text(Some("Choose the kind of annotation to add"));
    let x = gtk::SpinButton::with_range(0.0, f64::from(image_width.saturating_sub(1)), 1.0);
    let y = gtk::SpinButton::with_range(0.0, f64::from(image_height.saturating_sub(1)), 1.0);
    let width = gtk::SpinButton::with_range(1.0, f64::from(image_width.max(1)), 1.0);
    let height = gtk::SpinButton::with_range(1.0, f64::from(image_height.max(1)), 1.0);
    width.set_value(f64::from(image_width.clamp(1, 160)));
    height.set_value(f64::from(image_height.clamp(1, 100)));
    let text_entry = Entry::new();
    text_entry.set_placeholder_text(Some("Annotation text"));
    text_entry.set_hexpand(true);
    for control in [&x, &y, &width, &height] {
        control.set_numeric(true);
    }
    controls.attach(&tool_label, 0, 0, 1, 1);
    controls.attach(&tool, 1, 0, 1, 1);
    controls.attach(&Label::new(Some("X")), 2, 0, 1, 1);
    controls.attach(&x, 3, 0, 1, 1);
    controls.attach(&Label::new(Some("Y")), 4, 0, 1, 1);
    controls.attach(&y, 5, 0, 1, 1);
    let width_label = Label::new(Some("Width"));
    let height_label = Label::new(Some("Height"));
    let text_label = Label::new(Some("Text"));
    controls.attach(&width_label, 0, 1, 1, 1);
    controls.attach(&width, 1, 1, 1, 1);
    controls.attach(&height_label, 2, 1, 1, 1);
    controls.attach(&height, 3, 1, 1, 1);
    controls.attach(&text_label, 0, 2, 1, 1);
    controls.attach(&text_entry, 1, 2, 5, 1);
    let update_control_visibility = {
        let width_label = width_label.clone();
        let width = width.clone();
        let height_label = height_label.clone();
        let height = height.clone();
        let text_label = text_label.clone();
        let text_entry = text_entry.clone();
        move |selected: u32| {
            let rectangle = selected == 1;
            let text = selected == 2;
            width_label.set_visible(rectangle);
            width.set_visible(rectangle);
            height_label.set_visible(rectangle);
            height.set_visible(rectangle);
            text_label.set_visible(text);
            text_entry.set_visible(text);
        }
    };
    update_control_visibility(tool.selected());
    let width_label_for_selection = width_label.clone();
    let width_for_selection = width.clone();
    let height_label_for_selection = height_label.clone();
    let height_for_selection = height.clone();
    let text_label_for_selection = text_label.clone();
    let text_for_selection = text_entry.clone();
    tool.connect_selected_notify(move |tool| {
        let rectangle = tool.selected() == 1;
        let text = tool.selected() == 2;
        width_label_for_selection.set_visible(rectangle);
        width_for_selection.set_visible(rectangle);
        height_label_for_selection.set_visible(rectangle);
        height_for_selection.set_visible(rectangle);
        text_label_for_selection.set_visible(text);
        text_for_selection.set_visible(text);
    });
    content.append(&controls);

    let error_label = Label::new(None);
    error_label.add_css_class("error");
    error_label.set_xalign(0.0);
    error_label.set_wrap(true);
    content.append(&error_label);
    let actions = GtkBox::new(Orientation::Horizontal, 8);
    let add_button = Button::with_label("Add mark");
    add_button.add_css_class("suggested-action");
    let reset_button = Button::with_label("Reset annotations");
    actions.append(&add_button);
    actions.append(&reset_button);
    content.append(&actions);
    dialog.content_area().append(&content);

    let draft = Rc::new(RefCell::new(AnnotationDraft::new(image)));
    let draft_for_reset = Rc::clone(&draft);
    let picture_for_reset = picture.clone();
    let error_for_reset = error_label.clone();
    reset_button.connect_clicked(move |_| {
        draft_for_reset.borrow_mut().reset();
        match display_annotation_image(
            &picture_for_reset,
            draft_for_reset.borrow().staged().bytes(),
        ) {
            Ok(_) => error_for_reset.set_text(""),
            Err(error) => error_for_reset.set_text(&error),
        }
    });

    let draft_for_apply = Rc::clone(&draft);
    let dialog_for_apply = dialog.clone();
    let picture_for_apply = picture.clone();
    let error_for_apply = error_label.clone();
    let reset_for_apply = reset_button.clone();
    let confirm_for_apply = confirm_button.clone();
    add_button.connect_clicked(move |add_button| {
        let annotation = match tool.selected() {
            0 => Annotation::Pointer { x: x.value_as_int() as u32, y: y.value_as_int() as u32 },
            1 => Annotation::Rectangle {
                x: x.value_as_int() as u32,
                y: y.value_as_int() as u32,
                width: width.value_as_int() as u32,
                height: height.value_as_int() as u32,
            },
            _ if text_entry.text().trim().is_empty() => {
                error_for_apply.set_text("Enter annotation text before adding the mark.");
                return;
            }
            _ => Annotation::Text {
                x: x.value_as_int() as u32,
                y: y.value_as_int() as u32,
                text: text_entry.text().to_string(),
            },
        };
        error_for_apply.set_text("");
        add_button.set_sensitive(false);
        reset_for_apply.set_sensitive(false);
        confirm_for_apply.set_sensitive(false);
        let staged = draft_for_apply.borrow().staged().clone();
        let (sender, receiver) = mpsc::channel();
        let _ = thread::Builder::new().name("captee-annotation".to_owned()).spawn(move || {
            let result = PngAnnotationBackend::new().annotate(&staged, &annotation);
            let _ = sender.send(result);
        });

        let draft = Rc::clone(&draft_for_apply);
        let dialog = dialog_for_apply.clone();
        let picture = picture_for_apply.clone();
        let error_label = error_for_apply.clone();
        let add_button = add_button.clone();
        let reset_button = reset_for_apply.clone();
        let confirm_button = confirm_for_apply.clone();
        glib::timeout_add_local(Duration::from_millis(50), move || {
            if !dialog.is_visible() {
                return glib::ControlFlow::Break;
            }
            match receiver.try_recv() {
                Ok(AnnotationResult::Completed(image)) => {
                    draft.borrow_mut().replace_staged(image);
                    match display_annotation_image(&picture, draft.borrow().staged().bytes()) {
                        Ok(_) => error_label.set_text(""),
                        Err(error) => error_label.set_text(&error),
                    }
                }
                Ok(AnnotationResult::Cancelled) => {
                    error_label.set_text("Annotation cancelled.");
                }
                Ok(AnnotationResult::Failed(error)) => {
                    error_label.set_text(&format!("Could not add annotation: {error}"));
                }
                Err(TryRecvError::Empty) => return glib::ControlFlow::Continue,
                Err(TryRecvError::Disconnected) => {
                    error_label.set_text("Annotation worker stopped unexpectedly.");
                }
            }
            add_button.set_sensitive(true);
            reset_button.set_sensitive(true);
            confirm_button.set_sensitive(true);
            glib::ControlFlow::Break
        });
    });

    let project_ui = project_ui.clone();
    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Accept {
            let image = draft.borrow().confirmed();
            *project_ui.pending_capture.borrow_mut() = None;
            *project_ui.pending_annotation.borrow_mut() = None;
            dialog.hide();
            start_capture_storage(&project_ui, image);
            return;
        } else {
            *project_ui.pending_capture.borrow_mut() = None;
            *project_ui.pending_annotation.borrow_mut() = None;
            project_ui.status.set_text("Capture discarded; the project was not changed.");
        }
        dialog.hide();
    });
    dialog.present();
    Ok(())
}

fn show_capture_review_dialog(project_ui: &ProjectUi, image: CapturedImage) -> Result<(), String> {
    let Some(application) = project_ui.application() else {
        return Err("The workspace window is no longer available.".to_owned());
    };
    let selection = image.selection();
    let origin = project_ui.capture_origin.borrow().clone();
    let review = Rc::new(RefCell::new(CaptureReview::new(image.clone())));
    let capture_surface = gtk::Overlay::new();
    capture_surface.add_css_class("capture-review-surface");
    capture_surface.set_hexpand(true);
    capture_surface.set_vexpand(true);
    let review_window = Window::builder()
        .application(&application)
        .decorated(false)
        .deletable(false)
        .focusable(true)
        .modal(true)
        .resizable(false)
        .child(&capture_surface)
        .build();
    let review_title = format!("Captee Capture Review {}", std::process::id());
    review_window.set_title(Some(&review_title));
    review_window.add_css_class("capture-review-window");

    let panel = GtkBox::new(Orientation::Vertical, 8);
    panel.set_width_request(640);
    panel.set_height_request(360);
    panel.set_halign(Align::Fill);
    panel.set_valign(Align::Fill);
    panel.add_css_class("capture-review-panel");

    let source_context = project_ui
        .editor
        .borrow()
        .as_ref()
        .map(EditorBridge::state)
        .map(|state| {
            state
                .text
                .lines()
                .rev()
                .take(5)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|text| !text.is_empty())
        .unwrap_or_default();
    let context_label = Label::new(Some(&source_context));
    context_label.set_xalign(0.0);
    context_label.set_wrap(true);
    context_label.set_selectable(true);
    context_label.set_max_width_chars(100);
    context_label.add_css_class("capture-context");
    panel.append(&context_label);

    let selection_text = image
        .selection()
        .map(format_selection_geometry)
        .unwrap_or_else(|| "Selection geometry unavailable for this capture backend".to_owned());
    let selection_label = Label::new(Some(&selection_text));
    selection_label.set_xalign(0.0);
    selection_label.add_css_class("capture-context");
    panel.append(&selection_label);

    let placement = ToggleButton::with_label("Insert annotation before image");
    placement.set_active(true);
    placement.set_tooltip_text(Some(
        "Toggle whether the annotation code is before or after the image block",
    ));
    let placement_for_toggle = placement.clone();
    let review_for_toggle = Rc::clone(&review);
    placement.connect_toggled(move |button| {
        let mut review = review_for_toggle.borrow_mut();
        if review.before_image() == button.is_active() {
            review.toggle_placement();
        }
        if button.is_active() {
            placement_for_toggle.set_label("Insert annotation before image");
        } else {
            placement_for_toggle.set_label("Insert annotation after image");
        }
    });

    let code_buffer = sourceview::Buffer::builder().highlight_matching_brackets(true).build();
    configure_typst_buffer(&code_buffer);
    code_buffer.set_text("");
    let code_view = sourceview::View::with_buffer(&code_buffer);
    code_view.set_show_line_numbers(true);
    code_view.set_monospace(true);
    code_view.set_hexpand(true);
    code_view.set_vexpand(true);
    code_view.add_css_class("typst-editor");
    code_view.set_tooltip_text(Some("Full Typst annotation code"));

    let code_scroller = ScrolledWindow::builder()
        .child(&code_view)
        .hexpand(true)
        .vexpand(true)
        .min_content_height(220)
        .build();
    let code_editor = gtk::Overlay::new();
    code_editor.set_child(Some(&code_scroller));
    let code_placeholder = Label::new(Some("Type Typst annotation here…"));
    code_placeholder.set_halign(Align::Start);
    code_placeholder.set_valign(Align::Start);
    code_placeholder.set_margin_start(56);
    code_placeholder.set_margin_top(10);
    code_placeholder.add_css_class("capture-context");
    code_editor.add_overlay(&code_placeholder);
    let code_placeholder_for_change = code_placeholder.clone();
    code_buffer.connect_changed(move |buffer| {
        code_placeholder_for_change.set_visible(buffer.char_count() == 0);
    });
    panel.append(&code_editor);
    let bottom = GtkBox::new(Orientation::Horizontal, 8);
    bottom.set_margin_top(4);
    bottom.append(&placement);
    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    bottom.append(&spacer);
    let confirm = Button::with_label("Confirm");
    confirm.add_css_class("suggested-action");
    let cancel = Button::with_label("Cancel");
    bottom.append(&confirm);
    bottom.append(&cancel);
    panel.append(&bottom);

    let modify = Button::with_label("Modify");
    modify.set_halign(Align::End);
    modify.set_tooltip_text(Some("Discard this staged capture and select a new region"));
    panel.prepend(&modify);

    capture_surface.set_child(Some(&panel));
    review_window.set_default_size(640, 360);
    let completion_popover = Popover::new();
    completion_popover.set_parent(&code_view);
    let completion_list = GtkBox::new(Orientation::Vertical, 2);
    completion_popover.set_child(Some(&completion_list));
    let completion_key = gtk::EventControllerKey::new();
    let completion_popover_for_key = completion_popover.clone();
    let completion_list_for_key = completion_list.clone();
    let code_buffer_for_key = code_buffer.clone();
    completion_key.connect_key_pressed(move |_, key, _, state| {
        if key == gtk::gdk::Key::space && state.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
            while let Some(child) = completion_list_for_key.first_child() {
                completion_list_for_key.remove(&child);
            }
            let text = code_buffer_for_key
                .text(&code_buffer_for_key.start_iter(), &code_buffer_for_key.end_iter(), true)
                .to_string();
            let cursor_chars =
                code_buffer_for_key.iter_at_mark(&code_buffer_for_key.get_insert()).offset().max(0)
                    as usize;
            let cursor = byte_offset_for_character(&text, cursor_chars);
            for suggestion in typst_completions(&text, cursor) {
                let item = Button::with_label(&suggestion.label);
                let buffer = code_buffer_for_key.clone();
                let popover = completion_popover_for_key.clone();
                item.connect_clicked(move |_| {
                    let text =
                        buffer.text(&buffer.start_iter(), &buffer.end_iter(), true).to_string();
                    let cursor_chars =
                        buffer.iter_at_mark(&buffer.get_insert()).offset().max(0) as usize;
                    let cursor = byte_offset_for_character(&text, cursor_chars);
                    if let Some(edit) = typst_completions(&text, cursor)
                        .into_iter()
                        .find(|candidate| candidate.label == suggestion.label)
                        .and_then(|candidate| completion_edit(&text, cursor, &candidate))
                    {
                        let start_chars = text[..edit.range.start].chars().count() as i32;
                        let end_chars = text[..edit.range.end].chars().count() as i32;
                        let mut start = buffer.iter_at_offset(start_chars);
                        let mut end = buffer.iter_at_offset(end_chars);
                        buffer.delete(&mut start, &mut end);
                        let mut insert = buffer.iter_at_offset(start_chars);
                        buffer.insert(&mut insert, &edit.replacement);
                    }
                    popover.popdown();
                });
                completion_list_for_key.append(&item);
            }
            if completion_list_for_key.first_child().is_some() {
                completion_popover_for_key.popup();
            }
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    code_view.add_controller(completion_key);
    let editor_key = gtk::EventControllerKey::new();
    let confirm_for_editor = confirm.clone();
    let cancel_for_editor = cancel.clone();
    editor_key.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            cancel_for_editor.emit_clicked();
            return glib::Propagation::Stop;
        }
        if key == gtk::gdk::Key::Return {
            confirm_for_editor.emit_clicked();
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    code_view.add_controller(editor_key);
    let key_controller = gtk::EventControllerKey::new();
    let confirm_for_key = confirm.clone();
    let cancel_for_key = cancel.clone();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            cancel_for_key.emit_clicked();
            return glib::Propagation::Stop;
        }
        if key == gtk::gdk::Key::Return {
            confirm_for_key.emit_clicked();
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    panel.add_controller(key_controller);
    code_view.grab_focus();

    let modify_ui = project_ui.clone();
    let modify_window = review_window.clone();
    modify.connect_clicked(move |_| {
        modify_window.close();
        *modify_ui.pending_capture.borrow_mut() = None;
        modify_ui.status.set_text("Select a new capture region.");
        start_capture(&modify_ui);
    });

    let cancel_ui = project_ui.clone();
    let cancel_window = review_window.clone();
    cancel.connect_clicked(move |_| {
        cancel_window.close();
        *cancel_ui.pending_capture.borrow_mut() = None;
        *cancel_ui.pending_annotation.borrow_mut() = None;
        cancel_ui.status.set_text("Capture discarded; the project was not changed.");
    });

    let confirm_ui = project_ui.clone();
    let confirm_window = review_window.clone();
    let review_for_confirm = Rc::clone(&review);
    confirm.connect_clicked(move |_| {
        let text =
            code_buffer.text(&code_buffer.start_iter(), &code_buffer.end_iter(), true).to_string();
        let annotation = text.trim().to_owned();
        if annotation.is_empty() {
            confirm_ui.status.set_text("Enter Typst annotation code before confirming.");
            return;
        }
        let mut review = review_for_confirm.borrow_mut();
        review.set_annotation(annotation);
        let confirmed = match review.confirm() {
            Ok(confirmed) => confirmed,
            Err(_) => {
                confirm_ui.status.set_text("Enter Typst annotation code before confirming.");
                return;
            }
        };
        confirm_window.close();
        *confirm_ui.pending_capture.borrow_mut() = None;
        *confirm_ui.pending_annotation.borrow_mut() = None;
        start_capture_storage_with_review(
            &confirm_ui,
            confirmed.image,
            confirmed.annotation,
            confirmed.before_image,
        );
    });

    review_window.present();
    if let (Some(origin), Some(selection)) = (origin, selection) {
        let Some((popup_x, popup_y)) = capture_popup_position(selection, &origin) else {
            return Ok(());
        };
        let title = review_title.clone();
        let workspace = origin.workspace;
        let _ =
            thread::Builder::new().name("captee-capture-placement".to_owned()).spawn(move || {
                for _ in 0..20 {
                    if place_capture_review_window(&title, &workspace, popup_x, popup_y).is_ok() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(25));
                }
            });
    }

    Ok(())
}

fn capture_popup_position(
    selection: SelectionGeometry,
    origin: &CaptureOrigin,
) -> Option<(i64, i64)> {
    const POPUP_WIDTH: i64 = 640;
    const POPUP_HEIGHT: i64 = 360;
    const GAP: i64 = 16;
    const INSET: i64 = 8;
    let right_x = i64::from(selection.x) + i64::from(selection.width) + GAP;
    let left_x = i64::from(selection.x) - POPUP_WIDTH - GAP;
    let mut x =
        if right_x + POPUP_WIDTH <= origin.x + origin.width - INSET { right_x } else { left_x };
    let mut y = i64::from(selection.y);
    x = x.clamp(origin.x + INSET, origin.x + origin.width - POPUP_WIDTH - INSET);
    y = y.clamp(origin.y + INSET, origin.y + origin.height - POPUP_HEIGHT - INSET);
    Some((x, y))
}

fn start_capture_storage_with_review(
    project_ui: &ProjectUi,
    image: CapturedImage,
    annotation: String,
    before_image: bool,
) {
    let snapshot = project_ui.shell.borrow().snapshot();
    let Some(project) = snapshot.app.project else {
        project_ui.status.set_text("The active project closed before the capture could be saved.");
        return;
    };
    if let Err(error) = project_ui.shell.borrow_mut().dispatch(UiCommand::StoreCapture) {
        project_ui.status.set_text(&format!("Error: {error}"));
        return;
    }
    let task = match project_ui.coordinator.borrow_mut().begin(OperationKind::Capture, false) {
        Ok(task) => task,
        Err(error) => {
            project_ui.status.set_text(&format!("Error: {error}"));
            return;
        }
    };
    project_ui.status.set_text("Validating and saving capture…");
    let root = PathBuf::from(project.root);
    let _ = thread::Builder::new().name("captee-capture-store".to_owned()).spawn(move || {
        let outcome = match AssetStore::new(root)
            .and_then(|store| store.save_png(AnnotatedImage::new(image.bytes().to_vec())))
        {
            Ok(asset) => OperationOutcome::Completed(WorkspaceOperationResult::CaptureStored {
                asset,
                annotation,
                before_image,
            }),
            Err(error) => OperationOutcome::Failed(error.to_string()),
        };
        let _ = task.finish(outcome);
    });
}

#[allow(dead_code)]
fn start_capture_storage(project_ui: &ProjectUi, image: AnnotatedImage) {
    let snapshot = project_ui.shell.borrow().snapshot();
    let Some(project) = snapshot.app.project else {
        *project_ui.pending_annotation.borrow_mut() = Some(image);
        project_ui.status.set_text("The active project closed before the capture could be saved.");
        return;
    };
    if let Err(error) = project_ui.shell.borrow_mut().dispatch(UiCommand::StoreCapture) {
        *project_ui.pending_annotation.borrow_mut() = Some(image);
        project_ui.status.set_text(&format!("Error: {error}"));
        return;
    }
    let task = match project_ui.coordinator.borrow_mut().begin(OperationKind::Capture, false) {
        Ok(task) => task,
        Err(error) => {
            *project_ui.pending_annotation.borrow_mut() = Some(image);
            let _ = project_ui
                .shell
                .borrow_mut()
                .dispatch(UiCommand::Fail { message: error.to_string() });
            project_ui.status.set_text(&format!("Error: {error}"));
            return;
        }
    };
    project_ui.status.set_text("Validating and saving capture…");
    let root = PathBuf::from(project.root);
    let _ = thread::Builder::new().name("captee-capture-store".to_owned()).spawn(move || {
        let outcome = match AssetStore::new(root).and_then(|store| store.save_png(image)) {
            Ok(asset) => OperationOutcome::Completed(WorkspaceOperationResult::CaptureStored {
                asset,
                annotation: String::new(),
                before_image: true,
            }),
            Err(error) => OperationOutcome::Failed(error.to_string()),
        };
        let _ = task.finish(outcome);
    });
}

fn display_annotation_image(picture: &gtk::Picture, bytes: &[u8]) -> Result<(i32, i32), String> {
    let bytes = glib::Bytes::from(bytes);
    let texture = gtk::gdk::Texture::from_bytes(&bytes)
        .map_err(|error| format!("Could not display captured PNG: {error}"))?;
    let size = (texture.width(), texture.height());
    picture.set_paintable(Some(&texture));
    Ok(size)
}

fn format_selection_geometry(selection: SelectionGeometry) -> String {
    format!(
        "Selection: x={}, y={}, width={}, height={}",
        selection.x, selection.y, selection.width, selection.height
    )
}

fn configure_typst_buffer(buffer: &sourceview::Buffer) {
    let manager = sourceview::LanguageManager::default();
    let asset_dir = format!("{}/assets", env!("CARGO_MANIFEST_DIR"));
    let mut search_path =
        manager.search_path().into_iter().map(|path| path.to_string()).collect::<Vec<_>>();
    if !search_path.iter().any(|path| path == &asset_dir) {
        search_path.insert(0, asset_dir);
        let search_path = search_path.iter().map(String::as_str).collect::<Vec<_>>();
        manager.set_search_path(&search_path);
    }
    let language = manager.language("typst").or_else(|| manager.language("markdown"));
    buffer.set_language(language.as_ref());
    buffer.set_highlight_syntax(language.is_some());
    let schemes = sourceview::StyleSchemeManager::default();
    let dark_scheme = ["Adwaita-dark", "oblivion", "solarized-dark"]
        .into_iter()
        .find_map(|name| schemes.scheme(name));
    buffer.set_style_scheme(dark_scheme.as_ref());
}

fn capture_insertion_expression(
    image_expression: &str,
    annotation: &str,
    before_image: bool,
) -> String {
    if annotation.trim().is_empty() {
        return format!("{image_expression}\n");
    }
    if before_image {
        format!("{}\n{}\n", annotation.trim(), image_expression)
    } else {
        format!("{}\n{}\n", image_expression, annotation.trim())
    }
}

fn show_settings_dialog(project_ui: &ProjectUi) {
    let snapshot = project_ui.shell.borrow().snapshot();
    if snapshot.app.project.is_none() {
        project_ui.status.set_text("Open a project before changing settings.");
        return;
    }
    if snapshot.progress.is_some() {
        project_ui.status.set_text("Wait for the active operation before changing settings.");
        return;
    }
    let Some(window) = project_ui.window() else {
        return;
    };
    let settings = snapshot.app.settings;
    let dialog = Dialog::builder()
        .title("Project settings")
        .transient_for(&window)
        .modal(true)
        .default_width(620)
        .default_height(680)
        .build();
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Save settings", ResponseType::Accept);
    dialog.set_default_response(ResponseType::Accept);

    let content = GtkBox::new(Orientation::Vertical, 12);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);

    let formatting_title = Label::new(Some("Formatting"));
    formatting_title.add_css_class("heading");
    formatting_title.set_xalign(0.0);
    content.append(&formatting_title);
    let line_width = gtk::SpinButton::with_range(1.0, 400.0, 1.0);
    line_width.set_value(f64::from(settings.formatting.line_width));
    let format_on_save = CheckButton::with_label("Format before saving");
    format_on_save.set_active(settings.formatting.format_on_save);
    let formatting_grid = gtk::Grid::builder().column_spacing(8).row_spacing(8).build();
    let line_width_label = Label::new(Some("Preferred line width"));
    line_width_label.set_xalign(0.0);
    formatting_grid.attach(&line_width_label, 0, 0, 1, 1);
    formatting_grid.attach(&line_width, 1, 0, 1, 1);
    formatting_grid.attach(&format_on_save, 0, 1, 2, 1);
    content.append(&formatting_grid);

    let capture_title = Label::new(Some("Capture"));
    capture_title.add_css_class("heading");
    capture_title.set_xalign(0.0);
    content.append(&capture_title);
    let portal_enabled = CheckButton::with_label("Use the desktop screenshot portal");
    portal_enabled.set_active(settings.capture.portal_enabled);
    let fallback_enabled =
        CheckButton::with_label("Use slurp/grim (preferred automatically on Hyprland)");
    fallback_enabled.set_active(settings.capture.fallback_enabled);
    content.append(&portal_enabled);
    content.append(&fallback_enabled);

    let preview_title = Label::new(Some("Preview"));
    preview_title.add_css_class("heading");
    preview_title.set_xalign(0.0);
    content.append(&preview_title);
    let auto_render = CheckButton::with_label("Render automatically after edits");
    auto_render.set_active(settings.preview.auto_render);
    let zoom = gtk::SpinButton::with_range(25.0, 500.0, 5.0);
    zoom.set_value(f64::from(settings.preview.zoom_percent));
    let preview_grid = gtk::Grid::builder().column_spacing(8).row_spacing(8).build();
    preview_grid.attach(&auto_render, 0, 0, 2, 1);
    let zoom_label = Label::new(Some("Preview zoom (%)"));
    zoom_label.set_xalign(0.0);
    preview_grid.attach(&zoom_label, 0, 1, 1, 1);
    preview_grid.attach(&zoom, 1, 1, 1, 1);
    content.append(&preview_grid);

    let keybindings_title = Label::new(Some("Keyboard shortcuts"));
    keybindings_title.add_css_class("heading");
    keybindings_title.set_xalign(0.0);
    content.append(&keybindings_title);
    let keybindings = gtk::Grid::builder().column_spacing(8).row_spacing(8).build();
    let save_key = Entry::new();
    save_key.set_text(&settings.keybindings.save);
    let format_key = Entry::new();
    format_key.set_text(&settings.keybindings.format);
    let find_key = Entry::new();
    find_key.set_text(&settings.keybindings.find_replace);
    let completion_key = Entry::new();
    completion_key.set_text(&settings.keybindings.completion);
    let capture_key = Entry::new();
    capture_key.set_text(&settings.keybindings.capture);
    let preview_key = Entry::new();
    preview_key.set_text(&settings.keybindings.preview);
    let export_key = Entry::new();
    export_key.set_text(&settings.keybindings.export);
    for (row, (label, entry)) in [
        ("Save", &save_key),
        ("Format", &format_key),
        ("Find and Replace", &find_key),
        ("Completion", &completion_key),
        ("Capture", &capture_key),
        ("Preview", &preview_key),
        ("Export PDF", &export_key),
    ]
    .into_iter()
    .enumerate()
    {
        let label = Label::new(Some(label));
        label.set_xalign(0.0);
        entry.set_hexpand(true);
        entry.set_tooltip_text(Some("GTK accelerator, for example <Primary><Shift>c"));
        keybindings.attach(&label, 0, row as i32, 1, 1);
        keybindings.attach(entry, 1, row as i32, 1, 1);
    }
    content.append(&keybindings);
    let error_label = Label::new(None);
    error_label.add_css_class("error");
    error_label.set_xalign(0.0);
    error_label.set_wrap(true);
    content.append(&error_label);
    dialog.content_area().append(
        &ScrolledWindow::builder()
            .child(&content)
            .hexpand(true)
            .vexpand(true)
            .min_content_height(520)
            .build(),
    );

    let project_ui = project_ui.clone();
    dialog.connect_response(move |dialog, response| {
        if response != ResponseType::Accept {
            dialog.close();
            return;
        }
        let mut updated = settings.clone();
        updated.formatting.line_width = line_width.value_as_int() as u16;
        updated.formatting.format_on_save = format_on_save.is_active();
        updated.capture.portal_enabled = portal_enabled.is_active();
        updated.capture.fallback_enabled = fallback_enabled.is_active();
        updated.preview.auto_render = auto_render.is_active();
        updated.preview.zoom_percent = zoom.value_as_int() as u16;
        updated.keybindings = KeybindingSettings {
            save: save_key.text().to_string(),
            format: format_key.text().to_string(),
            find_replace: find_key.text().to_string(),
            completion: completion_key.text().to_string(),
            capture: capture_key.text().to_string(),
            preview: preview_key.text().to_string(),
            export: export_key.text().to_string(),
        };
        if let Err(error) = crate::validate_settings(&updated) {
            error_label.set_text(&error.to_string());
            return;
        }
        if let Some((action, binding)) = updated
            .keybindings
            .named_bindings()
            .into_iter()
            .find(|(_, binding)| gtk::accelerator_parse(*binding).is_none())
        {
            error_label.set_text(&format!("{action} has an invalid accelerator: {binding}"));
            return;
        }
        dialog.close();
        start_settings_save(&project_ui, updated);
    });
    dialog.present();
}

fn start_settings_save(project_ui: &ProjectUi, settings: ProjectSettings) {
    let snapshot = project_ui.shell.borrow().snapshot();
    let Some(project) = snapshot.app.project else {
        project_ui.status.set_text("The project closed before settings could be saved.");
        return;
    };
    if let Err(error) = project_ui.shell.borrow_mut().dispatch(UiCommand::SaveSettings) {
        project_ui.status.set_text(&format!("Error: {error}"));
        return;
    }
    let task = match project_ui.coordinator.borrow_mut().begin(OperationKind::Settings, false) {
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
    project_ui.status.set_text("Saving project settings…");
    let root = PathBuf::from(project.root);
    let _ = thread::Builder::new().name("captee-settings".to_owned()).spawn(move || {
        let outcome = match save_project_settings(root, settings) {
            Ok(config) => OperationOutcome::Completed(WorkspaceOperationResult::SettingsSaved(
                config.settings,
            )),
            Err(error) => OperationOutcome::Failed(error.to_string()),
        };
        let _ = task.finish(outcome);
    });
}

fn apply_project_accelerators(application: &Application, keybindings: &KeybindingSettings) {
    for (action, binding) in [
        ("save", keybindings.save.as_str()),
        ("format", keybindings.format.as_str()),
        ("find-replace", keybindings.find_replace.as_str()),
        ("completion", keybindings.completion.as_str()),
        ("capture", keybindings.capture.as_str()),
        ("preview", keybindings.preview.as_str()),
        ("export", keybindings.export.as_str()),
    ] {
        application.set_accels_for_action(&format!("app.{action}"), &[binding]);
    }
}

fn display_preview_pages(project_ui: &ProjectUi, pages: Vec<Vec<u8>>) -> Result<(), String> {
    let mut pictures = Vec::with_capacity(pages.len());
    for png in pages {
        let bytes = glib::Bytes::from_owned(png);
        let texture = gtk::gdk::Texture::from_bytes(&bytes)
            .map_err(|error| format!("could not decode preview page: {error}"))?;
        let picture = gtk::Picture::for_paintable(&texture);
        picture.set_can_shrink(true);
        picture.set_hexpand(true);
        picture.set_halign(Align::Center);
        pictures.push(picture);
    }
    while let Some(child) = project_ui.preview_pages.first_child() {
        project_ui.preview_pages.remove(&child);
    }
    for picture in pictures {
        project_ui.preview_pages.append(&picture);
    }
    apply_preview_zoom(project_ui);
    Ok(())
}

fn connect_preview_scale(project_ui: &ProjectUi) {
    let scale_ui = project_ui.clone();
    project_ui.preview_scale.connect_selected_notify(move |dropdown| {
        let mode = match dropdown.selected() {
            0 => PreviewScale::FitPage,
            1 => PreviewScale::FitPageWidth,
            2 => PreviewScale::Percent(50),
            3 => PreviewScale::Percent(75),
            4 => PreviewScale::Percent(100),
            5 => PreviewScale::Percent(125),
            6 => PreviewScale::Percent(150),
            7 => PreviewScale::Percent(200),
            _ => PreviewScale::Percent(300),
        };
        scale_ui.preview_scale_mode.set(mode);
        apply_preview_zoom(&scale_ui);
    });

    let resize_ui = project_ui.clone();
    project_ui.preview_scroller.connect_notify_local(Some("width"), move |_, _| {
        if matches!(
            resize_ui.preview_scale_mode.get(),
            PreviewScale::FitPage | PreviewScale::FitPageWidth
        ) {
            apply_preview_zoom(&resize_ui);
        }
    });

    let resize_ui = project_ui.clone();
    project_ui.preview_scroller.connect_notify_local(Some("height"), move |_, _| {
        if matches!(
            resize_ui.preview_scale_mode.get(),
            PreviewScale::FitPage | PreviewScale::FitPageWidth
        ) {
            apply_preview_zoom(&resize_ui);
        }
    });
}

fn apply_preview_zoom(project_ui: &ProjectUi) {
    let mode = project_ui.preview_scale_mode.get();
    let available_size = Some((
        i64::from(project_ui.preview_scroller.width().saturating_sub(24)),
        i64::from(project_ui.preview_scroller.height().saturating_sub(24)),
    ))
    .filter(|(width, height)| *width > 0 && *height > 0);
    let mut child = project_ui.preview_pages.first_child();
    while let Some(widget) = child {
        if let Ok(picture) = widget.clone().downcast::<gtk::Picture>() {
            if let Some(paintable) = picture.paintable() {
                let intrinsic_width = i64::from(paintable.intrinsic_width()).max(1);
                let intrinsic_height = i64::from(paintable.intrinsic_height()).max(1);
                let width = preview_width(mode, intrinsic_width, intrinsic_height, available_size);
                let height = (intrinsic_height * width / intrinsic_width).clamp(1, 8192);
                picture.set_size_request(width as i32, height as i32);
            }
        }
        child = widget.next_sibling();
    }
}

fn preview_width(
    mode: PreviewScale,
    intrinsic_width: i64,
    intrinsic_height: i64,
    available_size: Option<(i64, i64)>,
) -> i64 {
    match mode {
        PreviewScale::FitPage => available_size
            .map(|(available_width, available_height)| {
                let width_fit = available_width;
                let height_fit = intrinsic_width * available_height / intrinsic_height.max(1);
                width_fit.min(height_fit)
            })
            .unwrap_or(intrinsic_width)
            .clamp(1, 8192),
        PreviewScale::FitPageWidth => available_size
            .map(|(available_width, _)| available_width)
            .unwrap_or(intrinsic_width)
            .clamp(1, 8192),
        PreviewScale::Percent(percent) => {
            (intrinsic_width * i64::from(percent) / 100).clamp(1, 8192)
        }
    }
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
        drain_ready_results(&project_ui.coordinator, |result| {
            apply_operation_result(&project_ui, result);
        });
        loop {
            let result = project_ui.background_receiver.borrow().try_recv();
            match result {
                Ok(result) => apply_background_result(&project_ui, result),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        sync_operation_feedback(&project_ui);
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
                OperationOutcome::Completed(WorkspaceOperationResult::Saved {
                    document,
                    formatted,
                }) => {
                    let mut editor = project_ui.editor.borrow_mut();
                    if formatted
                        && editor
                            .as_ref()
                            .is_some_and(|editor| editor.state().text != document.text())
                    {
                        let _ = editor
                            .as_mut()
                            .and_then(|editor| editor.update_from_buffer(document.text()).ok())
                            .flatten();
                    }
                    let state =
                        editor.as_mut().and_then(|editor| editor.apply_saved_document(document));
                    drop(editor);
                    if let Some(state) = state {
                        let _ = project_ui
                            .shell
                            .borrow_mut()
                            .dispatch(UiCommand::Complete { message: "Document saved".to_owned() });
                        let _ = project_ui
                            .shell
                            .borrow_mut()
                            .dispatch(UiCommand::SetDirty(state.dirty));
                        project_ui.status.set_text(if formatted {
                            "Document formatted and saved."
                        } else {
                            "Document saved."
                        });
                        apply_editor_state(project_ui, &state, formatted);
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
                    let failure = outcome.result.as_ref().err().map(|error| error.message.clone());
                    let (accepted, pages) =
                        outcome.apply_to_with_pages(&mut project_ui.render_state.borrow_mut());
                    if !accepted {
                        let message = "Preview result ignored because its revision is stale.";
                        let _ = project_ui
                            .shell
                            .borrow_mut()
                            .dispatch(UiCommand::Warn { message: message.to_owned() });
                        project_ui.status.set_text(message);
                    } else if let Some(pages) = pages {
                        match display_preview_pages(project_ui, pages) {
                            Ok(()) => {
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
                                project_ui.status.set_text(&format!("Error: {message}"));
                            }
                        }
                    } else if let Some(message) = failure {
                        let _ = project_ui
                            .shell
                            .borrow_mut()
                            .dispatch(UiCommand::Fail { message: message.clone() });
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
                    *project_ui.pending_capture.borrow_mut() = Some(image.clone());
                    *project_ui.pending_annotation.borrow_mut() = None;
                    match show_capture_review_dialog(project_ui, image) {
                        Ok(()) => {
                            let _ = project_ui.shell.borrow_mut().dispatch(UiCommand::Complete {
                                message: "Capture ready".to_owned(),
                            });
                            project_ui.status.set_text("Capture ready for annotation.");
                        }
                        Err(message) => {
                            *project_ui.pending_capture.borrow_mut() = None;
                            let _ = project_ui
                                .shell
                                .borrow_mut()
                                .dispatch(UiCommand::Fail { message: message.clone() });
                            project_ui.status.set_text(&format!("Error: {message}"));
                        }
                    }
                }
                OperationOutcome::Completed(WorkspaceOperationResult::CaptureStored {
                    asset,
                    annotation,
                    before_image,
                }) => {
                    let focused = project_ui.shell.borrow().snapshot().focused
                        == crate::FocusTarget::SourceEditor;
                    let character_offset =
                        project_ui.source_buffer.cursor_position().max(0) as usize;
                    let mut editor = project_ui.editor.borrow_mut();
                    let cursor = editor
                        .as_ref()
                        .map(EditorBridge::state)
                        .map(|state| byte_offset_for_character(&state.text, character_offset))
                        .unwrap_or_default();
                    let target = if focused { editor.as_mut() } else { None };
                    let expression = capture_insertion_expression(
                        &asset.typst_image_expression(),
                        &annotation,
                        before_image,
                    );
                    let insertion = {
                        let mut adapter = EditorInsertionBridge::new(target, cursor);
                        if focused {
                            adapter.insert_image_expression(&expression)
                        } else {
                            InsertionResult::NoFocusedEditor
                        }
                    };
                    let state = editor.as_ref().map(EditorBridge::state);
                    drop(editor);
                    match insertion {
                        InsertionResult::Inserted => {
                            if let Some(state) = state {
                                apply_editor_state(project_ui, &state, true);
                            }
                            let message = format!(
                                "Capture saved and inserted from {}.",
                                asset.relative_path().display()
                            );
                            let _ = project_ui
                                .shell
                                .borrow_mut()
                                .dispatch(UiCommand::Complete { message: message.clone() });
                            project_ui.status.set_text(&message);
                        }
                        InsertionResult::NoFocusedEditor => {
                            let message = format!(
                                "Capture saved to {}, but no source editor was focused.",
                                asset.relative_path().display()
                            );
                            let _ = project_ui
                                .shell
                                .borrow_mut()
                                .dispatch(UiCommand::Warn { message: message.clone() });
                            project_ui.status.set_text(&message);
                        }
                        InsertionResult::Cancelled => {
                            let _ = project_ui.shell.borrow_mut().dispatch(UiCommand::Cancel);
                            project_ui
                                .status
                                .set_text("Capture insertion cancelled; image was saved.");
                        }
                        InsertionResult::Failed(error) => {
                            let message = format!(
                                "Capture saved to {}, but insertion failed: {error}",
                                asset.relative_path().display()
                            );
                            let _ = project_ui
                                .shell
                                .borrow_mut()
                                .dispatch(UiCommand::Fail { message: message.clone() });
                            project_ui.status.set_text(&format!("Error: {message}"));
                        }
                    }
                }
                OperationOutcome::Completed(WorkspaceOperationResult::SettingsSaved(settings)) => {
                    let _ = project_ui
                        .shell
                        .borrow_mut()
                        .dispatch(UiCommand::Complete { message: "Settings saved".to_owned() });
                    let applied = project_ui
                        .shell
                        .borrow_mut()
                        .dispatch(UiCommand::ApplySettings(settings.clone()));
                    match applied {
                        Ok(()) => {
                            if let Some(application) = project_ui.application() {
                                apply_project_accelerators(&application, &settings.keybindings);
                            }
                            apply_preview_zoom(project_ui);
                            if settings.preview.auto_render {
                                if let Some(state) =
                                    project_ui.editor.borrow().as_ref().map(EditorBridge::state)
                                {
                                    schedule_preview(project_ui, &state);
                                }
                            }
                            project_ui.status.set_text("Project settings saved and applied.");
                        }
                        Err(error) => {
                            let message = format!("Saved settings could not be applied: {error}");
                            let _ = project_ui
                                .shell
                                .borrow_mut()
                                .dispatch(UiCommand::Fail { message: message.clone() });
                            project_ui.status.set_text(&format!("Error: {message}"));
                        }
                    }
                }
                OperationOutcome::Completed(WorkspaceOperationResult::AuthoringFailure {
                    message,
                }) => {
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
        ResultDisposition::Stale(result)
            if project_ui.coordinator.borrow().active_context().is_none()
                && project_ui.coordinator.borrow().active_source().as_ref()
                    == Some(result.context.source()) =>
        {
            // Explicitly cancelled work is expected to report late. Keep the
            // user's cancellation status instead of replacing it with a warning.
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
                    let current = editor.state();
                    let edit = completion_edit(&current.text, cursor, item)?;
                    editor.replace_range(edit.range, &edit.replacement).ok()
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
        project_ui.project_label.set_text("Captee");
        project_ui.project_name_label.set_text("Project");
        project_ui.project_panel_title.set_text("Project");
        return;
    };
    let modified = if snapshot.app.dirty { " • Modified" } else { "" };
    project_ui.project_label.set_text(&format!("{} · Captee{modified}", project.root));
    project_ui.project_name_label.set_text(&project.name);
    project_ui.project_panel_title.set_text(&project.name);
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
    if let Some(folder) = last_open_project_folder() {
        let folder = gio::File::for_path(folder);
        let _ = dialog.set_current_folder(Some(&folder));
    }
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

fn last_open_project_folder() -> Option<PathBuf> {
    let store_path = glib::user_data_dir().join("captee/recent-projects.json");
    RecentProjectStore::new(store_path)
        .load()
        .ok()?
        .paths
        .into_iter()
        .filter_map(|path| project_parent_folder(Path::new(&path)))
        .find(|path| path.is_dir())
}

fn project_parent_folder(path: &Path) -> Option<PathBuf> {
    path.parent().map(Path::to_path_buf)
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
        settings: project.settings.clone(),
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
            *project_ui.render_state.borrow_mut() = RenderState::new(0);
            *project_ui.pending_capture.borrow_mut() = None;
            *project_ui.pending_annotation.borrow_mut() = None;
            reset_preview_scale(project_ui);
            project_ui.expanded_tree.borrow_mut().clear();
            project_ui.tree_initialized.set(false);
            clear_preview_pages(project_ui);
            if let Some(application) = project_ui.application() {
                apply_project_accelerators(&application, &project.settings.keybindings);
            }
            refresh_project_label(project_ui);
            refresh_project_tree(project_ui);
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

fn clear_preview_pages(project_ui: &ProjectUi) {
    while let Some(child) = project_ui.preview_pages.first_child() {
        project_ui.preview_pages.remove(&child);
    }
}

fn reset_preview_scale(project_ui: &ProjectUi) {
    project_ui.preview_scale_mode.set(PreviewScale::FitPage);
    project_ui.preview_scale.set_selected(0);
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
    let closed = project_ui.shell.borrow_mut().dispatch(UiCommand::CloseProject);
    match closed {
        Ok(()) => {
            project_ui.autosave_sequence.set(project_ui.autosave_sequence.get().saturating_add(1));
            project_ui.coordinator.borrow_mut().deactivate_project();
            *project_ui.editor.borrow_mut() = None;
            project_ui.stack.set_visible_child_name("home");
            project_ui.syncing_buffer.set(true);
            project_ui.source_buffer.set_text("");
            project_ui.syncing_buffer.set(false);
            *project_ui.render_state.borrow_mut() = RenderState::new(0);
            *project_ui.pending_capture.borrow_mut() = None;
            *project_ui.pending_annotation.borrow_mut() = None;
            reset_preview_scale(project_ui);
            project_ui.expanded_tree.borrow_mut().clear();
            project_ui.tree_initialized.set(false);
            clear_preview_pages(project_ui);
            if let Some(application) = project_ui.application() {
                apply_project_accelerators(&application, &KeybindingSettings::default());
            }
            refresh_project_label(project_ui);
            refresh_project_tree(project_ui);
            project_ui.status.set_text("Project closed. Create or open a project to begin.");
        }
        Err(error) => project_ui.status.set_text(&format!("Error: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        byte_offset_for_character, capture_insertion_expression, preview_width,
        project_parent_folder, recovery_draft, validate_project_name, PreviewScale,
    };
    use captee_platform::AutosaveSnapshot;
    use std::path::{Path, PathBuf};

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
    fn remembered_open_folder_uses_project_parent() {
        assert_eq!(
            project_parent_folder(Path::new("Documents/TestProject")),
            Some(PathBuf::from("Documents"))
        );
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

    #[test]
    fn capture_annotation_can_be_inserted_before_or_after_image() {
        assert_eq!(
            capture_insertion_expression("#image(\"img/capture.png\")", "#line(length: 1em)", true),
            "#line(length: 1em)\n#image(\"img/capture.png\")\n"
        );
        assert_eq!(
            capture_insertion_expression(
                "#image(\"img/capture.png\")",
                "#line(length: 1em)",
                false
            ),
            "#image(\"img/capture.png\")\n#line(length: 1em)\n"
        );
    }

    #[test]
    fn preview_fit_page_width_uses_viewport_width_without_height_constraint() {
        assert_eq!(preview_width(PreviewScale::FitPageWidth, 800, 1000, Some((420, 120))), 420);
        assert_eq!(preview_width(PreviewScale::FitPage, 800, 1000, Some((420, 120))), 96);
    }
}
