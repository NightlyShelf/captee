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
