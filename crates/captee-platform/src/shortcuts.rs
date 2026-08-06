//! Desktop global shortcut registration through the XDG portal.

use ashpd::desktop::global_shortcuts::{BindShortcutsOptions, GlobalShortcuts, NewShortcut};
use ashpd::desktop::CreateSessionOptions;
use ashpd::{register_host_app_with_connection, AppID};
use futures_util::StreamExt;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

const APPLICATION_ID: &str = "com.nightlyshelf.captee";
const DESKTOP_ENTRY: &str =
    include_str!("../../../packaging/appimage/com.nightlyshelf.captee.desktop");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlobalShortcutEvent {
    Activated,
    Failed(String),
}

/// Registers the capture shortcut without requiring the Captee window to have focus.
/// The returned receiver is driven by the GTK main context.
pub fn register_capture_shortcut(trigger: impl Into<String>) -> Receiver<GlobalShortcutEvent> {
    let (sender, receiver) = mpsc::channel();
    let trigger = trigger.into();
    let _ = thread::Builder::new()
        .name("captee-global-shortcut".to_owned())
        .spawn(move || register_shortcut_worker(trigger, sender));
    receiver
}

fn register_shortcut_worker(trigger: String, sender: Sender<GlobalShortcutEvent>) {
    let sender_for_worker = sender.clone();
    let result = async_io::block_on(async move {
        ensure_desktop_entry().map_err(|error| error.to_string())?;
        let connection =
            ashpd::zbus::Connection::session().await.map_err(|error| error.to_string())?;
        let app_id = AppID::try_from(APPLICATION_ID).map_err(|error| error.to_string())?;
        register_host_app_with_connection(connection.clone(), app_id)
            .await
            .map_err(|error| error.to_string())?;
        let portal = GlobalShortcuts::with_connection(connection)
            .await
            .map_err(|error| error.to_string())?;
        let session = portal
            .create_session(CreateSessionOptions::default())
            .await
            .map_err(|error| error.to_string())?;
        let shortcut = NewShortcut::new("capture", "Capture a screen region")
            .preferred_trigger(Some(trigger.as_str()));
        let request = portal
            .bind_shortcuts(&session, &[shortcut], None, BindShortcutsOptions::default())
            .await
            .map_err(|error| error.to_string())?;
        request.response().map_err(|error| error.to_string())?;
        let mut activated = portal.receive_activated().await.map_err(|error| error.to_string())?;
        while let Some(event) = activated.next().await {
            if event.shortcut_id() == "capture"
                && sender_for_worker.send(GlobalShortcutEvent::Activated).is_err()
            {
                break;
            }
        }
        Ok::<(), String>(())
    });
    if let Err(error) = result {
        let _ = sender.send(GlobalShortcutEvent::Failed(error));
    }
}

/// Ensures that a host-launched development build has portal-visible app info.
///
/// Installed packages normally provide this desktop entry themselves. A
/// `cargo run` binary does not, so the host portal cannot resolve its app ID
/// until the user-level entry exists.
fn ensure_desktop_entry() -> Result<(), std::io::Error> {
    if ashpd::is_sandboxed() {
        return Ok(());
    }
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "no user data directory")
        })?;
    let applications = data_home.join("applications");
    fs::create_dir_all(&applications)?;
    let destination = applications.join(format!("{APPLICATION_ID}.desktop"));
    let contents = desktop_entry_for_current_executable()?;
    let existing = fs::read_to_string(&destination).ok();
    if existing.as_deref() == Some(contents.as_str()) {
        return Ok(());
    }
    if destination.exists() && existing.as_deref() != Some(DESKTOP_ENTRY) {
        return Ok(());
    }
    let temporary =
        applications.join(format!(".{APPLICATION_ID}.desktop.{}.tmp", std::process::id()));
    fs::write(&temporary, contents)?;
    fs::rename(temporary, destination)
}

fn desktop_entry_for_current_executable() -> Result<String, std::io::Error> {
    let executable = std::env::current_exe()?.to_string_lossy().replace('"', "\\\"");
    Ok(DESKTOP_ENTRY.replace("Exec=captee-ui", &format!("Exec=\"{executable}\"")))
}
