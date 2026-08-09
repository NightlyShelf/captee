# screenshot-annotation Specification

## Purpose

Captures a screen region, lets the user add lightweight markup and explanation, and inserts a portable image reference into the focused Typst document.
## Requirements
### Requirement: Capture backends and cancellation

The application SHALL prefer the desktop portal capture flow and SHALL provide a configured `grim`/`slurp` fallback, while treating cancellation as a no-op.

#### Scenario: Portal capture

- **WHEN** the portal returns a selected region image
- **THEN** the image is offered for annotation without changing the project

#### Scenario: Fallback capture

- **WHEN** the portal is unavailable and fallback is enabled
- **THEN** the configured fallback backend is attempted and its failure is reported without mutating the project

#### Scenario: Cancel capture

- **WHEN** the user cancels region selection or annotation
- **THEN** no image file or Typst source change is created

### Requirement: Annotation and asset storage

The application SHALL support pointer, rectangle, and text annotations, preserve the original capture until confirmation, and save the confirmed result as a PNG under the active project's `img/` directory.

#### Scenario: Confirm annotated image

- **WHEN** a user confirms an annotated capture
- **THEN** a unique PNG is atomically written under `img/` and the resulting project-relative path is returned

#### Scenario: Reject invalid image output

- **WHEN** the annotation pipeline cannot produce a valid PNG
- **THEN** the operation fails and no partial asset is left in the project

### Requirement: Typst insertion

The application SHALL insert a reference to the confirmed project-relative image at the focused editor position only after asset storage succeeds.

#### Scenario: Insert image reference

- **WHEN** asset storage succeeds and an editor focus is available
- **THEN** the editor receives a valid Typst image expression referencing the new asset

#### Scenario: No focused editor

- **WHEN** a capture is confirmed without a focused Typst editor
- **THEN** the image remains saved in the project and the application reports that insertion was skipped

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

Before and after region selection, the application SHALL retain the active desktop workspace context when the platform exposes it. After selection succeeds, it SHALL open only a small borderless floating review popup at the selection's screen coordinates on that same workspace, without preserving the selection stroke, rerendering the captured region, dimming the desktop, or placing a monitor-sized surface over the active window. The popup SHALL contain the preceding Typst context, an annotation code editor, and the bottom controls Before/After, Confirm, and Cancel. The annotation editor SHALL support full Typst syntax and show its default description inside the textbox until the user types. A separate Modify action SHALL reopen region selection without committing the staged capture. The default insertion order SHALL place annotation code before the image block.

#### Scenario: Region selection becomes a staged review

- **WHEN** the user completes a region drag
- **THEN** a small borderless review popup appears at the selection coordinates on the capture workspace, the live active window remains unchanged behind it, and no selection overlay, desktop screenshot, or source/asset mutation is introduced before confirmation

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

### Requirement: Unified capture review editor

The capture review SHALL show one Typst editor with the preceding document context grayed and the editable annotation at the insertion point. It SHALL preserve the original document line numbers, hide selection coordinates, align the placeholder with editor text, and clear the placeholder when annotation input begins. An empty annotation SHALL be valid and insert only the confirmed image.

#### Scenario: Edit capture annotation in context

- **WHEN** capture review opens
- **THEN** the user sees prior Typst context grayed in the same editor as the annotation input
- **AND** the annotation input is aligned with its line number

#### Scenario: Type an annotation

- **WHEN** the user enters annotation text
- **THEN** the placeholder disappears and only entered text is inserted with the image

#### Scenario: Confirm an empty annotation

- **WHEN** the user confirms without annotation text
- **THEN** only the image expression is inserted

#### Scenario: Modify a staged capture

- **WHEN** the user activates Modify and completes a new region selection
- **THEN** the staged annotation text and before/after placement are preserved in the returning review

#### Scenario: Return to inserted capture

- **WHEN** a staged capture is confirmed and inserted
- **THEN** the source editor focuses the inserted image location
- **AND** the preview scrolls to its end
