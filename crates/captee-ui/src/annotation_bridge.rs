use captee_core::{AnnotatedImage, Annotation, AnnotationBackend, AnnotationResult, CapturedImage};

/// UI-owned reversible annotation draft. The original encoded capture is kept
/// immutable while each accepted mark replaces only the staged image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotationDraft {
    original: CapturedImage,
    staged: CapturedImage,
}

impl AnnotationDraft {
    pub fn new(original: CapturedImage) -> Self {
        let staged = original.clone();
        Self { original, staged }
    }

    pub fn original(&self) -> &CapturedImage {
        &self.original
    }

    pub fn staged(&self) -> &CapturedImage {
        &self.staged
    }

    pub fn apply(
        &mut self,
        backend: &impl AnnotationBackend,
        annotation: &Annotation,
    ) -> AnnotationResult<()> {
        match backend.annotate(&self.staged, annotation) {
            AnnotationResult::Completed(image) => {
                self.staged = CapturedImage::new(image.into_bytes());
                AnnotationResult::Completed(())
            }
            AnnotationResult::Cancelled => AnnotationResult::Cancelled,
            AnnotationResult::Failed(error) => AnnotationResult::Failed(error),
        }
    }

    pub fn reset(&mut self) {
        self.staged = self.original.clone();
    }

    pub fn replace_staged(&mut self, image: AnnotatedImage) {
        self.staged = CapturedImage::new(image.into_bytes());
    }

    pub fn confirmed(&self) -> AnnotatedImage {
        AnnotatedImage::new(self.staged.bytes().to_vec())
    }

    pub fn into_confirmed(self) -> AnnotatedImage {
        AnnotatedImage::new(self.staged.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use captee_core::AnnotationError;

    struct AppendBackend;

    impl AnnotationBackend for AppendBackend {
        fn annotate(
            &self,
            image: &CapturedImage,
            _annotation: &Annotation,
        ) -> AnnotationResult<AnnotatedImage> {
            let mut bytes = image.bytes().to_vec();
            bytes.extend_from_slice(b" marked");
            AnnotationResult::Completed(AnnotatedImage::new(bytes))
        }
    }

    struct FailedBackend;

    impl AnnotationBackend for FailedBackend {
        fn annotate(
            &self,
            _image: &CapturedImage,
            _annotation: &Annotation,
        ) -> AnnotationResult<AnnotatedImage> {
            AnnotationResult::Failed(AnnotationError::new("failed"))
        }
    }

    #[test]
    fn applying_and_resetting_marks_never_mutates_the_original() {
        let mut draft = AnnotationDraft::new(CapturedImage::new(b"original"));
        assert_eq!(
            draft.apply(&AppendBackend, &Annotation::Pointer { x: 1, y: 2 }),
            AnnotationResult::Completed(())
        );
        assert_eq!(draft.original().bytes(), b"original");
        assert_eq!(draft.staged().bytes(), b"original marked");

        draft.reset();
        assert_eq!(draft.original().bytes(), b"original");
        assert_eq!(draft.staged().bytes(), b"original");
    }

    #[test]
    fn failed_annotation_keeps_the_previous_stage() {
        let mut draft = AnnotationDraft::new(CapturedImage::new(b"original"));
        assert!(matches!(
            draft.apply(&FailedBackend, &Annotation::Text { x: 0, y: 0, text: "note".into() }),
            AnnotationResult::Failed(_)
        ));
        assert_eq!(draft.original().bytes(), b"original");
        assert_eq!(draft.staged().bytes(), b"original");
    }
}
