use captee_core::{CancellationToken, OperationKind};
use std::cell::RefCell;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, SendError, Sender, TryRecvError};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectIdentity {
    generation: u64,
    root: PathBuf,
}

impl ProjectIdentity {
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceIdentity {
    project: ProjectIdentity,
    revision: u64,
}

impl SourceIdentity {
    pub fn project(&self) -> &ProjectIdentity {
        &self.project
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationId(u64);

impl OperationId {
    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationContext {
    id: OperationId,
    kind: OperationKind,
    source: SourceIdentity,
}

impl OperationContext {
    pub fn id(&self) -> OperationId {
        self.id
    }

    pub fn kind(&self) -> OperationKind {
        self.kind
    }

    pub fn source(&self) -> &SourceIdentity {
        &self.source
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationOutcome<T> {
    Completed(T),
    Cancelled,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationResult<T> {
    pub context: OperationContext,
    pub outcome: OperationOutcome<T>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultDisposition<T> {
    Current(OperationResult<T>),
    Stale(OperationResult<T>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordinatorError {
    ProjectRequired,
    Busy,
    NotCancellable,
    NoOperation,
    IdentifierExhausted,
}

impl fmt::Display for CoordinatorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProjectRequired => formatter.write_str("an active project is required"),
            Self::Busy => formatter.write_str("another operation is already running"),
            Self::NotCancellable => formatter.write_str("the active operation cannot be cancelled"),
            Self::NoOperation => formatter.write_str("no operation is running"),
            Self::IdentifierExhausted => {
                formatter.write_str("operation identity space is exhausted")
            }
        }
    }
}

impl std::error::Error for CoordinatorError {}

struct ActiveOperation {
    context: OperationContext,
    cancellable: bool,
    cancellation: CancellationToken,
}

/// A single-use worker-side handle. Dropping it always reports a terminal result.
pub struct OperationTask<T> {
    context: OperationContext,
    cancellation: CancellationToken,
    sender: Option<Sender<OperationResult<T>>>,
}

impl<T> OperationTask<T> {
    pub fn context(&self) -> &OperationContext {
        &self.context
    }

    pub fn cancellation(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    pub fn finish(
        mut self,
        outcome: OperationOutcome<T>,
    ) -> Result<(), SendError<OperationResult<T>>> {
        let sender = self.sender.take().expect("operation task has one result sender");
        sender.send(OperationResult { context: self.context.clone(), outcome })
    }
}

impl<T> Drop for OperationTask<T> {
    fn drop(&mut self) {
        let Some(sender) = self.sender.take() else {
            return;
        };
        let outcome = if self.cancellation.is_cancelled() {
            OperationOutcome::Cancelled
        } else {
            OperationOutcome::Failed("operation ended without reporting a result".to_owned())
        };
        let _ = sender.send(OperationResult { context: self.context.clone(), outcome });
    }
}

/// UI-owned operation identity, cancellation, and result-routing boundary.
///
/// The coordinator never performs platform work. Callers move `OperationTask`
/// into a worker and poll `try_next_result` from the GTK main context.
pub struct OperationCoordinator<T> {
    active_project: Option<ProjectIdentity>,
    source_revision: u64,
    next_project_generation: u64,
    next_operation_id: u64,
    active_operation: Option<ActiveOperation>,
    result_sender: Sender<OperationResult<T>>,
    result_receiver: Receiver<OperationResult<T>>,
}

impl<T> Default for OperationCoordinator<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> OperationCoordinator<T> {
    pub fn new() -> Self {
        let (result_sender, result_receiver) = mpsc::channel();
        Self {
            active_project: None,
            source_revision: 0,
            next_project_generation: 0,
            next_operation_id: 0,
            active_operation: None,
            result_sender,
            result_receiver,
        }
    }

    pub fn activate_project(
        &mut self,
        root: impl Into<PathBuf>,
    ) -> Result<ProjectIdentity, CoordinatorError> {
        self.cancel_for_lifetime_change();
        self.next_project_generation = self
            .next_project_generation
            .checked_add(1)
            .ok_or(CoordinatorError::IdentifierExhausted)?;
        let identity =
            ProjectIdentity { generation: self.next_project_generation, root: root.into() };
        self.active_project = Some(identity.clone());
        self.source_revision = 0;
        Ok(identity)
    }

    pub fn deactivate_project(&mut self) {
        self.cancel_for_lifetime_change();
        self.active_project = None;
        self.source_revision = 0;
    }

    pub fn active_source(&self) -> Option<SourceIdentity> {
        self.active_project
            .clone()
            .map(|project| SourceIdentity { project, revision: self.source_revision })
    }

    pub fn set_source_revision(&mut self, revision: u64) -> Result<(), CoordinatorError> {
        if self.active_project.is_none() {
            return Err(CoordinatorError::ProjectRequired);
        }
        if revision != self.source_revision {
            self.source_revision = revision;
            if let Some(active) = &self.active_operation {
                if active.cancellable {
                    active.cancellation.cancel();
                }
            }
        }
        Ok(())
    }

    pub fn begin(
        &mut self,
        kind: OperationKind,
        cancellable: bool,
    ) -> Result<OperationTask<T>, CoordinatorError> {
        if self.active_operation.is_some() {
            return Err(CoordinatorError::Busy);
        }
        let source = self.active_source().ok_or(CoordinatorError::ProjectRequired)?;
        self.next_operation_id =
            self.next_operation_id.checked_add(1).ok_or(CoordinatorError::IdentifierExhausted)?;
        let context = OperationContext { id: OperationId(self.next_operation_id), kind, source };
        let cancellation = CancellationToken::default();
        self.active_operation = Some(ActiveOperation {
            context: context.clone(),
            cancellable,
            cancellation: cancellation.clone(),
        });
        Ok(OperationTask { context, cancellation, sender: Some(self.result_sender.clone()) })
    }

    pub fn active_context(&self) -> Option<&OperationContext> {
        self.active_operation.as_ref().map(|active| &active.context)
    }

    pub fn cancel_active(&mut self) -> Result<OperationContext, CoordinatorError> {
        let active = self.active_operation.as_ref().ok_or(CoordinatorError::NoOperation)?;
        if !active.cancellable {
            return Err(CoordinatorError::NotCancellable);
        }
        active.cancellation.cancel();
        let active = self.active_operation.take().expect("active operation exists");
        Ok(active.context)
    }

    pub fn try_next_result(&mut self) -> Option<ResultDisposition<T>> {
        let result = match self.result_receiver.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => return None,
        };
        let is_current = self.active_operation.as_ref().is_some_and(|active| {
            active.context == result.context
                && !active.cancellation.is_cancelled()
                && self.active_source().as_ref() == Some(result.context.source())
        });
        if self
            .active_operation
            .as_ref()
            .is_some_and(|active| active.context.id == result.context.id)
        {
            self.active_operation = None;
        }
        Some(if is_current {
            ResultDisposition::Current(result)
        } else {
            ResultDisposition::Stale(result)
        })
    }

    fn cancel_for_lifetime_change(&mut self) {
        if let Some(active) = self.active_operation.take() {
            active.cancellation.cancel();
        }
    }
}

/// Drains ready results without holding the coordinator borrow while applying
/// them. Result handlers may synchronously update project or source identity.
pub fn drain_ready_results<T>(
    coordinator: &RefCell<OperationCoordinator<T>>,
    mut apply: impl FnMut(ResultDisposition<T>),
) {
    loop {
        let result = { coordinator.borrow_mut().try_next_result() };
        let Some(result) = result else {
            break;
        };
        apply(result);
    }
}

impl<T> Drop for OperationCoordinator<T> {
    fn drop(&mut self) {
        self.cancel_for_lifetime_change();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reopening_the_same_root_gets_a_new_project_identity() {
        let mut coordinator = OperationCoordinator::<()>::new();
        let first = coordinator.activate_project("/tmp/notes").expect("first project");
        coordinator.deactivate_project();
        let second = coordinator.activate_project("/tmp/notes").expect("second project");

        assert_eq!(first.root(), second.root());
        assert_ne!(first.generation(), second.generation());
    }

    #[test]
    fn revision_changes_cancel_and_reject_old_work() {
        let mut coordinator = OperationCoordinator::new();
        coordinator.activate_project("/tmp/notes").expect("project");
        let task = coordinator.begin(OperationKind::Preview, true).expect("preview task");
        let cancellation = task.cancellation();

        coordinator.set_source_revision(1).expect("new revision");
        assert!(cancellation.is_cancelled());
        task.finish(OperationOutcome::Completed(())).expect("result sent");
        assert!(matches!(coordinator.try_next_result(), Some(ResultDisposition::Stale(_))));
    }

    #[test]
    fn dropping_the_coordinator_cancels_outstanding_work() {
        let task = {
            let mut coordinator = OperationCoordinator::<()>::new();
            coordinator.activate_project("/tmp/notes").expect("project");
            coordinator.begin(OperationKind::Capture, true).expect("capture task")
        };

        assert!(task.cancellation().is_cancelled());
    }

    #[test]
    fn result_handlers_can_reborrow_the_coordinator() {
        let coordinator = RefCell::new(OperationCoordinator::new());
        coordinator.borrow_mut().activate_project("/tmp/notes").expect("project");
        let task = coordinator.borrow_mut().begin(OperationKind::Save, false).expect("save task");
        task.finish(OperationOutcome::Completed(())).expect("result");

        drain_ready_results(&coordinator, |_| {
            coordinator.borrow_mut().set_source_revision(1).expect("handler reborrow");
        });

        assert_eq!(coordinator.borrow().active_source().expect("source").revision(), 1);
    }
}
