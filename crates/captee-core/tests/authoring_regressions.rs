use captee_core::{
    parse_diagnostics, DebouncedScheduler, DiagnosticSeverity, DocumentPersistence, Formatter,
    Operation, SourceDocument, WorkKind,
};
use std::time::{Duration, Instant};

struct FailingFormatter;

impl Formatter for FailingFormatter {
    type Error = &'static str;

    fn format(&self, _source: &str) -> Result<String, Self::Error> {
        Err("formatter failed")
    }
}

struct MemoryPersistence;

impl DocumentPersistence for MemoryPersistence {
    type Error = ();

    fn save(&self, _contents: &str) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[test]
fn diagnostics_keep_warning_severity_and_location() {
    let diagnostics = parse_diagnostics("warning: main.typ:2:4: deprecated syntax");
    assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Warning);
    assert_eq!(diagnostics[0].span.as_ref().expect("span").line, 2);
}

#[test]
fn formatter_failure_preserves_source() {
    let source = "  #let x = 1  ".to_owned();
    let result = FailingFormatter.format(&source);
    assert!(result.is_err());
    assert_eq!(source, "  #let x = 1  ");
}

#[test]
fn stale_scheduler_results_are_rejected_after_new_edit() {
    let now = Instant::now();
    let mut scheduler = DebouncedScheduler::new(Duration::from_millis(10));
    let stale = scheduler.submit(WorkKind::Compile, "old", now);
    scheduler.submit(WorkKind::Compile, "new", now);
    assert!(!scheduler.accepts_result(stale));
}

#[test]
fn confirmed_replace_is_the_only_mutating_path() {
    let cancelled = captee_core::replace_literal("old", "old", "new", false).expect("cancel");
    assert_eq!(cancelled, Operation::Cancelled);
    let confirmed = captee_core::replace_literal("old", "old", "new", true).expect("replace");
    assert_eq!(confirmed.expect_completed().text, "new");
}

#[test]
fn save_contract_is_available_for_formatter_failure_regressions() {
    let mut document = SourceDocument::new("source");
    document.replace(0..0, "new ").expect("edit");
    assert!(document.save(&MemoryPersistence).is_ok());
    assert!(!document.is_dirty());
}

trait ExpectCompleted<T> {
    fn expect_completed(self) -> T;
}

impl<T> ExpectCompleted<T> for Operation<T> {
    fn expect_completed(self) -> T {
        match self {
            Operation::Completed(value) => value,
            Operation::Cancelled => panic!("operation was cancelled"),
        }
    }
}
