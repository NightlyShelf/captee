use captee_core::CapturedImage;

/// Headless state machine for the staged document-aware capture review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureReview {
    image: CapturedImage,
    annotation: String,
    before_image: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmedCapture {
    pub image: CapturedImage,
    pub annotation: String,
    pub before_image: bool,
}

impl CaptureReview {
    pub fn new(image: CapturedImage) -> Self {
        Self { image, annotation: String::new(), before_image: true }
    }

    pub fn image(&self) -> &CapturedImage {
        &self.image
    }

    pub fn annotation(&self) -> &str {
        &self.annotation
    }

    pub fn before_image(&self) -> bool {
        self.before_image
    }

    pub fn set_annotation(&mut self, annotation: impl Into<String>) {
        self.annotation = annotation.into();
    }

    pub fn replace_image(&mut self, image: CapturedImage) {
        self.image = image;
    }

    pub fn toggle_placement(&mut self) {
        self.before_image = !self.before_image;
    }

    pub fn confirm(&self) -> ConfirmedCapture {
        ConfirmedCapture {
            image: self.image.clone(),
            annotation: self.annotation.trim().to_owned(),
            before_image: self.before_image,
        }
    }

    pub fn discard(self) {}

    pub fn modify(self) -> CapturedImage {
        self.image
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use captee_core::SelectionGeometry;

    fn image() -> CapturedImage {
        CapturedImage::with_selection(
            b"capture".to_vec(),
            SelectionGeometry { x: 12, y: 24, width: 80, height: 60 },
        )
    }

    #[test]
    fn empty_review_confirms_image_only() {
        let review = CaptureReview::new(image());
        assert!(review.before_image());
        assert_eq!(review.image().selection().expect("selection").x, 12);
        let confirmed = review.confirm();
        assert_eq!(confirmed.annotation, "");
        assert_eq!(confirmed.image.bytes(), b"capture");
    }

    #[test]
    fn placement_toggle_and_confirmation_keep_staged_capture_immutable() {
        let mut review = CaptureReview::new(image());
        review.set_annotation("#line(length: 1em)");
        review.toggle_placement();

        let confirmed = review.confirm();
        assert!(!confirmed.before_image);
        assert_eq!(confirmed.annotation, "#line(length: 1em)");
        assert_eq!(confirmed.image.bytes(), b"capture");
        assert_eq!(review.image().bytes(), b"capture");
    }

    #[test]
    fn replacing_capture_keeps_annotation_and_placement() {
        let mut review = CaptureReview::new(image());
        review.set_annotation("#line(length: 1em)");
        review.toggle_placement();
        let replacement = CapturedImage::with_selection(
            b"replacement".to_vec(),
            SelectionGeometry { x: 32, y: 48, width: 120, height: 90 },
        );

        review.replace_image(replacement.clone());

        assert_eq!(review.annotation(), "#line(length: 1em)");
        assert!(!review.before_image());
        assert_eq!(review.image(), &replacement);
    }

    #[test]
    fn discard_and_modify_have_no_confirmation_side_effects() {
        let image = image();
        let modified = CaptureReview::new(image.clone()).modify();
        assert_eq!(modified, image);
        CaptureReview::new(image).discard();
    }
}
