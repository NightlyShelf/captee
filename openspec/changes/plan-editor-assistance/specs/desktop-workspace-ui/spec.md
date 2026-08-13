## ADDED Requirements

### Requirement: Tinymist editor assistance

The application SHALL use the bundled Tinymist language server to provide context-aware completion and current diagnostics in the main source editor and capture-review editor. Tinymist SHALL run locally for the active project and SHALL not replace the existing Typst compiler used for formatting, preview, and export. Completion and diagnostics SHALL not block normal editing or other authoring operations when Tinymist is unavailable. The application SHALL not expose a manual completion command, button, or keybinding.

#### Scenario: Request completion

- **WHEN** the user types `#` in a Typst editor
- **THEN** Tinymist matching candidates appear in a popup anchored near the cursor and update while the command prefix changes
- **AND** the user can select with Up/Down and accept with Enter, Tab, or pointer input

#### Scenario: Dismiss completion

- **WHEN** the completion popup is open and the user presses Escape or types unrelated input
- **THEN** the popup closes without changing the source

#### Scenario: Apply completion edit

- **WHEN** the user accepts a current Tinymist completion
- **THEN** only its returned text edit or replacement range changes
- **AND** a stale or empty response does not change the source

#### Scenario: Mark current compiler error

- **WHEN** Tinymist reports a current error or warning with a source range
- **THEN** the corresponding source range is marked with a severity-colored curvy underline
- **AND** hovering it exposes its diagnostic message

#### Scenario: Clear stale markers

- **WHEN** source changes or a newer render replaces diagnostics
- **THEN** markers from the older source revision are removed

#### Scenario: Summarize current errors

- **WHEN** current Tinymist diagnostics are displayed in the main editor
- **THEN** a tiny transparent indicator at the editor's bottom-right shows a centered white cross on red with the error count
- **AND** when there are no current errors it shows a centered white tick on green with “No errors”

#### Scenario: Restore the reopened workspace view

- **WHEN** the user closes and later reopens a project
- **THEN** the editor restores the saved caret and editor viewport from project-local hidden view state
- **AND** the preview restores its saved viewport after the first rendered page layout is ready
- **AND** ordinary edits and renders do not force either pane to a different position

#### Scenario: Tinymist unavailable

- **WHEN** Tinymist cannot start or exits unexpectedly
- **THEN** the application reports that editor assistance is unavailable
- **AND** editing, saving, formatting, preview, and export remain usable

### Requirement: Scrollable collapsed project tree

The project file list SHALL scroll vertically within the project panel without horizontal scrolling. Every newly opened project SHALL initially show all folders collapsed, while allowing folders to be expanded and collapsed for the rest of that project session.

#### Scenario: Open project tree

- **WHEN** a project with nested folders opens
- **THEN** every folder is collapsed and root entries remain visible

#### Scenario: Browse a large project tree

- **WHEN** visible project entries exceed the panel height
- **THEN** the user can scroll vertically to every visible entry
- **AND** long names remain truncated without horizontal scrolling

### Requirement: Confirm dirty application exit

The application SHALL prompt before exiting when the active source has unsaved changes. The prompt SHALL offer Save, Discard, and Cancel, and clean Save or Discard exits SHALL remove the autosave so the next normal run does not offer recovery. Autosave SHALL remain available for recovery after a crash or interrupted session.

#### Scenario: Save before exit

- **WHEN** the user chooses Save in the dirty-exit prompt
- **THEN** the source is saved atomically, the autosave is cleared, and the application exits
- **AND** a save failure is shown while the application remains open

#### Scenario: Discard before exit

- **WHEN** the user chooses Discard in the dirty-exit prompt
- **THEN** the autosave is cleared without writing dirty source to the project and the application exits

#### Scenario: Cancel exit

- **WHEN** the user chooses Cancel or closes the dirty-exit prompt
- **THEN** the application remains open with source and autosave unchanged

#### Scenario: Exit clean source

- **WHEN** the user exits with no unsaved source changes
- **THEN** the application exits without showing the dirty-exit prompt

### Requirement: Workspace menu organization

The workspace menu strip SHALL group project and output commands under File, editing and configuration commands under Edit, and preview presentation commands under View. Capture and Settings SHALL appear under Edit, Export PDF SHALL appear under File, and no separate Capture menu SHALL remain. About SHALL be a separate top-level menu button that opens application identity, version, license, repository, and bundled-tool acknowledgements.

#### Scenario: Show workspace command groups

- **WHEN** a project workspace is active
- **THEN** File contains Export PDF, Edit contains Capture and Settings, and View contains preview controls
- **AND** no separate Capture menu is shown

#### Scenario: Open About

- **WHEN** the user activates the separate About menu button
- **THEN** an About dialog shows Captee's name, version, GPL-3.0-or-later license, repository link, and Typst and Tinymist acknowledgements
