//! GTK application entry point.
//!
//! GTK wiring is intentionally introduced after the headless state and command
//! layers are implemented. Keeping this binary minimal prevents UI concerns
//! from leaking into domain tests.

fn main() {
    println!("Captee UI scaffold");
}
