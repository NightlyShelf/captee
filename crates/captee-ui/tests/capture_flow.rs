use captee_core::{
    Annotation, AnnotationResult, CaptureBackend, CaptureError, CaptureResult, CaptureSettings,
    CapturedImage, InsertionResult, OperationKind,
};
use captee_platform::{
    insert_saved_asset, AssetStore, CaptureSelector, PngAnnotationBackend, SavedAsset,
};
use captee_ui::annotation_bridge::AnnotationDraft;
use captee_ui::editor_bridge::{EditorBridge, EditorInsertionBridge};
use captee_ui::operation::{OperationCoordinator, OperationOutcome, ResultDisposition};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

struct FixedCapture {
    result: CaptureResult<CapturedImage>,
    calls: Arc<Mutex<u32>>,
}

impl CaptureBackend for FixedCapture {
    fn capture(&self) -> CaptureResult<CapturedImage> {
        *self.calls.lock().expect("capture calls") += 1;
        self.result.clone()
    }
}

fn backend(result: CaptureResult<CapturedImage>) -> (FixedCapture, Arc<Mutex<u32>>) {
    let calls = Arc::new(Mutex::new(0));
    (FixedCapture { result, calls: calls.clone() }, calls)
}

fn fixture_png() -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, 24, 16);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("PNG header");
    writer.write_image_data(&[240; 24 * 16 * 4]).expect("PNG pixels");
    drop(writer);
    bytes
}

fn test_root(name: &str) -> PathBuf {
    let suffix = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos();
    let root = std::env::temp_dir().join(format!("captee-ui-capture-{name}-{suffix}"));
    fs::create_dir_all(root.join("img")).expect("asset directory");
    root
}

fn coordinated_capture(
    selector: impl CaptureBackend,
) -> (OperationCoordinator<CapturedImage>, ResultDisposition<CapturedImage>) {
    let mut coordinator = OperationCoordinator::new();
    coordinator.activate_project("/tmp/capture-project").expect("active project");
    let task = coordinator.begin(OperationKind::Capture, true).expect("capture operation");
    let outcome = match selector.capture() {
        CaptureResult::Completed(image) => OperationOutcome::Completed(image),
        CaptureResult::Cancelled => OperationOutcome::Cancelled,
        CaptureResult::Failed(error) => OperationOutcome::Failed(error.to_string()),
    };
    task.finish(outcome).expect("capture result");
    let result = coordinator.try_next_result().expect("coordinated result");
    (coordinator, result)
}

fn completed_image(result: ResultDisposition<CapturedImage>) -> CapturedImage {
    let ResultDisposition::Current(result) = result else {
        panic!("capture result should be current");
    };
    let OperationOutcome::Completed(image) = result.outcome else {
        panic!("capture should complete");
    };
    image
}

fn store_and_insert(root: &PathBuf, image: captee_core::AnnotatedImage) -> (SavedAsset, String) {
    let asset = AssetStore::new(root).expect("asset store").save_png(image).expect("saved asset");
    let mut editor = EditorBridge::new("main.typ", "before after");
    let mut insertion = EditorInsertionBridge::new(Some(&mut editor), 7);
    assert_eq!(insert_saved_asset(&asset, Some(&mut insertion)), InsertionResult::Inserted);
    (asset, editor.state().text)
}

#[test]
fn portal_capture_annotation_storage_and_insertion_complete_as_one_flow() {
    let original = fixture_png();
    let (portal, _) = backend(CaptureResult::Completed(CapturedImage::new(original.clone())));
    let (fallback, fallback_calls) =
        backend(CaptureResult::Failed(CaptureError::new("fallback must not run")));
    let selector = CaptureSelector::new(portal, fallback, CaptureSettings::default());
    let (_, result) = coordinated_capture(selector);
    let captured = completed_image(result);
    let mut draft = AnnotationDraft::new(captured);
    assert_eq!(
        draft.apply(
            &PngAnnotationBackend::new(),
            &Annotation::Rectangle { x: 2, y: 2, width: 12, height: 8 },
        ),
        AnnotationResult::Completed(())
    );
    assert_eq!(draft.original().bytes(), original.as_slice());
    assert_ne!(draft.staged().bytes(), original.as_slice());

    let root = test_root("portal");
    let (asset, source) = store_and_insert(&root, draft.into_confirmed());
    assert!(root.join(asset.relative_path()).is_file());
    assert_eq!(source, format!("before {}after", asset.typst_image_expression()));
    assert_eq!(*fallback_calls.lock().expect("fallback calls"), 0);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn portal_failure_uses_fallback_capture_in_the_connected_flow() {
    let (portal, portal_calls) =
        backend(CaptureResult::Failed(CaptureError::new("portal unavailable")));
    let (fallback, fallback_calls) =
        backend(CaptureResult::Completed(CapturedImage::new(fixture_png())));
    let selector = CaptureSelector::new(portal, fallback, CaptureSettings::default());
    let (_, result) = coordinated_capture(selector);
    let captured = completed_image(result);

    assert!(!captured.bytes().is_empty());
    assert_eq!(*portal_calls.lock().expect("portal calls"), 1);
    assert_eq!(*fallback_calls.lock().expect("fallback calls"), 1);
}

#[test]
fn capture_cancellation_never_reaches_annotation_storage_or_insertion() {
    let (portal, _) = backend(CaptureResult::Cancelled);
    let (fallback, fallback_calls) =
        backend(CaptureResult::Completed(CapturedImage::new(fixture_png())));
    let selector = CaptureSelector::new(portal, fallback, CaptureSettings::default());
    let (_, result) = coordinated_capture(selector);
    let ResultDisposition::Current(result) = result else {
        panic!("cancellation should be current");
    };
    assert_eq!(result.outcome, OperationOutcome::Cancelled);
    assert_eq!(*fallback_calls.lock().expect("fallback calls"), 0);

    let root = test_root("cancel");
    let editor = EditorBridge::new("main.typ", "unchanged");
    let before = editor.state();
    assert_eq!(fs::read_dir(root.join("img")).expect("images").count(), 0);
    assert_eq!(editor.state(), before);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn malformed_capture_fails_annotation_without_creating_an_asset() {
    let mut draft = AnnotationDraft::new(CapturedImage::new(b"not a PNG"));
    assert!(matches!(
        draft.apply(&PngAnnotationBackend::new(), &Annotation::Pointer { x: 1, y: 1 }),
        AnnotationResult::Failed(_)
    ));
    let root = test_root("malformed");
    assert!(AssetStore::new(&root).expect("store").save_png(draft.into_confirmed()).is_err());
    assert_eq!(fs::read_dir(root.join("img")).expect("images").count(), 0);
    fs::remove_dir_all(root).expect("cleanup");
}
