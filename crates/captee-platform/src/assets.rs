//! Validated, collision-resistant storage for confirmed capture assets.

use crate::atomic::{atomic_create, AtomicWriteError};
use crate::{PathError, ProjectPaths};
use captee_core::AnnotatedImage;
use png::Decoder;
use std::fmt;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const IMAGE_DIRECTORY: &str = crate::workspace::IMAGE_DIRECTORY;
const MAX_ASSET_BYTES: usize = 32 * 1024 * 1024;
const MAX_ASSET_PIXELS: usize = 16 * 1024 * 1024;
const MAX_DECODED_BYTES: usize = 64 * 1024 * 1024;
const MAX_NAME_ATTEMPTS: usize = 128;
static ASSET_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Stores confirmed annotated PNGs below a project root.
#[derive(Debug, Clone)]
pub struct AssetStore {
    paths: ProjectPaths,
}

impl AssetStore {
    pub fn new(project_root: impl AsRef<Path>) -> Result<Self, AssetError> {
        Ok(Self { paths: ProjectPaths::open(project_root).map_err(AssetError::Path)? })
    }

    /// Validates and atomically creates an asset, returning its project-relative path.
    pub fn save_png(&self, image: AnnotatedImage) -> Result<SavedAsset, AssetError> {
        let bytes = image.into_bytes();
        validate_png(&bytes)?;
        let image_directory =
            self.paths.require_directory(IMAGE_DIRECTORY).map_err(AssetError::Path)?;
        if !image_directory.exists() {
            return Err(AssetError::Path(PathError::ExpectedDirectory(image_directory)));
        }

        for _ in 0..MAX_NAME_ATTEMPTS {
            let relative_path = next_asset_name();
            let destination = self.paths.resolve(&relative_path).map_err(AssetError::Path)?;
            match atomic_create(&destination, &bytes) {
                Ok(()) => {
                    return Ok(SavedAsset { relative_path });
                }
                Err(AtomicWriteError::Io(error))
                    if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(AssetError::Atomic(error)),
            }
        }

        Err(AssetError::NameExhausted(image_directory))
    }
}

/// A project-relative path for a successfully stored capture asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedAsset {
    relative_path: PathBuf,
}

impl SavedAsset {
    pub fn relative_path(&self) -> &Path {
        &self.relative_path
    }
}

fn validate_png(bytes: &[u8]) -> Result<(), AssetError> {
    if bytes.is_empty() || bytes.len() > MAX_ASSET_BYTES {
        return Err(AssetError::InvalidPng(format!(
            "PNG must be between 1 byte and {MAX_ASSET_BYTES} bytes"
        )));
    }

    let decoder = Decoder::new(Cursor::new(bytes));
    let mut reader = decoder
        .read_info()
        .map_err(|error| AssetError::InvalidPng(format!("could not read PNG header: {error}")))?;
    let info = reader.info();
    let pixels = (info.width as usize)
        .checked_mul(info.height as usize)
        .filter(|pixels| *pixels <= MAX_ASSET_PIXELS)
        .ok_or_else(|| AssetError::InvalidPng("PNG dimensions are too large".to_owned()))?;
    if pixels == 0 {
        return Err(AssetError::InvalidPng("PNG dimensions cannot be zero".to_owned()));
    }
    let output_size = reader.output_buffer_size();
    if output_size > MAX_DECODED_BYTES {
        return Err(AssetError::InvalidPng("decoded PNG is too large".to_owned()));
    }
    let mut output = vec![0; output_size];
    reader
        .next_frame(&mut output)
        .map_err(|error| AssetError::InvalidPng(format!("could not read PNG frame: {error}")))?;
    Ok(())
}

fn next_asset_name() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let sequence = ASSET_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    PathBuf::from(format!(
        "{IMAGE_DIRECTORY}/capture-{stamp}-{}-{sequence}.png",
        std::process::id()
    ))
}

#[derive(Debug)]
pub enum AssetError {
    Path(PathError),
    InvalidPng(String),
    Atomic(AtomicWriteError),
    NameExhausted(PathBuf),
}

impl fmt::Display for AssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path(error) => write!(formatter, "asset path is invalid: {error}"),
            Self::InvalidPng(message) => write!(formatter, "invalid PNG asset: {message}"),
            Self::Atomic(error) => write!(formatter, "asset write failed: {error}"),
            Self::NameExhausted(path) => {
                write!(formatter, "could not allocate a unique asset name in {}", path.display())
            }
        }
    }
}

impl std::error::Error for AssetError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos();
        let root = std::env::temp_dir().join(format!("captee-assets-{name}-{suffix}"));
        fs::create_dir_all(root.join(IMAGE_DIRECTORY)).expect("temporary asset directory");
        root
    }

    fn fixture_png() -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("PNG header");
        writer.write_image_data(&[255; 16]).expect("PNG pixels");
        drop(writer);
        bytes
    }

    #[test]
    fn saves_valid_png_with_unique_project_relative_name() {
        let root = test_root("valid");
        let store = AssetStore::new(&root).expect("store");
        let first = store.save_png(AnnotatedImage::new(fixture_png())).expect("first asset");
        let second = store.save_png(AnnotatedImage::new(fixture_png())).expect("second asset");

        assert_ne!(first, second);
        assert!(first.relative_path().starts_with(IMAGE_DIRECTORY));
        assert!(first.relative_path().extension().is_some_and(|extension| extension == "png"));
        assert_eq!(fs::read(root.join(first.relative_path())).expect("first file"), fixture_png());
        assert_eq!(fs::read_dir(root.join(IMAGE_DIRECTORY)).expect("asset directory").count(), 2);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn rejects_malformed_png_without_creating_an_asset() {
        let root = test_root("malformed");
        let store = AssetStore::new(&root).expect("store");
        let result = store.save_png(AnnotatedImage::new(b"not a png"));

        assert!(matches!(result, Err(AssetError::InvalidPng(_))));
        assert_eq!(fs::read_dir(root.join(IMAGE_DIRECTORY)).expect("asset directory").count(), 0);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn rejects_missing_asset_directory_without_mutating_project() {
        let root = std::env::temp_dir().join(format!(
            "captee-assets-missing-{}",
            SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos()
        ));
        fs::create_dir_all(&root).expect("temporary root");
        let store = AssetStore::new(&root).expect("store");
        let result = store.save_png(AnnotatedImage::new(fixture_png()));

        assert!(matches!(result, Err(AssetError::Path(PathError::ExpectedDirectory(_)))));
        assert_eq!(fs::read_dir(&root).expect("project root").count(), 0);
        fs::remove_dir_all(root).expect("cleanup");
    }
}
