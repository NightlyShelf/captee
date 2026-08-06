## ADDED Requirements

### Requirement: Connected capture and annotation workflow

The workspace MUST connect portal-first or configured fallback capture to annotation, confirmation, validated PNG storage, and editor insertion without mutating project state on cancellation.

#### Scenario: Capture is inserted after confirmation

- **WHEN** the user captures a region, adds optional annotations, and confirms
- **THEN** a validated PNG is atomically stored under the active project image directory and a safe Typst image expression is inserted at the focused editor position

#### Scenario: Capture cancellation is a no-op

- **WHEN** the user cancels region selection or annotation
- **THEN** no image file or source edit is created

#### Scenario: Capture backend failure is actionable

- **WHEN** the portal and configured fallback cannot capture
- **THEN** the UI reports the failure and returns to idle without changing the project

### Requirement: Connected annotation controls

The UI MUST provide controls for pointer, rectangle, and text annotations while preserving the original capture until confirmation.

#### Scenario: Annotation remains reversible before confirmation

- **WHEN** the user changes or cancels an annotation
- **THEN** the original captured image remains available and no project asset is written

### Requirement: Document-aware capture review

After region selection succeeds, the application SHALL retain the selected region and its available screen-space geometry, preserve the existing screen surface and selection stroke, and add a borderless transparent fullscreen review surface on that same capture monitor rather than rerendering the captured region as a background image or placing the review inside the main workspace window. The review surface SHALL visibly darken the surrounding screen and contain only the preceding Typst context, an annotation code editor, and the three bottom controls Before/After, Confirm, and Cancel. The preceding Typst lines SHALL be rendered as dimmed context, while the annotation editor SHALL support full Typst syntax. A separate Modify action SHALL reopen region selection without committing the staged capture. The default insertion order SHALL place annotation code before the image block.

#### Scenario: Region selection becomes a staged review

- **WHEN** the user completes a region drag
- **THEN** the selected screen region and stroke remain visible under the dimmed screen, the transparent review surface is added beside it on the capture monitor, and no duplicate captured-image background is rendered, without changing the project source or writing an asset

#### Scenario: Default annotation placement

- **WHEN** the staged review opens
- **THEN** the annotation code position is before the staged image block and the review retains the default Before state

#### Scenario: Move annotation after the image

- **WHEN** the user activates the Before/After control
- **THEN** the staged insertion order changes to place the annotation code after the image block

#### Scenario: Confirm with keyboard

- **WHEN** the staged review is focused and the user presses Enter
- **THEN** the review is confirmed using the selected order, the image is stored, and the annotation plus image Typst blocks are inserted at the proposed document position

#### Scenario: Discard with keyboard

- **WHEN** the staged review is open and the user presses Escape
- **THEN** the review is discarded, the selected image is released, and no project asset or source edit is created

#### Scenario: Modify the selected region

- **WHEN** the user activates Modify in the staged review
- **THEN** the region-selection interaction reopens for the current capture so the user can reselect the image before returning to review

#### Scenario: Start capture without workspace focus

- **WHEN** the user invokes the registered global capture shortcut while another application is focused
- **THEN** Captee starts region selection without requiring the user to switch to or focus the Captee window first
