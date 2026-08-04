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
