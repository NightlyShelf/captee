//! Safe project-root and project-relative path handling.

use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ProjectPaths {
    root: PathBuf,
    canonical_root: PathBuf,
}

impl ProjectPaths {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, PathError> {
        let root = root.as_ref();
        let metadata = fs::symlink_metadata(root).map_err(PathError::Io)?;
        if !metadata.is_dir() {
            return Err(PathError::RootNotDirectory(root.to_path_buf()));
        }
        let canonical_root = fs::canonicalize(root).map_err(PathError::Io)?;
        Ok(Self { root: root.to_path_buf(), canonical_root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn resolve(&self, relative: impl AsRef<Path>) -> Result<PathBuf, PathError> {
        let relative = relative.as_ref();
        if relative.is_absolute()
            || relative.components().any(|component| matches!(component, Component::ParentDir))
        {
            return Err(PathError::UnsafeRelativePath(relative.to_path_buf()));
        }

        let candidate = self.root.join(relative);
        let checked = if candidate.exists() {
            fs::canonicalize(&candidate).map_err(PathError::Io)?
        } else {
            let parent = candidate
                .parent()
                .ok_or_else(|| PathError::UnsafeRelativePath(relative.to_path_buf()))?;
            let canonical_parent = fs::canonicalize(parent).map_err(PathError::Io)?;
            canonical_parent.join(
                candidate
                    .file_name()
                    .ok_or_else(|| PathError::UnsafeRelativePath(relative.to_path_buf()))?,
            )
        };

        if !checked.starts_with(&self.canonical_root) {
            return Err(PathError::OutsideRoot(relative.to_path_buf()));
        }
        Ok(checked)
    }

    pub fn require_file(&self, relative: impl AsRef<Path>) -> Result<PathBuf, PathError> {
        let path = self.resolve(relative)?;
        if path.exists() && !fs::metadata(&path).map_err(PathError::Io)?.is_file() {
            return Err(PathError::ExpectedFile(path));
        }
        Ok(path)
    }

    pub fn require_directory(&self, relative: impl AsRef<Path>) -> Result<PathBuf, PathError> {
        let path = self.resolve(relative)?;
        if path.exists() && !fs::metadata(&path).map_err(PathError::Io)?.is_dir() {
            return Err(PathError::ExpectedDirectory(path));
        }
        Ok(path)
    }
}

#[derive(Debug)]
pub enum PathError {
    Io(std::io::Error),
    RootNotDirectory(PathBuf),
    UnsafeRelativePath(PathBuf),
    OutsideRoot(PathBuf),
    ExpectedFile(PathBuf),
    ExpectedDirectory(PathBuf),
}

impl fmt::Display for PathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "path operation failed: {error}"),
            Self::RootNotDirectory(path) => {
                write!(formatter, "project root is not a directory: {}", path.display())
            }
            Self::UnsafeRelativePath(path) => {
                write!(formatter, "unsafe project-relative path: {}", path.display())
            }
            Self::OutsideRoot(path) => {
                write!(formatter, "path escapes project root: {}", path.display())
            }
            Self::ExpectedFile(path) => write!(formatter, "expected a file: {}", path.display()),
            Self::ExpectedDirectory(path) => {
                write!(formatter, "expected a directory: {}", path.display())
            }
        }
    }
}

impl std::error::Error for PathError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_root() -> PathBuf {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos();
        let root = std::env::temp_dir().join(format!("captee-paths-{suffix}"));
        fs::create_dir_all(&root).expect("temporary root");
        root
    }

    #[test]
    fn rejects_traversal_and_absolute_paths() {
        let root = test_root();
        let paths = ProjectPaths::open(&root).expect("root");
        assert!(matches!(paths.resolve("../outside"), Err(PathError::UnsafeRelativePath(_))));
        assert!(matches!(
            paths.resolve(PathBuf::from("/tmp/outside")),
            Err(PathError::UnsafeRelativePath(_))
        ));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn checks_expected_file_and_directory_types() {
        let root = test_root();
        fs::write(root.join("note.typ"), "#let x = 1").expect("file");
        fs::create_dir(root.join("img")).expect("directory");
        let paths = ProjectPaths::open(&root).expect("root");
        assert!(paths.require_file("note.typ").is_ok());
        assert!(paths.require_directory("img").is_ok());
        assert!(matches!(paths.require_file("img"), Err(PathError::ExpectedFile(_))));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_escape() {
        let root = test_root();
        let outside = test_root();
        fs::write(outside.join("secret.typ"), "secret").expect("outside file");
        std::os::unix::fs::symlink(&outside, root.join("link")).expect("symlink");
        let paths = ProjectPaths::open(&root).expect("root");
        assert!(matches!(paths.resolve("link/secret.typ"), Err(PathError::OutsideRoot(_))));
        fs::remove_dir_all(root).expect("cleanup root");
        fs::remove_dir_all(outside).expect("cleanup outside");
    }
}
