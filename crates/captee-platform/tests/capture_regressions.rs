use captee_core::{
    CaptureBackend, CaptureResult, CaptureSettings, CapturedImage, EditorInserter, InsertionResult,
};
use captee_platform::{insert_saved_asset, AssetError, AssetStore, CaptureSelector};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

struct FakeBackend {
    result: CaptureResult<CapturedImage>,
    calls: Arc<Mutex<u32>>,
}

impl CaptureBackend for FakeBackend {
    fn capture(&self) -> CaptureResult<CapturedImage> {
        *self.calls.lock().expect("backend calls") += 1;
        self.result.clone()
    }
}

#[derive(Default)]
struct RecordingEditor {
    expression: Option<String>,
}

impl EditorInserter for RecordingEditor {
    fn insert_image_expression(&mut self, expression: &str) -> InsertionResult {
        self.expression = Some(expression.to_owned());
        InsertionResult::Inserted
    }
}

fn test_root(name: &str) -> PathBuf {
    let suffix = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos();
    let root = std::env::temp_dir().join(format!("captee-platform-capture-{name}-{suffix}"));
    fs::create_dir_all(root.join("img")).expect("asset directory");
    root
}

fn fixture_png() -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("PNG header");
    writer.write_image_data(&[255, 255, 255, 255]).expect("PNG pixel");
    drop(writer);
    bytes
}

#[test]
fn portal_cancellation_does_not_invoke_fallback() {
    let portal_calls = Arc::new(Mutex::new(0));
    let fallback_calls = Arc::new(Mutex::new(0));
    let selector = CaptureSelector::new(
        FakeBackend { result: CaptureResult::Cancelled, calls: portal_calls.clone() },
        FakeBackend {
            result: CaptureResult::Completed(CapturedImage::new(b"fallback")),
            calls: fallback_calls.clone(),
        },
        CaptureSettings::default(),
    );

    assert_eq!(selector.capture(), CaptureResult::Cancelled);
    assert_eq!(*portal_calls.lock().expect("portal calls"), 1);
    assert_eq!(*fallback_calls.lock().expect("fallback calls"), 0);
}

#[test]
fn malformed_asset_is_rejected_without_project_mutation() {
    let root = test_root("malformed");
    let store = AssetStore::new(&root).expect("asset store");

    assert!(matches!(
        store.save_png(captee_core::AnnotatedImage::new(b"malformed")),
        Err(AssetError::InvalidPng(_))
    ));
    assert_eq!(fs::read_dir(root.join("img")).expect("asset directory").count(), 0);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn stored_asset_insertion_uses_exact_project_relative_expression() {
    let root = test_root("insertion");
    let store = AssetStore::new(&root).expect("asset store");
    let asset = store.save_png(captee_core::AnnotatedImage::new(fixture_png())).expect("asset");
    let mut editor = RecordingEditor::default();
    let expected = asset.typst_image_expression();

    assert_eq!(insert_saved_asset(&asset, Some(&mut editor)), InsertionResult::Inserted);
    assert_eq!(editor.expression.as_deref(), Some(expected.as_str()));
    fs::remove_dir_all(root).expect("cleanup");
}
