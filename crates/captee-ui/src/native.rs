use crate::{UiCommand, UiShell};
use captee_core::{ProjectConfig, ProjectSession};
use captee_platform::{create_project, open_project};
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, Label, Orientation, Paned,
    PopoverMenu, ScrolledWindow, Stack,
};
use gtk4 as gtk;
use sourceview::prelude::*;
use sourceview5 as sourceview;
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;

const APPLICATION_ID: &str = "com.nightlyshelf.Captee";

/// Starts the GTK application with a project home screen and workspace shell.
pub fn run() -> glib::ExitCode {
    let application = Application::builder().application_id(APPLICATION_ID).build();
    application.connect_activate(build_ui);
    application.run()
}

fn build_ui(application: &Application) {
    let file_menu = install_actions(application);
    let shell = Rc::new(RefCell::new(UiShell::new()));
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

    let new_button = Button::with_label("New project");
    new_button.add_css_class("suggested-action");
    new_button.set_tooltip_text(Some("Create a new Captee project"));
    let open_button = Button::with_label("Open project");
    open_button.add_css_class("suggested-action");
    open_button.set_tooltip_text(Some("Open an existing Captee project"));
    let file_button = Button::with_label("File");
    file_button.add_css_class("suggested-action");
    file_button.set_tooltip_text(Some("Project and document actions"));
    let file_popover = PopoverMenu::from_model(Some(&file_menu));
    file_popover.set_parent(&file_button);
    let file_popover_for_click = file_popover.clone();
    file_button.connect_clicked(move |_| file_popover_for_click.popup());

    let home_new_button = Button::with_label("New project");
    home_new_button.add_css_class("suggested-action");
    let home_open_button = Button::with_label("Open project");
    home_open_button.add_css_class("suggested-action");
    let stack = Stack::builder().hexpand(true).vexpand(true).build();
    stack.add_named(&build_home(&home_new_button, &home_open_button), Some("home"));
    stack.add_named(&build_workspace(&source_view), Some("workspace"));
    stack.set_visible_child_name("home");

    let project_ui = ProjectUi {
        window: window.clone(),
        shell: Rc::clone(&shell),
        status: status.clone(),
        stack: stack.clone(),
        source_buffer: source_buffer.clone(),
        project_label: project_label.clone(),
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
    header.append(&new_button);
    header.append(&open_button);
    header.append(&file_button);

    let root = GtkBox::new(Orientation::Vertical, 0);
    root.append(&header);
    root.append(&stack);
    root.append(&status);
    window.set_child(Some(&root));

    connect_ui_actions(&shell, &status, &project_ui, application);
    connect_project_button(&new_button, true, &project_ui);
    connect_project_button(&open_button, false, &project_ui);
    connect_project_button(&home_new_button, true, &project_ui);
    connect_project_button(&home_open_button, false, &project_ui);
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

fn build_workspace(source_view: &sourceview::View) -> Paned {
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
    preview.append(&Label::new(Some("Render a document to see its PDF preview.")));

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
    workspace
}

fn install_actions(application: &Application) -> gio::Menu {
    let menu = gio::Menu::new();
    let file = gio::Menu::new();
    file.append(Some("New project"), Some("app.new-project"));
    file.append(Some("Open project"), Some("app.open-project"));
    file.append(Some("Close project"), Some("app.close-project"));
    file.append(Some("Save"), Some("app.save"));
    file.append(Some("Format"), Some("app.format"));
    file.append(Some("Find and Replace"), Some("app.find-replace"));
    file.append(Some("Capture"), Some("app.capture"));
    file.append(Some("Preview"), Some("app.preview"));
    file.append(Some("Export PDF"), Some("app.export"));
    menu.append_submenu(Some("File"), &file);
    application.set_menubar(Some(&menu));

    for (name, accelerator) in [
        ("new-project", "<Primary>n"),
        ("open-project", "<Primary>o"),
        ("close-project", "<Primary>w"),
        ("save", "<Primary>s"),
        ("format", "<Primary><Shift>f"),
        ("find-replace", "<Primary>f"),
        ("capture", "<Primary><Shift>c"),
        ("preview", "<Primary>r"),
        ("export", "<Primary><Shift>e"),
    ] {
        let action = gio::SimpleAction::new(name, None);
        application.add_action(&action);
        application.set_accels_for_action(&format!("app.{name}"), &[accelerator]);
    }
    file
}

fn connect_ui_actions(
    shell: &Rc<RefCell<UiShell>>,
    status: &Label,
    project_ui: &ProjectUi,
    application: &Application,
) {
    for (name, command) in [
        ("save", UiCommand::Save),
        ("format", UiCommand::Format),
        ("find-replace", UiCommand::FindReplace),
        ("capture", UiCommand::Capture),
        ("preview", UiCommand::Preview),
        ("export", UiCommand::Export),
    ] {
        let action = application.lookup_action(name).expect("installed application action");
        let action = action.downcast::<gio::SimpleAction>().expect("simple action");
        let shell = Rc::clone(shell);
        let status = status.clone();
        action
            .connect_activate(move |_, _| dispatch_and_announce(&shell, &status, command.clone()));
    }

    for (name, create) in [("new-project", true), ("open-project", false)] {
        let action = application.lookup_action(name).expect("installed project action");
        let action = action.downcast::<gio::SimpleAction>().expect("simple action");
        connect_project_action(&action, create, project_ui);
    }

    let action = application.lookup_action("close-project").expect("installed close action");
    let action = action.downcast::<gio::SimpleAction>().expect("simple action");
    let shell = Rc::clone(shell);
    let status = status.clone();
    let stack = project_ui.stack.clone();
    let source_buffer = project_ui.source_buffer.clone();
    let project_label = project_ui.project_label.clone();
    action.connect_activate(move |_, _| {
        match shell.borrow_mut().dispatch(UiCommand::CloseProject) {
            Ok(()) => {
                stack.set_visible_child_name("home");
                source_buffer.set_text("");
                project_label.set_text("No project open");
                status.set_text("Project closed. Create or open a project to begin.");
            }
            Err(error) => status.set_text(&format!("Error: {error}")),
        }
    });
}

fn connect_project_button(button: &Button, create: bool, project_ui: &ProjectUi) {
    let project_ui = project_ui.clone();
    button.connect_clicked(move |_| {
        choose_project_folder(create, &project_ui);
    });
}

fn connect_project_action(action: &gio::SimpleAction, create: bool, project_ui: &ProjectUi) {
    let project_ui = project_ui.clone();
    action.connect_activate(move |_, _| {
        choose_project_folder(create, &project_ui);
    });
}

#[derive(Clone)]
struct ProjectUi {
    window: ApplicationWindow,
    shell: Rc<RefCell<UiShell>>,
    status: Label,
    stack: Stack,
    source_buffer: sourceview::Buffer,
    project_label: Label,
}

fn choose_project_folder(create: bool, project_ui: &ProjectUi) {
    let dialog = gtk::FileDialog::builder()
        .title(if create { "Choose project folder" } else { "Open project folder" })
        .accept_label(if create { "Create here" } else { "Open" })
        .modal(true)
        .build();
    let project_ui = project_ui.clone();
    dialog.select_folder(Some(&project_ui.window), None::<&gio::Cancellable>, move |result| {
        let Ok(file) = result else {
            return;
        };
        let Some(path) = file.path() else {
            project_ui.status.set_text("The selected location is not a local filesystem path.");
            return;
        };
        let result = if create { create_loaded_project(&path) } else { load_project(&path) };
        match result {
            Ok(project) => {
                project_ui.source_buffer.set_text(&project.source);
                match project_ui.shell.borrow_mut().dispatch(UiCommand::OpenProject {
                    session: project.session.clone(),
                    settings: project.settings,
                }) {
                    Ok(()) => {
                        project_ui.project_label.set_text(&format!(
                            "{} · {}",
                            project.session.name,
                            path.display()
                        ));
                        project_ui.stack.set_visible_child_name("workspace");
                        project_ui.status.set_text(if create {
                            "Project created. Ready to edit."
                        } else {
                            "Project opened. Ready to edit."
                        });
                    }
                    Err(error) => project_ui.status.set_text(&format!("Error: {error}")),
                }
            }
            Err(error) => project_ui.status.set_text(&format!(
                "Could not {} project: {error}",
                if create { "create" } else { "open" }
            )),
        }
    });
}

struct LoadedProject {
    session: ProjectSession,
    settings: captee_core::ProjectSettings,
    source: String,
}

fn create_loaded_project(path: &Path) -> Result<LoadedProject, String> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Captee project");
    let config = ProjectConfig::new(name, "main.typ").map_err(|error| error.to_string())?;
    create_project(path, config).map_err(|error| error.to_string())?;
    load_project(path)
}

fn load_project(path: &Path) -> Result<LoadedProject, String> {
    let workspace = open_project(path).map_err(|error| error.to_string())?;
    let source_path = path.join(&workspace.config.entry_document);
    let source = fs::read_to_string(&source_path)
        .map_err(|error| format!("could not read {}: {error}", source_path.display()))?;
    Ok(LoadedProject {
        session: ProjectSession::new(
            path.to_string_lossy(),
            workspace.config.name.clone(),
            workspace.config.entry_document.clone(),
        ),
        settings: workspace.config.settings,
        source,
    })
}

fn dispatch_and_announce(shell: &Rc<RefCell<UiShell>>, status: &Label, command: UiCommand) {
    let result = shell.borrow_mut().dispatch(command);
    let snapshot = shell.borrow().snapshot();
    if let Err(error) = result {
        status.set_text(&format!("Error: {error}"));
    } else if let Some(announcement) = snapshot.announcement {
        status.set_text(&announcement.label);
    }
}
