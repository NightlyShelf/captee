//! Desktop application entry point.
//!
//! The presentation model is kept in `captee_ui::UiShell` so the GTK adapter
//! can be compiled and exercised independently from project side effects.

fn main() {
    captee_ui::native::run();
}
