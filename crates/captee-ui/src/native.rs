use crate::{UiCommand, UiShell};
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Label, Orientation, Paned, ScrolledWindow,
};
use gtk4 as gtk;
use sourceview::prelude::*;
use sourceview5 as sourceview;
use std::cell::RefCell;
use std::rc::Rc;

const APPLICATION_ID: &str = "com.nightlyshelf.Captee";

/// Starts the GTK application and builds the accessible three-pane workspace.
pub fn run() -> glib::ExitCode {
    let application = Application::builder().application_id(APPLICATION_ID).build();
    application.connect_activate(build_ui);
    application.run()
}

fn build_ui(application: &Application) {
    install_actions(application);
    let shell = Rc::new(RefCell::new(UiShell::new()));
    let window = ApplicationWindow::builder()
        .application(application)
        .title("Captee")
        .default_width(1280)
        .default_height(800)
        .build();

    let navigation = GtkBox::new(Orientation::Vertical, 12);
    navigation.set_margin_top(16);
    navigation.set_margin_bottom(16);
    navigation.set_margin_start(16);
    navigation.set_margin_end(16);
    navigation.set_width_request(220);
    navigation.append(&Label::new(Some("Projects")));
    navigation.append(&Label::new(Some("Open or create a project")));

    let source_buffer = sourceview::Buffer::builder().highlight_matching_brackets(true).build();
    let source_view = sourceview::View::with_buffer(&source_buffer);
    source_view.set_show_line_numbers(true);
    source_view.set_monospace(true);
    source_view.set_hexpand(true);
    source_view.set_vexpand(true);
    source_view.set_tooltip_text(Some("Typst source editor"));
    let editor_scroll =
        ScrolledWindow::builder().child(&source_view).hexpand(true).vexpand(true).build();

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

    let status = Label::new(Some("Ready"));
    status.set_xalign(0.0);
    status.set_margin_start(16);
    status.set_margin_end(16);
    status.set_margin_top(8);
    status.set_margin_bottom(8);
    status.set_tooltip_text(Some("Accessible operation status"));

    let root = GtkBox::new(Orientation::Vertical, 0);
    root.append(&workspace);
    root.append(&status);
    window.set_child(Some(&root));

    connect_ui_actions(&shell, &status, application);
    window.present();
}

fn install_actions(application: &Application) {
    let menu = gio::Menu::new();
    let file = gio::Menu::new();
    file.append(Some("Save"), Some("app.save"));
    file.append(Some("Format"), Some("app.format"));
    file.append(Some("Find and Replace"), Some("app.find-replace"));
    file.append(Some("Capture"), Some("app.capture"));
    file.append(Some("Preview"), Some("app.preview"));
    file.append(Some("Export PDF"), Some("app.export"));
    menu.append_submenu(Some("File"), &file);
    application.set_menubar(Some(&menu));

    for (name, accelerator) in [
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
}

fn connect_ui_actions(shell: &Rc<RefCell<UiShell>>, status: &Label, application: &Application) {
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
