//! Headless capture, annotation, and editor-insertion boundaries.

use std::fmt;

/// Raw image bytes returned by a capture backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedImage {
    bytes: Vec<u8>,
    selection: Option<SelectionGeometry>,
    background: Option<Vec<u8>>,
}

/// Screen-space region selected by an interactive capture backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl CapturedImage {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self { bytes: bytes.into(), selection: None, background: None }
    }

    pub fn with_selection(bytes: impl Into<Vec<u8>>, selection: SelectionGeometry) -> Self {
        Self { bytes: bytes.into(), selection: Some(selection), background: None }
    }

    pub fn with_selection_and_background(
        bytes: impl Into<Vec<u8>>,
        selection: SelectionGeometry,
        background: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            bytes: bytes.into(),
            selection: Some(selection),
            background: Some(background.into()),
        }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn selection(&self) -> Option<SelectionGeometry> {
        self.selection
    }

    pub fn background_bytes(&self) -> Option<&[u8]> {
        self.background.as_deref()
    }
}

/// Image bytes produced by an annotation operation and awaiting confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotatedImage {
    bytes: Vec<u8>,
}

impl AnnotatedImage {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self { bytes: bytes.into() }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// Lightweight annotation intent understood by platform image adapters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Annotation {
    Pointer { x: u32, y: u32 },
    Rectangle { x: u32, y: u32, width: u32, height: u32 },
    Text { x: u32, y: u32, text: String },
}

/// Typed outcome for a capture backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureResult<T> {
    Completed(T),
    Cancelled,
    Failed(CaptureError),
}

/// Typed outcome for an annotation operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotationResult<T> {
    Completed(T),
    Cancelled,
    Failed(AnnotationError),
}

/// Typed outcome for inserting an image expression into the focused editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertionResult {
    Inserted,
    NoFocusedEditor,
    Cancelled,
    Failed(InsertionError),
}

/// Capture backend boundary, implemented by portal and fallback adapters.
pub trait CaptureBackend {
    fn capture(&self) -> CaptureResult<CapturedImage>;
}

/// Annotation backend boundary, implemented by an in-memory image adapter.
pub trait AnnotationBackend {
    fn annotate(
        &self,
        image: &CapturedImage,
        annotation: &Annotation,
    ) -> AnnotationResult<AnnotatedImage>;
}

/// Focused-editor insertion boundary, implemented by the editor adapter.
pub trait EditorInserter {
    fn insert_image_expression(&mut self, expression: &str) -> InsertionResult;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureError {
    pub message: String,
}

impl CaptureError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for CaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CaptureError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotationError {
    pub message: String,
}

impl AnnotationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for AnnotationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AnnotationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsertionError {
    pub message: String,
}

impl InsertionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for InsertionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for InsertionError {}

#[cfg(test)]
mod tests {
    use super::*;

    struct CancelledCapture;

    impl CaptureBackend for CancelledCapture {
        fn capture(&self) -> CaptureResult<CapturedImage> {
            CaptureResult::Cancelled
        }
    }

    struct FailedAnnotation;

    impl AnnotationBackend for FailedAnnotation {
        fn annotate(
            &self,
            _image: &CapturedImage,
            _annotation: &Annotation,
        ) -> AnnotationResult<AnnotatedImage> {
            AnnotationResult::Failed(AnnotationError::new("annotation failed"))
        }
    }

    struct NoFocusedEditor;

    impl EditorInserter for NoFocusedEditor {
        fn insert_image_expression(&mut self, _expression: &str) -> InsertionResult {
            InsertionResult::NoFocusedEditor
        }
    }

    #[test]
    fn image_boundaries_preserve_owned_bytes_until_consumed() {
        let captured = CapturedImage::new(b"capture".to_vec());
        let annotated = AnnotatedImage::new(captured.bytes().to_vec());
        assert_eq!(captured.bytes(), b"capture");
        assert_eq!(annotated.into_bytes(), b"capture");
    }

    #[test]
    fn interfaces_expose_cancellation_failure_and_no_focus_outcomes() {
        assert_eq!(CancelledCapture.capture(), CaptureResult::Cancelled);
        assert_eq!(
            FailedAnnotation
                .annotate(&CapturedImage::new(Vec::new()), &Annotation::Pointer { x: 1, y: 2 }),
            AnnotationResult::Failed(AnnotationError::new("annotation failed"))
        );
        assert_eq!(
            NoFocusedEditor.insert_image_expression("#image(\"img/capture.png\")"),
            InsertionResult::NoFocusedEditor
        );
    }
}
