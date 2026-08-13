## Why

Capture keybindings are incorrectly stored per project and the registered system shortcut does not update after Settings changes. Capture review navigation also fails to keep the editable annotation and inserted content reliably visible.

## What Changes

Make keybindings user-global, register capture only while a project is open, and default capture to Ctrl+~. Remove horizontal capture-review scrolling, position the annotation near the editor bottom, and reveal confirmed insertion in both source and preview without changing project-local formatting, capture backend, or preview settings.
