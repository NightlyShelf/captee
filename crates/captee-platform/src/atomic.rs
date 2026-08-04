//! Atomic project persistence and autosave recovery.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Replaces a file only after the complete contents are flushed successfully.
pub fn atomic_write(path: impl AsRef<Path>, contents: &[u8]) -> Result<(), AtomicWriteError> {
    let path = path.as_ref();
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(AtomicWriteError::Io)?;
    let temporary = temporary_path(path);

    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(AtomicWriteError::Io)?;
        file.write_all(contents).map_err(AtomicWriteError::Io)?;
        file.sync_all().map_err(AtomicWriteError::Io)?;
        fs::rename(&temporary, path).map_err(AtomicWriteError::Io)?;
        sync_directory(parent).map_err(AtomicWriteError::Io)?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// Creates a file only when the destination does not already exist, after the
/// complete contents have been flushed to a same-directory temporary file.
///
/// Linking the complete temporary file into place is atomic and cannot replace
/// an existing asset. The temporary link is removed after the destination is
/// installed.
pub(crate) fn atomic_create(
    path: impl AsRef<Path>,
    contents: &[u8],
) -> Result<(), AtomicWriteError> {
    let path = path.as_ref();
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let temporary = temporary_path(path);

    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(AtomicWriteError::Io)?;
        file.write_all(contents).map_err(AtomicWriteError::Io)?;
        file.sync_all().map_err(AtomicWriteError::Io)?;
        fs::hard_link(&temporary, path).map_err(AtomicWriteError::Io)?;
        fs::remove_file(&temporary).map_err(AtomicWriteError::Io)?;
        sync_directory(parent).map_err(AtomicWriteError::Io)?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// Writes a revisioned autosave and can recover it without touching the main file.
#[derive(Debug, Clone)]
pub struct AutosaveStore {
    path: PathBuf,
}

impl AutosaveStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn write(&self, revision: u64, contents: &[u8]) -> Result<(), AtomicWriteError> {
        let mut payload = format!("CAPTEE-AUTOSAVE 1 {revision}\n").into_bytes();
        payload.extend_from_slice(contents);
        atomic_write(&self.path, &payload)
    }

    pub fn recover(&self) -> Result<Option<AutosaveSnapshot>, AtomicWriteError> {
        if !self.path.exists() {
            return Ok(None);
        }
        let mut bytes = Vec::new();
        File::open(&self.path)
            .and_then(|mut file| file.read_to_end(&mut bytes))
            .map_err(AtomicWriteError::Io)?;
        let newline = bytes
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or(AtomicWriteError::MalformedAutosave)?;
        let header = std::str::from_utf8(&bytes[..newline])
            .map_err(|_| AtomicWriteError::MalformedAutosave)?;
        let mut parts = header.split(' ');
        if parts.next() != Some("CAPTEE-AUTOSAVE") || parts.next() != Some("1") {
            return Err(AtomicWriteError::MalformedAutosave);
        }
        let revision = parts
            .next()
            .and_then(|value| value.parse().ok())
            .ok_or(AtomicWriteError::MalformedAutosave)?;
        if parts.next().is_some() {
            return Err(AtomicWriteError::MalformedAutosave);
        }
        Ok(Some(AutosaveSnapshot { revision, contents: bytes[newline + 1..].to_vec() }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutosaveSnapshot {
    pub revision: u64,
    pub contents: Vec<u8>,
}

#[derive(Debug)]
pub enum AtomicWriteError {
    Io(io::Error),
    NoParent(PathBuf),
    MalformedAutosave,
}

impl fmt::Display for AtomicWriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "atomic write failed: {error}"),
            Self::NoParent(path) => write!(formatter, "path has no parent: {}", path.display()),
            Self::MalformedAutosave => formatter.write_str("malformed Captee autosave"),
        }
    }
}

impl std::error::Error for AtomicWriteError {}

fn temporary_path(path: &Path) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("file");
    path.with_file_name(format!(".{name}.captee-tmp-{}-{stamp}", std::process::id()))
}

fn sync_directory(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        File::open(path)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_root() -> PathBuf {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos();
        let root =
            std::env::temp_dir().join(format!("captee-atomic-{}-{suffix}", std::process::id()));
        fs::create_dir_all(&root).expect("temporary root");
        root
    }

    #[test]
    fn atomic_write_replaces_complete_destination() {
        let root = test_root();
        let destination = root.join("nested/note.typ");
        atomic_write(&destination, b"first").expect("first write");
        atomic_write(&destination, b"second").expect("second write");
        assert_eq!(fs::read(&destination).expect("read destination"), b"second");
        assert_eq!(
            fs::read_dir(destination.parent().expect("parent")).expect("read dir").count(),
            1
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn autosave_round_trips_revision_and_contents() {
        let root = test_root();
        let store = AutosaveStore::new(root.join("note.autosave"));
        store.write(7, b"draft").expect("autosave");
        assert_eq!(
            store.recover().expect("recover"),
            Some(AutosaveSnapshot { revision: 7, contents: b"draft".to_vec() })
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn atomic_create_preserves_existing_destination() {
        let root = test_root();
        let destination = root.join("img/capture.png");
        fs::create_dir(destination.parent().expect("parent")).expect("asset directory");
        atomic_create(&destination, b"first").expect("first create");
        let result = atomic_create(&destination, b"second");
        assert!(
            matches!(result, Err(AtomicWriteError::Io(error)) if error.kind() == io::ErrorKind::AlreadyExists)
        );
        assert_eq!(fs::read(&destination).expect("read destination"), b"first");
        assert_eq!(
            fs::read_dir(destination.parent().expect("parent")).expect("read dir").count(),
            1
        );
        fs::remove_dir_all(root).expect("cleanup");
    }
}
