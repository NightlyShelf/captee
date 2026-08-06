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

After region selection succeeds, the application SHALL retain the selected region and present a staged review surface at the place where the user ended dragging. The review surface SHALL visibly darken the surrounding window and show a few preceding Typst lines, the proposed insertion location, an annotation code editor, and the image block as a document composition. The surrounding or unaffected Typst content and the image block SHALL be visually dimmed to distinguish staged context from editable annotation content. The annotation editor SHALL support full Typst syntax, and the default insertion order SHALL place annotation code before the image block.

#### Scenario: Region selection becomes a staged review

- **WHEN** the user completes a region drag
- **THEN** the selected image remains visible, the background around the review surface is darkened, and the review shows the prior Typst context, annotation editor, and image block without changing the project source or writing an asset

#### Scenario: Default annotation placement

- **WHEN** the staged review opens
- **THEN** the annotation code position is before the image block and the image block is shown dimmed as the downstream staged document content

#### Scenario: Move annotation after the image

- **WHEN** the user activates the Before/After control
- **THEN** the staged document order changes to place the annotation code after the image block, and the dimmed context updates to match

#### Scenario: Confirm with keyboard

- **WHEN** the staged review is focused and the user presses Enter
- **THEN** the review is confirmed using the selected order, the image is stored, and the annotation plus image Typst blocks are inserted at the proposed document position

#### Scenario: Discard with keyboard

- **WHEN** the staged review is open and the user presses Escape
- **THEN** the review is discarded, the selected image is released, and no project asset or source edit is created

#### Scenario: Modify the selected region

- **WHEN** the user activates Modify in the staged review
- **THEN** the region-selection interaction reopens for the current capture so the user can reselect the image before returning to review
