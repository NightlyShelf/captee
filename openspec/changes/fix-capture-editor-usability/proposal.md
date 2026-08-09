## Why

Capture keybindings are incorrectly stored per project and the registered system shortcut does not update after Settings changes. Keybindings must be user-global, persist outside project files, register only while a project is open, and default capture to Ctrl+~. Capture review should open at the editable annotation, and confirmed capture insertion should reveal its source location as well as the preview. This change fixes those regressions without changing project-local formatting, capture backend, or preview settings.
