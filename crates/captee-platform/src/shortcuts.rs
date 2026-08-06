//! Desktop global shortcut registration through the XDG portal.

use ashpd::desktop::global_shortcuts::{BindShortcutsOptions, GlobalShortcuts, NewShortcut};
use ashpd::desktop::CreateSessionOptions;
use ashpd::{register_host_app_with_connection, AppID};
use futures_util::StreamExt;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

const APPLICATION_ID: &str = "com.nightlyshelf.Captee";

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
