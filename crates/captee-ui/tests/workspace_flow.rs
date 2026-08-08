use captee_core::ProjectConfig;
use captee_platform::{
    confirm_and_trash, create_project, create_project_item, list_project_tree, move_project_item,
    rename_project_item, TrashBackend, TrashError, TrashOutcome,
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

fn root(name: &str) -> PathBuf {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos();
    let path = std::env::temp_dir().join(format!("captee-ui-workspace-{name}-{stamp}"));
    std::fs::create_dir_all(&path).expect("root");
    path
}

struct TrashDouble(Arc<Mutex<Vec<PathBuf>>>);

impl TrashBackend for TrashDouble {
    fn move_to_trash(&self, path: &Path) -> Result<(), TrashError> {
        self.0.lock().expect("trash lock").push(path.to_owned());
        Ok(())
    }
}

#[test]
fn tree_navigation_and_context_mutations_refresh_expected_hierarchy() {
    let path = root("mutations");
    create_project(&path, ProjectConfig::new("Notes", "main.typ").expect("config"))
        .expect("project");
    create_project_item(&path, "", "notes", true).expect("folder");
    create_project_item(&path, "notes", "draft.typ", false).expect("file");
    move_project_item(&path, "notes/draft.typ", "").expect("move to root");
    rename_project_item(&path, "draft.typ", "final.typ").expect("rename");

    let entries = list_project_tree(&path).expect("tree");
    assert!(entries.iter().any(|entry| entry.relative_path == Path::new("final.typ")));
    assert!(!entries.iter().any(|entry| entry.relative_path == Path::new("notes/draft.typ")));
    std::fs::remove_dir_all(path).expect("cleanup");
}

#[test]
fn declined_delete_confirmation_does_not_mutate_project() {
    let path = root("cancel-delete");
    create_project(&path, ProjectConfig::new("Notes", "main.typ").expect("config"))
        .expect("project");
    let calls = Arc::new(Mutex::new(Vec::new()));
    let result = confirm_and_trash(&TrashDouble(calls.clone()), &path.join("main.typ"), false)
        .expect("cancel");
    assert_eq!(result, TrashOutcome::Cancelled);
    assert!(path.join("main.typ").is_file());
    assert!(calls.lock().expect("trash lock").is_empty());
    std::fs::remove_dir_all(path).expect("cleanup");
}
