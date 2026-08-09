# Captee

Captee is local-first desktop workspace for Typst notes and annotated screenshots.

Unlike separate editors, screenshot tools, and PDF exporters, Captee keeps writing, live preview, capture, annotation, and export in one app. Projects and assets stay on your machine; no account or cloud service required.

## Features

- Typst editor with syntax highlighting, completion, diagnostics, and live preview
- Project file tree with safe create, move, rename, and delete actions
- Screen-region capture, annotation, and direct Typst insertion
- PDF export from latest successful preview
- Autosave, recovery, and global capture shortcut

## Install

Captee currently installs from source on Linux. Install [Rust 1.97.1](https://rustup.rs/), GTK 4, and GtkSourceView 5:

```sh
sudo apt-get install libgtk-4-dev libgtksourceview-5-dev
git clone https://github.com/NightlyShelf/captee.git
cd captee
./tools/fetch-typst.sh
```

## Build and run

```sh
cargo build --release -p captee-ui
./target/release/captee-ui
```

## Configuration

Recent projects: `$XDG_DATA_HOME/captee/recent-projects.json` (usually `~/.local/share/captee/`).
