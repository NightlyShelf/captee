//! Debounced revision scheduling and stale-result rejection.

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkKind {
    Compile,
    Format,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingWork {
    pub revision: u64,
    pub kind: WorkKind,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct DebouncedScheduler {
    debounce: Duration,
    current_revision: u64,
    pending: Option<(Instant, PendingWork)>,
}

impl DebouncedScheduler {
    pub fn new(debounce: Duration) -> Self {
        Self { debounce, current_revision: 0, pending: None }
    }

    pub fn current_revision(&self) -> u64 {
        self.current_revision
    }

    /// Queues only the newest source snapshot and returns its revision ID.
    pub fn submit(&mut self, kind: WorkKind, source: impl Into<String>, now: Instant) -> u64 {
        self.current_revision = self.current_revision.saturating_add(1);
        self.pending = Some((
            now + self.debounce,
            PendingWork { revision: self.current_revision, kind, source: source.into() },
        ));
        self.current_revision
    }

    /// Returns the newest work once the debounce interval has elapsed.
    pub fn take_ready(&mut self, now: Instant) -> Option<PendingWork> {
        let (due, _) = self.pending.as_ref()?;
        if now < *due {
            return None;
        }
        self.pending.take().map(|(_, work)| work)
    }

    /// Accepts a worker result only when it belongs to the current revision.
    pub fn accepts_result(&self, revision: u64) -> bool {
        revision == self.current_revision
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debounce_delays_and_coalesces_work() {
        let start = Instant::now();
        let mut scheduler = DebouncedScheduler::new(Duration::from_millis(50));
        scheduler.submit(WorkKind::Compile, "old", start);
        scheduler.submit(WorkKind::Compile, "new", start + Duration::from_millis(10));
        assert!(scheduler.take_ready(start + Duration::from_millis(59)).is_none());
        assert_eq!(
            scheduler.take_ready(start + Duration::from_millis(60)).expect("ready").source,
            "new"
        );
    }

    #[test]
    fn stale_results_are_rejected() {
        let start = Instant::now();
        let mut scheduler = DebouncedScheduler::new(Duration::ZERO);
        let first = scheduler.submit(WorkKind::Compile, "first", start);
        scheduler.submit(WorkKind::Compile, "second", start);
        assert!(!scheduler.accepts_result(first));
        assert!(scheduler.accepts_result(scheduler.current_revision()));
    }
}
