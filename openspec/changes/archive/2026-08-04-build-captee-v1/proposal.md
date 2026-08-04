## Why

Typst users need a fast, local way to turn screen context into portable notes. Existing screenshot and document tools do not provide a focused flow for annotating a captured region directly into a Typst project while retaining live source feedback.

## What Changes

- Introduce a Linux-first desktop application for local Typst projects, distributed first as an x86_64 AppImage.
- Add project creation, opening, configuration, secure file management, recent projects, and autosave.
- Add a Typst source editor with live compiler diagnostics, formatting, completions, simple find/replace, and a rendered preview with PDF export.
- Add global-shortcut screenshot capture with portal support and a `grim`/`slurp` fallback, markup annotation, and insertion into the focused Typst file.
- Add settings for keybindings, formatting, capture fallback, and project-local preview preferences.

## Capabilities

### New Capabilities

- `workspace-management`: Manage local Captee project folders, files, recent projects, and project settings safely.
- `typst-authoring`: Edit Typst source with live diagnostics, formatting, completion, and search.
- `document-preview-export`: Render the selected Typst entry document and export it to PDF.
- `screenshot-annotation`: Capture, annotate, and insert screenshots into a focused Typst document.
- `desktop-workspace-ui`: Provide the project home, menus, three-pane workspace, controls, and settings.

### Modified Capabilities

- None.

## Impact

- Adds a Rust workspace, GTK 4/GtkSourceView UI, bundled Typst compiler and formatter, Linux capture integrations, and GitHub Actions CI.
- Creates and modifies only user-selected project folders; screenshots are stored as PNG files within each project's `img/` directory.
- Requires platform-safe filesystem, system-trash, portal, and optional external capture-tool adapters.
