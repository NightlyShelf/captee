//! Minimal stdio LSP client for the bundled Tinymist language server.

use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use url::Url;

const INITIALIZE_REQUEST_ID: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TinymistCompletion {
    pub label: String,
    pub insert_text: String,
    pub range: Option<LspRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TinymistDiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TinymistDiagnostic {
    pub range: LspRange,
    pub severity: TinymistDiagnosticSeverity,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TinymistEvent {
    Completion { uri: String, version: i32, request_id: u64, items: Vec<TinymistCompletion> },
    Diagnostics { uri: String, version: Option<i32>, items: Vec<TinymistDiagnostic> },
    Failed(String),
}

#[derive(Debug)]
pub enum TinymistError {
    Io(io::Error),
    Protocol(String),
    InvalidProjectRoot(PathBuf),
}

impl fmt::Display for TinymistError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => error.fmt(formatter),
            Self::Protocol(message) => formatter.write_str(message),
            Self::InvalidProjectRoot(path) => {
                write!(
                    formatter,
                    "project root cannot be represented as a file URI: {}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for TinymistError {}

impl From<io::Error> for TinymistError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug, Clone)]
pub struct TinymistRunner {
    executable: PathBuf,
}

impl TinymistRunner {
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self { executable: executable.into() }
    }

    pub fn discover() -> Self {
        if let Some(executable) = std::env::var_os("CAPTEE_TINYMIST_BINARY") {
            return Self::new(executable);
        }
        if let Ok(current_executable) = std::env::current_exe() {
            if let Some(directory) = current_executable.parent() {
                for candidate in [
                    directory.join("tinymist"),
                    directory.join("../share/captee/tinymist/tinymist"),
                    directory.join("../lib/captee/tinymist"),
                ] {
                    if candidate.is_file() {
                        return Self::new(candidate);
                    }
                }
            }
        }
        let development =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../dist/tinymist/tinymist");
        if development.is_file() {
            return Self::new(development);
        }
        Self::new("tinymist")
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.executable);
        command.arg("lsp");
        command
    }
}

#[derive(Debug, Clone)]
struct PendingCompletion {
    uri: String,
    version: i32,
}

pub struct TinymistSession {
    writer: Arc<Mutex<ChildStdin>>,
    events: mpsc::Receiver<TinymistEvent>,
    pending: Arc<Mutex<BTreeMap<u64, PendingCompletion>>>,
    next_request_id: AtomicU64,
    child: Option<Child>,
}

impl fmt::Debug for TinymistSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("TinymistSession").finish_non_exhaustive()
    }
}

impl TinymistSession {
    pub fn start(project_root: &Path) -> Result<Self, TinymistError> {
        Self::start_with_runner(TinymistRunner::discover(), project_root)
    }

    pub fn start_with_runner(
        runner: TinymistRunner,
        project_root: &Path,
    ) -> Result<Self, TinymistError> {
        let root_uri = document_uri(project_root)
            .ok_or_else(|| TinymistError::InvalidProjectRoot(project_root.to_path_buf()))?;
        let mut child = runner
            .command()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| {
                if error.kind() == io::ErrorKind::NotFound {
                    TinymistError::Protocol(
                        "Tinymist not found; run tools/fetch-tinymist.sh or set CAPTEE_TINYMIST_BINARY"
                            .to_owned(),
                    )
                } else {
                    TinymistError::Io(error)
                }
            })?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| TinymistError::Protocol("Tinymist stdin unavailable".to_owned()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| TinymistError::Protocol("Tinymist stdout unavailable".to_owned()))?;
        let mut reader = BufReader::new(stdout);

        write_message(&mut stdin, &initialize_request(&root_uri))?;
        wait_for_initialize(&mut reader)?;
        write_message(&mut stdin, &json!({"jsonrpc":"2.0","method":"initialized","params":{}}))?;

        let writer = Arc::new(Mutex::new(stdin));
        let pending = Arc::new(Mutex::new(BTreeMap::new()));
        let (event_sender, events) = mpsc::channel();
        let reader_pending = Arc::clone(&pending);
        thread::Builder::new()
            .name("captee-tinymist-reader".to_owned())
            .spawn(move || read_events(reader, reader_pending, event_sender))?;

        Ok(Self {
            writer,
            events,
            pending,
            next_request_id: AtomicU64::new(INITIALIZE_REQUEST_ID + 1),
            child: Some(child),
        })
    }

    pub fn open_document(&self, uri: &str, version: i32, text: &str) -> Result<(), TinymistError> {
        self.notify("textDocument/didOpen", open_document_params(uri, version, text))
    }

    pub fn change_document(
        &self,
        uri: &str,
        version: i32,
        text: &str,
    ) -> Result<(), TinymistError> {
        self.notify("textDocument/didChange", change_document_params(uri, version, text))
    }

    pub fn close_document(&self, uri: &str) -> Result<(), TinymistError> {
        self.notify("textDocument/didClose", json!({"textDocument":{"uri":uri}}))
    }

    pub fn request_completion(
        &self,
        uri: &str,
        version: i32,
        position: LspPosition,
    ) -> Result<u64, TinymistError> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        self.pending
            .lock()
            .map_err(|_| TinymistError::Protocol("Tinymist request state unavailable".to_owned()))?
            .insert(request_id, PendingCompletion { uri: uri.to_owned(), version });
        let message = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": position.line, "character": position.character },
                "context": { "triggerKind": 1 }
            }
        });
        if let Err(error) = self.send(&message) {
            if let Ok(mut pending) = self.pending.lock() {
                pending.remove(&request_id);
            }
            return Err(error);
        }
        Ok(request_id)
    }

    pub fn try_recv(&self) -> Result<TinymistEvent, mpsc::TryRecvError> {
        self.events.try_recv()
    }

    fn notify(&self, method: &str, params: Value) -> Result<(), TinymistError> {
        self.send(&json!({"jsonrpc":"2.0","method":method,"params":params}))
    }

    fn send(&self, message: &Value) -> Result<(), TinymistError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| TinymistError::Protocol("Tinymist writer unavailable".to_owned()))?;
        write_message(&mut *writer, message).map_err(TinymistError::Io)
    }

    pub fn shutdown(&mut self) {
        if self.child.is_none() {
            return;
        }
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let _ = self.send(&json!({
            "jsonrpc":"2.0",
            "id":request_id,
            "method":"shutdown",
            "params":null
        }));
        let _ = self.notify("exit", json!({}));
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for TinymistSession {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn document_uri(path: &Path) -> Option<String> {
    Url::from_file_path(path).ok().map(Into::into)
}

pub fn capture_review_uri(project_root: &Path) -> Option<String> {
    document_uri(&project_root.join(".captee-capture-review.typ"))
}

fn initialize_request(root_uri: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": INITIALIZE_REQUEST_ID,
        "method": "initialize",
        "params": {
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {
                "textDocument": {
                    "completion": {
                        "completionItem": { "snippetSupport": false }
                    },
                    "publishDiagnostics": { "versionSupport": true }
                }
            }
        }
    })
}

fn open_document_params(uri: &str, version: i32, text: &str) -> Value {
    json!({
        "textDocument": {
            "uri": uri,
            "languageId": "typst",
            "version": version,
            "text": text
        }
    })
}

fn change_document_params(uri: &str, version: i32, text: &str) -> Value {
    json!({
        "textDocument": { "uri": uri, "version": version },
        "contentChanges": [{ "text": text }]
    })
}

fn wait_for_initialize<R: BufRead>(reader: &mut R) -> Result<(), TinymistError> {
    loop {
        let message = read_message(reader)?.ok_or_else(|| {
            TinymistError::Protocol("Tinymist exited during initialization".to_owned())
        })?;
        if message.get("id").and_then(Value::as_u64) != Some(INITIALIZE_REQUEST_ID) {
            continue;
        }
        if let Some(error) = message.get("error") {
            return Err(TinymistError::Protocol(format!(
                "Tinymist initialization failed: {error}"
            )));
        }
        return Ok(());
    }
}

fn read_events<R: BufRead>(
    mut reader: R,
    pending: Arc<Mutex<BTreeMap<u64, PendingCompletion>>>,
    sender: mpsc::Sender<TinymistEvent>,
) {
    loop {
        match read_message(&mut reader) {
            Ok(Some(message)) => {
                if let Some(event) = parse_event(&message, &pending) {
                    if sender.send(event).is_err() {
                        return;
                    }
                }
            }
            Ok(None) => {
                let _ = sender.send(TinymistEvent::Failed(
                    "Tinymist exited; editor assistance is unavailable".to_owned(),
                ));
                return;
            }
            Err(error) => {
                let _ = sender
                    .send(TinymistEvent::Failed(format!("Tinymist protocol failed: {error}")));
                return;
            }
        }
    }
}

fn parse_event(
    message: &Value,
    pending: &Arc<Mutex<BTreeMap<u64, PendingCompletion>>>,
) -> Option<TinymistEvent> {
    if let Some(request_id) = message.get("id").and_then(Value::as_u64) {
        let request = pending.lock().ok()?.remove(&request_id)?;
        let items = if message.get("error").is_some() {
            Vec::new()
        } else {
            parse_completion_items(message.get("result").unwrap_or(&Value::Null))
        };
        return Some(TinymistEvent::Completion {
            uri: request.uri,
            version: request.version,
            request_id,
            items,
        });
    }
    if message.get("method").and_then(Value::as_str) == Some("textDocument/publishDiagnostics") {
        let params = message.get("params")?;
        return Some(TinymistEvent::Diagnostics {
            uri: params.get("uri")?.as_str()?.to_owned(),
            version: params
                .get("version")
                .and_then(Value::as_i64)
                .and_then(|version| i32::try_from(version).ok()),
            items: params
                .get("diagnostics")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(parse_diagnostic)
                .collect(),
        });
    }
    None
}

fn parse_completion_items(result: &Value) -> Vec<TinymistCompletion> {
    let items = result.as_array().or_else(|| result.get("items").and_then(Value::as_array));
    items
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let label = item.get("label")?.as_str()?.to_owned();
            let text_edit = item.get("textEdit");
            let insert_text = text_edit
                .and_then(|edit| edit.get("newText"))
                .and_then(Value::as_str)
                .or_else(|| item.get("insertText").and_then(Value::as_str))
                .unwrap_or(&label)
                .to_owned();
            let range = text_edit.and_then(|edit| {
                edit.get("range").or_else(|| edit.get("insert")).and_then(parse_range)
            });
            Some(TinymistCompletion { label, insert_text, range })
        })
        .collect()
}

fn parse_diagnostic(value: &Value) -> Option<TinymistDiagnostic> {
    let severity = match value.get("severity").and_then(Value::as_u64) {
        Some(1) => TinymistDiagnosticSeverity::Error,
        Some(2) => TinymistDiagnosticSeverity::Warning,
        _ => return None,
    };
    Some(TinymistDiagnostic {
        range: parse_range(value.get("range")?)?,
        severity,
        message: value.get("message")?.as_str()?.to_owned(),
    })
}

fn parse_range(value: &Value) -> Option<LspRange> {
    Some(LspRange {
        start: parse_position(value.get("start")?)?,
        end: parse_position(value.get("end")?)?,
    })
}

fn parse_position(value: &Value) -> Option<LspPosition> {
    Some(LspPosition {
        line: u32::try_from(value.get("line")?.as_u64()?).ok()?,
        character: u32::try_from(value.get("character")?.as_u64()?).ok()?,
    })
}

fn write_message<W: Write>(writer: &mut W, message: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(message)?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(&body)?;
    writer.flush()
}

fn read_message<R: BufRead>(reader: &mut R) -> io::Result<Option<Value>> {
    let mut content_length = None;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 {
            return Ok(None);
        }
        if header == "\r\n" || header == "\n" {
            break;
        }
        if let Some(value) = header.trim_end().strip_prefix("Content-Length:").map(str::trim) {
            content_length = value.parse::<usize>().ok();
        }
    }
    let length = content_length
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length"))?;
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    serde_json::from_slice(&body).map(Some).map_err(io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::sync::mpsc::TryRecvError;

    #[test]
    fn lsp_frames_round_trip() {
        let message = json!({"jsonrpc":"2.0","id":7,"result":[]});
        let mut bytes = Vec::new();
        write_message(&mut bytes, &message).expect("write frame");
        assert_eq!(read_message(&mut Cursor::new(bytes)).expect("read frame"), Some(message));
    }

    #[test]
    fn parses_completion_list_and_text_edit() {
        let result = json!({"items":[{
            "label":"image",
            "textEdit":{
                "newText":"image(\"\")",
                "range":{
                    "start":{"line":2,"character":1},
                    "end":{"line":2,"character":3}
                }
            }
        }]});
        let items = parse_completion_items(&result);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].insert_text, "image(\"\")");
        assert_eq!(items[0].range.expect("range").start, LspPosition { line: 2, character: 1 });
    }

    #[test]
    fn parses_only_error_and_warning_diagnostics() {
        let error = parse_diagnostic(&json!({
            "range":{"start":{"line":0,"character":0},"end":{"line":0,"character":2}},
            "severity":1,
            "message":"unknown variable"
        }))
        .expect("diagnostic");
        assert_eq!(error.severity, TinymistDiagnosticSeverity::Error);
        assert!(parse_diagnostic(&json!({
            "range":{"start":{"line":0,"character":0},"end":{"line":0,"character":1}},
            "severity":3,
            "message":"hint"
        }))
        .is_none());
    }

    #[test]
    fn creates_real_and_virtual_document_uris() {
        let root = Path::new("/tmp/captee-project");
        assert_eq!(
            document_uri(&root.join("main.typ")).as_deref(),
            Some("file:///tmp/captee-project/main.typ")
        );
        assert_eq!(
            capture_review_uri(root).as_deref(),
            Some("file:///tmp/captee-project/.captee-capture-review.typ")
        );
    }

    #[test]
    fn initialization_declares_project_and_editor_capabilities() {
        let request = initialize_request("file:///tmp/project");
        assert_eq!(request["method"], "initialize");
        assert_eq!(request["params"]["rootUri"], "file:///tmp/project");
        assert_eq!(
            request["params"]["capabilities"]["textDocument"]["publishDiagnostics"]
                ["versionSupport"],
            true
        );
    }

    #[test]
    fn document_sync_uses_full_text_and_monotonic_versions() {
        let opened = open_document_params("file:///tmp/main.typ", 4, "#let a = 1");
        assert_eq!(opened["textDocument"]["languageId"], "typst");
        assert_eq!(opened["textDocument"]["version"], 4);
        let changed = change_document_params("file:///tmp/main.typ", 5, "#let a = 2");
        assert_eq!(changed["textDocument"]["version"], 5);
        assert_eq!(changed["contentChanges"][0]["text"], "#let a = 2");
    }

    #[test]
    fn completion_response_retains_requested_document_version() {
        let pending = Arc::new(Mutex::new(BTreeMap::from([(
            9,
            PendingCompletion { uri: "file:///tmp/main.typ".into(), version: 7 },
        )])));
        let event = parse_event(&json!({"jsonrpc":"2.0","id":9,"result":[]}), &pending)
            .expect("completion event");
        assert!(matches!(event, TinymistEvent::Completion { version: 7, request_id: 9, .. }));
    }

    #[test]
    fn unavailable_runner_has_actionable_error() {
        let error = TinymistSession::start_with_runner(
            TinymistRunner::new("/definitely-missing-captee-tinymist"),
            Path::new("/tmp/captee-project"),
        )
        .expect_err("missing runner");
        assert!(error.to_string().contains("Tinymist not found"));
    }

    #[test]
    fn terminated_server_reports_failed_event() {
        let pending = Arc::new(Mutex::new(BTreeMap::new()));
        let (sender, receiver) = mpsc::channel();
        read_events(Cursor::new(Vec::<u8>::new()), pending, sender);
        assert!(
            matches!(receiver.try_recv(), Ok(TinymistEvent::Failed(message)) if message.contains("exited"))
        );
        assert_eq!(receiver.try_recv(), Err(TryRecvError::Disconnected));
    }
}
