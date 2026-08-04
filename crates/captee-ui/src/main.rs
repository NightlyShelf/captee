//! Desktop application entry point.
//!
//! The presentation model is kept in `captee_ui::UiShell` so the GTK adapter
//! can be compiled and exercised independently from project side effects.

use captee_ui::UiShell;

fn main() {
    let _shell = UiShell::new();
}
