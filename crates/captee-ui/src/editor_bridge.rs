use captee_core::{EditError, SourceDocument};
use std::ops::Range;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorState {
    pub text: String,
    pub revision: u64,
    pub dirty: bool,
}

/// Testable boundary between a text widget buffer and the active core document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorBridge {
    entry_document: PathBuf,
    document: SourceDocument,
}

impl EditorBridge {
    pub fn new(entry_document: impl Into<PathBuf>, source: impl Into<String>) -> Self {
        Self { entry_document: entry_document.into(), document: SourceDocument::new(source) }
    }

    pub fn entry_document(&self) -> &Path {
        &self.entry_document
    }

    pub fn state(&self) -> EditorState {
        EditorState {
            text: self.document.text().to_owned(),
            revision: self.document.revision(),
            dirty: self.document.is_dirty(),
        }
    }

    pub fn document_snapshot(&self) -> SourceDocument {
        self.document.clone()
    }

    pub fn apply_saved_document(&mut self, document: SourceDocument) -> Option<EditorState> {
        if document.revision() != self.document.revision()
            || document.text() != self.document.text()
            || document.is_dirty()
        {
            return None;
        }
        self.document = document;
        Some(self.state())
    }

    pub fn update_from_buffer(&mut self, text: &str) -> Result<Option<EditorState>, EditError> {
        if text == self.document.text() {
            return Ok(None);
        }
        let previous_len = self.document.text().len();
        self.document.replace(0..previous_len, text)?;
        Ok(Some(self.state()))
    }

    pub fn replace_range(
        &mut self,
        range: Range<usize>,
        replacement: &str,
    ) -> Result<EditorState, EditError> {
        self.document.replace(range, replacement)?;
        Ok(self.state())
    }

    pub fn undo(&mut self) -> Option<EditorState> {
        self.document.undo().then(|| self.state())
    }

    pub fn redo(&mut self) -> Option<EditorState> {
        self.document.redo().then(|| self.state())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_updates_track_revision_dirty_state_and_entry_document() {
        let mut bridge = EditorBridge::new("notes/main.typ", "hello");

        let state = bridge.update_from_buffer("hello world").expect("valid text").expect("change");

        assert_eq!(bridge.entry_document(), Path::new("notes/main.typ"));
        assert_eq!(state.text, "hello world");
        assert_eq!(state.revision, 1);
        assert!(state.dirty);
        assert!(bridge.update_from_buffer("hello world").expect("same text").is_none());
    }

    #[test]
    fn undo_and_redo_return_widget_ready_snapshots() {
        let mut bridge = EditorBridge::new("main.typ", "a");
        bridge.update_from_buffer("ab").expect("valid text");

        let undone = bridge.undo().expect("undo");
        assert_eq!(undone.text, "a");
        assert!(!undone.dirty);
        let redone = bridge.redo().expect("redo");
        assert_eq!(redone.text, "ab");
        assert!(redone.dirty);
    }

    #[test]
    fn only_the_current_successfully_saved_snapshot_clears_dirty_state() {
        struct MemoryPersistence;

        impl captee_core::DocumentPersistence for MemoryPersistence {
            type Error = ();

            fn save(&self, _contents: &str) -> Result<(), Self::Error> {
                Ok(())
            }
        }

        let mut bridge = EditorBridge::new("main.typ", "a");
        bridge.update_from_buffer("ab").expect("edit");
        let mut saved = bridge.document_snapshot();
        saved.save(&MemoryPersistence).expect("save");

        assert!(!bridge.apply_saved_document(saved).expect("current save").dirty);

        bridge.update_from_buffer("abc").expect("new edit");
        let mut stale = bridge.document_snapshot();
        bridge.update_from_buffer("abcd").expect("newer edit");
        stale.save(&MemoryPersistence).expect("stale save");
        assert!(bridge.apply_saved_document(stale).is_none());
        assert!(bridge.state().dirty);
    }

    #[test]
    fn range_replacement_is_recorded_as_one_undoable_edit() {
        let mut bridge = EditorBridge::new("main.typ", "hello");
        let state = bridge.replace_range(0..5, "goodbye").expect("replace");
        assert_eq!(state.text, "goodbye");
        assert_eq!(bridge.undo().expect("undo").text, "hello");
    }
}
