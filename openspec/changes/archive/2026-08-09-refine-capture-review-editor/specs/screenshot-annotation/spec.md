## ADDED Requirements

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
