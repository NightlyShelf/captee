//! Desktop global shortcut registration through the XDG portal.

use ashpd::desktop::global_shortcuts::{BindShortcutsOptions, GlobalShortcuts, NewShortcut};
use ashpd::desktop::CreateSessionOptions;
use ashpd::{register_host_app_with_connection, AppID};
use futures_util::future::{select, Either};
use futures_util::{pin_mut, StreamExt};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

const APPLICATION_ID: &str = "com.nightlyshelf.captee";
const DESKTOP_ENTRY: &str =
    include_str!("../../../packaging/appimage/com.nightlyshelf.captee.desktop");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlobalShortcutEvent {
    Activated,
    Failed(String),
}

enum ShortcutControl {
    Rebind(String),
    Stop,
}

pub struct GlobalShortcutRegistration {
    events: Receiver<GlobalShortcutEvent>,
    control: Sender<ShortcutControl>,
}

impl GlobalShortcutRegistration {
    pub fn try_recv(&self) -> Result<GlobalShortcutEvent, TryRecvError> {
        self.events.try_recv()
    }

    pub fn rebind(&self, trigger: impl Into<String>) -> Result<(), String> {
        self.control
            .send(ShortcutControl::Rebind(trigger.into()))
            .map_err(|_| "global shortcut worker stopped".to_owned())
    }

    pub fn stop(&self) {
        let _ = self.control.send(ShortcutControl::Stop);
    }
}

/// Registers the capture shortcut without requiring the Captee window to have focus.
/// The returned receiver is driven by the GTK main context.
pub fn register_capture_shortcut(trigger: impl Into<String>) -> GlobalShortcutRegistration {
    let (sender, events) = mpsc::channel();
    let (control, controls) = mpsc::channel();
    let trigger = trigger.into();
    let _ = thread::Builder::new()
        .name("captee-global-shortcut".to_owned())
        .spawn(move || register_shortcut_worker(trigger, controls, sender));
    GlobalShortcutRegistration { events, control }
}

fn register_shortcut_worker(
    mut trigger: String,
    controls: Receiver<ShortcutControl>,
    sender: Sender<GlobalShortcutEvent>,
) {
    loop {
        match async_io::block_on(register_shortcut_session(&trigger, &controls, &sender)) {
            Ok(Some(next_trigger)) => {
                if let Err(error) = remove_hyprland_bind(&trigger) {
                    let _ = sender.send(GlobalShortcutEvent::Failed(error));
                    return;
                }
                trigger = next_trigger;
            }
            Ok(None) => return,
            Err(error) => {
                let _ = sender.send(GlobalShortcutEvent::Failed(error));
                match controls.recv() {
                    Ok(ShortcutControl::Rebind(next_trigger)) => trigger = next_trigger,
                    Ok(ShortcutControl::Stop) | Err(_) => return,
                }
            }
        }
    }
}

async fn register_shortcut_session(
    trigger: &str,
    controls: &Receiver<ShortcutControl>,
    sender: &Sender<GlobalShortcutEvent>,
) -> Result<Option<String>, String> {
    let sender_for_worker = sender.clone();
    ensure_desktop_entry().map_err(|error| error.to_string())?;
    let connection = ashpd::zbus::Connection::session().await.map_err(|error| error.to_string())?;
    let app_id = AppID::try_from(APPLICATION_ID).map_err(|error| error.to_string())?;
    register_host_app_with_connection(connection.clone(), app_id)
        .await
        .map_err(|error| error.to_string())?;
    let portal =
        GlobalShortcuts::with_connection(connection).await.map_err(|error| error.to_string())?;
    let session = portal
        .create_session(CreateSessionOptions::default())
        .await
        .map_err(|error| error.to_string())?;
    let shortcut =
        NewShortcut::new("capture", "Capture a screen region").preferred_trigger(Some(trigger));
    let request = portal
        .bind_shortcuts(&session, &[shortcut], None, BindShortcutsOptions::default())
        .await
        .map_err(|error| error.to_string())?;
    configure_hyprland_bind(trigger)?;
    request.response().map_err(|error| error.to_string())?;
    let mut activated = portal.receive_activated().await.map_err(|error| error.to_string())?;
    loop {
        let event = activated.next();
        let timer = async_io::Timer::after(Duration::from_millis(100));
        pin_mut!(event, timer);
        match select(event, timer).await {
            Either::Left((Some(event), _)) => {
                if event.shortcut_id() == "capture"
                    && sender_for_worker.send(GlobalShortcutEvent::Activated).is_err()
                {
                    let _ = session.close().await;
                    return Ok(None);
                }
            }
            Either::Left((None, _)) => return Ok(None),
            Either::Right((_, _)) => match controls.try_recv() {
                Ok(ShortcutControl::Rebind(next_trigger)) => {
                    session.close().await.map_err(|error| error.to_string())?;
                    return Ok(Some(next_trigger));
                }
                Ok(ShortcutControl::Stop) | Err(TryRecvError::Disconnected) => {
                    session.close().await.map_err(|error| error.to_string())?;
                    return Ok(None);
                }
                Err(TryRecvError::Empty) => {}
            },
        }
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

fn configure_hyprland_bind(trigger: &str) -> Result<(), String> {
    if !std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .split(':')
        .any(|desktop| desktop.eq_ignore_ascii_case("hyprland"))
    {
        return Ok(());
    }
    let Some(bind) = hyprland_bind_argument(trigger) else {
        return Err(format!("Unsupported global shortcut trigger: {trigger}"));
    };
    let output = Command::new("hyprctl")
        .args(["keyword", "bind", &bind])
        .output()
        .map_err(|error| format!("Could not configure Hyprland global shortcut: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(if detail.is_empty() {
            "Hyprland rejected the global shortcut bind".to_owned()
        } else {
            format!("Hyprland rejected the global shortcut bind: {detail}")
        })
    }
}

fn remove_hyprland_bind(trigger: &str) -> Result<(), String> {
    if !std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .split(':')
        .any(|desktop| desktop.eq_ignore_ascii_case("hyprland"))
    {
        return Ok(());
    }
    let Some(bind) = hyprland_unbind_argument(trigger) else {
        return Err(format!("Unsupported global shortcut trigger: {trigger}"));
    };
    let output = Command::new("hyprctl")
        .args(["keyword", "unbind", &bind])
        .output()
        .map_err(|error| format!("Could not remove Hyprland global shortcut: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(if detail.is_empty() {
            "Hyprland rejected removal of the global shortcut".to_owned()
        } else {
            format!("Hyprland rejected removal of the global shortcut: {detail}")
        })
    }
}

fn hyprland_bind_argument(trigger: &str) -> Option<String> {
    let mut parts = trigger.split('+').filter(|part| !part.is_empty());
    let key = parts.next_back()?.to_ascii_uppercase();
    let modifiers = parts.map(str::to_ascii_uppercase).collect::<Vec<_>>();
    let modifier_text = modifiers.join("_");
    Some(format!("{modifier_text},{key},global,{APPLICATION_ID}:capture"))
}

fn hyprland_unbind_argument(trigger: &str) -> Option<String> {
    let mut parts = trigger.split('+').filter(|part| !part.is_empty());
    let key = parts.next_back()?.to_ascii_uppercase();
    let modifiers = parts.map(str::to_ascii_uppercase).collect::<Vec<_>>().join("_");
    Some(format!("{modifiers},{key}"))
}

#[cfg(test)]
mod tests {
    use super::{hyprland_bind_argument, hyprland_unbind_argument};

    #[test]
    fn converts_portal_trigger_to_hyprland_global_bind() {
        assert_eq!(
            hyprland_bind_argument("CTRL+SHIFT+C"),
            Some("CTRL_SHIFT,C,global,com.nightlyshelf.captee:capture".to_owned())
        );
    }

    #[test]
    fn accepts_single_key_portal_trigger() {
        assert_eq!(
            hyprland_bind_argument("Print"),
            Some(",PRINT,global,com.nightlyshelf.captee:capture".to_owned())
        );
    }

    #[test]
    fn converts_portal_trigger_to_hyprland_unbind() {
        assert_eq!(hyprland_unbind_argument("CTRL+SHIFT+C"), Some("CTRL_SHIFT,C".to_owned()));
    }
}
