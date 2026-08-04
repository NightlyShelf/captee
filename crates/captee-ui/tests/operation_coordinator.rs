use captee_core::OperationKind;
use captee_ui::operation::{
    OperationCoordinator, OperationOutcome, OperationTask, ResultDisposition,
};

struct WorkerDouble<T> {
    outcome: OperationOutcome<T>,
}

impl<T> WorkerDouble<T> {
    fn returning(outcome: OperationOutcome<T>) -> Self {
        Self { outcome }
    }

    fn run(self, task: OperationTask<T>) {
        task.finish(self.outcome).expect("coordinator accepts worker result");
    }
}

fn coordinator() -> OperationCoordinator<&'static str> {
    let mut coordinator = OperationCoordinator::new();
    coordinator.activate_project("/tmp/notes").expect("project activates");
    coordinator
}

#[test]
fn successful_worker_result_is_current_and_releases_the_operation() {
    let mut coordinator = coordinator();
    let task = coordinator.begin(OperationKind::Save, false).expect("save starts");

    WorkerDouble::returning(OperationOutcome::Completed("saved")).run(task);

    let Some(ResultDisposition::Current(result)) = coordinator.try_next_result() else {
        panic!("successful result should be current");
    };
    assert_eq!(result.outcome, OperationOutcome::Completed("saved"));
    assert!(coordinator.active_context().is_none());
}

#[test]
fn worker_cancellation_is_current_and_releases_the_operation() {
    let mut coordinator = coordinator();
    let task = coordinator.begin(OperationKind::Capture, true).expect("capture starts");

    WorkerDouble::returning(OperationOutcome::Cancelled).run(task);

    let Some(ResultDisposition::Current(result)) = coordinator.try_next_result() else {
        panic!("worker cancellation should be current");
    };
    assert_eq!(result.outcome, OperationOutcome::Cancelled);
    assert!(coordinator.active_context().is_none());
}

#[test]
fn worker_failure_is_current_and_preserves_the_active_source() {
    let mut coordinator = coordinator();
    let source = coordinator.active_source().expect("active source");
    let task = coordinator.begin(OperationKind::Preview, true).expect("preview starts");

    WorkerDouble::returning(OperationOutcome::Failed("compiler failed".to_owned())).run(task);

    let Some(ResultDisposition::Current(result)) = coordinator.try_next_result() else {
        panic!("failure should be current");
    };
    assert_eq!(result.outcome, OperationOutcome::Failed("compiler failed".to_owned()));
    assert_eq!(coordinator.active_source(), Some(source));
}

#[test]
fn explicitly_cancelled_worker_cannot_apply_a_late_success() {
    let mut coordinator = coordinator();
    let task = coordinator.begin(OperationKind::Capture, true).expect("capture starts");
    let cancellation = task.cancellation();

    coordinator.cancel_active().expect("capture cancels");
    assert!(cancellation.is_cancelled());
    WorkerDouble::returning(OperationOutcome::Completed("captured")).run(task);

    assert!(matches!(coordinator.try_next_result(), Some(ResultDisposition::Stale(_))));
    assert!(coordinator.active_context().is_none());
}

#[test]
fn project_replacement_rejects_results_from_the_previous_project_generation() {
    let mut coordinator = coordinator();
    let task = coordinator.begin(OperationKind::Preview, true).expect("preview starts");

    coordinator.activate_project("/tmp/other").expect("replacement project activates");
    WorkerDouble::returning(OperationOutcome::Completed("old preview")).run(task);

    assert!(matches!(coordinator.try_next_result(), Some(ResultDisposition::Stale(_))));
    assert_eq!(coordinator.active_source().expect("new source").project().root(), "/tmp/other");
}

#[test]
fn source_revision_change_rejects_results_from_the_previous_revision() {
    let mut coordinator = coordinator();
    let task = coordinator.begin(OperationKind::Format, true).expect("format starts");

    coordinator.set_source_revision(3).expect("source changes");
    WorkerDouble::returning(OperationOutcome::Completed("old source")).run(task);

    assert!(matches!(coordinator.try_next_result(), Some(ResultDisposition::Stale(_))));
    assert_eq!(coordinator.active_source().expect("active source").revision(), 3);
}
