//! Headless source-document state and revision-aware editing.

use std::ops::Range;

pub trait DocumentPersistence {
    type Error;

    fn save(&self, contents: &str) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDocument {
    text: String,
    saved_text: String,
    revision: u64,
    undo: Vec<String>,
    redo: Vec<String>,
}

impl SourceDocument {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            saved_text: text.clone(),
            text,
            revision: 0,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn is_dirty(&self) -> bool {
        self.text != self.saved_text
    }

    pub fn replace(&mut self, range: Range<usize>, replacement: &str) -> Result<(), EditError> {
        if range.start > range.end
            || range.end > self.text.len()
            || !self.text.is_char_boundary(range.start)
            || !self.text.is_char_boundary(range.end)
        {
            return Err(EditError::InvalidRange);
        }
        self.undo.push(self.text.clone());
        self.text.replace_range(range, replacement);
        self.redo.clear();
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        self.redo.push(self.text.clone());
        self.text = previous;
        self.revision = self.revision.saturating_add(1);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(self.text.clone());
        self.text = next;
        self.revision = self.revision.saturating_add(1);
        true
    }

    pub fn save<P: DocumentPersistence>(&mut self, persistence: &P) -> Result<(), P::Error> {
        persistence.save(&self.text)?;
        self.saved_text = self.text.clone();
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditError {
    InvalidRange,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct MemoryPersistence(RefCell<String>);

    impl DocumentPersistence for MemoryPersistence {
        type Error = ();

        fn save(&self, contents: &str) -> Result<(), Self::Error> {
            self.0.replace(contents.to_owned());
            Ok(())
        }
    }

    #[test]
    fn edit_tracks_dirty_state_and_revision() {
        let mut document = SourceDocument::new("hello");
        document.replace(5..5, " world").expect("valid edit");
        assert_eq!(document.text(), "hello world");
        assert!(document.is_dirty());
        assert_eq!(document.revision(), 1);
    }

    #[test]
    fn undo_and_redo_restore_snapshots() {
        let mut document = SourceDocument::new("a");
        document.replace(1..1, "b").expect("edit");
        assert!(document.undo());
        assert_eq!(document.text(), "a");
        assert!(document.redo());
        assert_eq!(document.text(), "ab");
    }

    #[test]
    fn save_clears_dirty_only_after_persistence_succeeds() {
        let mut document = SourceDocument::new("a");
        document.replace(1..1, "b").expect("edit");
        let persistence = MemoryPersistence(RefCell::new(String::new()));
        document.save(&persistence).expect("save");
        assert!(!document.is_dirty());
        assert_eq!(persistence.0.into_inner(), "ab");
    }
}

