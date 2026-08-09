use serde::{Deserialize, Serialize};
use std::fmt;

/// Current on-disk project configuration schema version.
pub const CONFIG_VERSION: u32 = 1;

/// A validated project configuration stored at the project root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub version: u32,
    pub name: String,
    pub entry_document: String,
    #[serde(default)]
    pub settings: ProjectSettings,
}

impl ProjectConfig {
    pub fn new(
        name: impl Into<String>,
        entry_document: impl Into<String>,
    ) -> Result<Self, ConfigError> {
        let config = Self {
            version: CONFIG_VERSION,
            name: name.into(),
            entry_document: entry_document.into(),
            settings: ProjectSettings::default(),
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.version != CONFIG_VERSION {
            return Err(ConfigError::UnsupportedVersion(self.version));
        }
        if self.name.trim().is_empty() {
            return Err(ConfigError::EmptyName);
        }
        if !is_safe_relative_typst_path(&self.entry_document) {
            return Err(ConfigError::InvalidEntryDocument(self.entry_document.clone()));
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, ConfigError> {
        self.validate()?;
        serde_json::to_string_pretty(self).map_err(ConfigError::Serialization)
    }

    pub fn from_json(input: &str) -> Result<Self, ConfigError> {
        let config: Self = serde_json::from_str(input).map_err(ConfigError::Serialization)?;
        config.validate()?;
        Ok(config)
    }
}

fn is_safe_relative_typst_path(path: &str) -> bool {
    let path = std::path::Path::new(path);
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path.extension().is_some_and(|extension| extension == "typ")
        && path.components().all(|component| !matches!(component, std::path::Component::ParentDir))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub formatting: FormattingSettings,
    pub capture: CaptureSettings,
    pub preview: PreviewSettings,
    #[serde(default, rename = "keybindings", skip_serializing)]
    legacy_keybindings: Option<KeybindingSettings>,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            formatting: FormattingSettings::default(),
            capture: CaptureSettings::default(),
            preview: PreviewSettings::default(),
            legacy_keybindings: None,
        }
    }
}

impl ProjectSettings {
    pub fn legacy_keybindings(&self) -> Option<&KeybindingSettings> {
        self.legacy_keybindings.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeybindingSettings {
    pub save: String,
    pub format: String,
    pub find_replace: String,
    pub completion: String,
    pub capture: String,
    pub preview: String,
    pub export: String,
}

impl Default for KeybindingSettings {
    fn default() -> Self {
        Self {
            save: "<Primary>s".to_owned(),
            format: "<Primary><Shift>f".to_owned(),
            find_replace: "<Primary>f".to_owned(),
            completion: "<Primary>space".to_owned(),
            capture: "<Primary>asciitilde".to_owned(),
            preview: "<Primary>r".to_owned(),
            export: "<Primary><Shift>e".to_owned(),
        }
    }
}

impl KeybindingSettings {
    pub fn named_bindings(&self) -> [(&'static str, &str); 7] {
        [
            ("Save", &self.save),
            ("Format", &self.format),
            ("Find and Replace", &self.find_replace),
            ("Completion", &self.completion),
            ("Capture", &self.capture),
            ("Preview", &self.preview),
            ("Export PDF", &self.export),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormattingSettings {
    pub line_width: u16,
    pub format_on_save: bool,
}

impl Default for FormattingSettings {
    fn default() -> Self {
        Self { line_width: 100, format_on_save: false }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureSettings {
    pub portal_enabled: bool,
    pub fallback_enabled: bool,
}

impl Default for CaptureSettings {
    fn default() -> Self {
        Self { portal_enabled: true, fallback_enabled: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewSettings {
    pub auto_render: bool,
    pub zoom_percent: u16,
}

impl Default for PreviewSettings {
    fn default() -> Self {
        Self { auto_render: true, zoom_percent: 100 }
    }
}

pub const RECENT_PROJECT_LIMIT: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentProject {
    pub name: String,
    pub path: String,
    pub last_access_unix_seconds: u64,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentProjects {
    #[serde(default)]
    pub entries: Vec<RecentProject>,
    #[serde(default, rename = "paths", skip_serializing)]
    legacy_paths: Vec<String>,
}

impl RecentProjects {
    pub fn record(
        &mut self,
        name: impl Into<String>,
        path: impl Into<String>,
        last_access_unix_seconds: u64,
    ) {
        let path = path.into();
        let pinned =
            self.entries.iter().find(|entry| entry.path == path).is_some_and(|entry| entry.pinned);
        self.entries.retain(|entry| entry.path != path);
        self.entries.push(RecentProject {
            name: name.into(),
            path,
            last_access_unix_seconds,
            pinned,
        });
        self.sort_and_limit();
    }

    pub fn set_pinned(&mut self, path: &str, pinned: bool) -> bool {
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.path == path) else {
            return false;
        };
        entry.pinned = pinned;
        self.sort_and_limit();
        true
    }

    pub fn remove(&mut self, path: &str) -> bool {
        let entry_count = self.entries.len();
        self.entries.retain(|entry| entry.path != path);
        entry_count != self.entries.len()
    }

    pub fn migrate_legacy_paths(&mut self) {
        if !self.entries.is_empty() || self.legacy_paths.is_empty() {
            return;
        }

        let count = self.legacy_paths.len() as u64;
        for (index, path) in self.legacy_paths.drain(..).enumerate() {
            let name = std::path::Path::new(&path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&path)
                .to_owned();
            self.entries.push(RecentProject {
                name,
                path,
                last_access_unix_seconds: count.saturating_sub(index as u64),
                pinned: false,
            });
        }
        self.sort_and_limit();
    }

    fn sort_and_limit(&mut self) {
        self.entries.sort_by(|left, right| {
            right
                .pinned
                .cmp(&left.pinned)
                .then_with(|| right.last_access_unix_seconds.cmp(&left.last_access_unix_seconds))
        });
        self.entries.truncate(RECENT_PROJECT_LIMIT);
    }
}

#[derive(Debug)]
pub enum ConfigError {
    EmptyName,
    InvalidEntryDocument(String),
    UnsupportedVersion(u32),
    Serialization(serde_json::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => formatter.write_str("project name cannot be empty"),
            Self::InvalidEntryDocument(path) => write!(formatter, "invalid entry document: {path}"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported config version: {version}")
            }
            Self::Serialization(error) => {
                write!(formatter, "configuration serialization failed: {error}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_round_trips_as_stable_json() {
        let config = ProjectConfig::new("Notes", "main.typ").expect("valid config");
        let json = config.to_json().expect("serializes");
        let decoded = ProjectConfig::from_json(&json).expect("deserializes");
        assert_eq!(decoded, config);
        assert!(json.contains("\"version\": 1"));
    }

    #[test]
    fn legacy_project_keybindings_are_read_but_not_written() {
        let json = r#"{
          "version": 1,
          "name": "Notes",
          "entry_document": "main.typ",
          "settings": {
            "formatting": { "line_width": 90, "format_on_save": false },
            "capture": { "portal_enabled": true, "fallback_enabled": true },
            "preview": { "auto_render": true, "zoom_percent": 100 },
            "keybindings": {
              "save": "<Primary>s",
              "format": "<Primary><Shift>f",
              "find_replace": "<Primary>f",
              "completion": "<Primary>space",
              "capture": "<Primary>asciitilde",
              "preview": "<Primary>r",
              "export": "<Primary><Shift>e"
            }
          }
        }"#;
        let config = ProjectConfig::from_json(json).expect("old config remains readable");
        assert_eq!(config.settings.legacy_keybindings(), Some(&KeybindingSettings::default()));
        assert!(!config.to_json().expect("serialize").contains("keybindings"));
    }

    #[test]
    fn invalid_entry_documents_are_rejected() {
        assert!(ProjectConfig::new("Notes", "../main.typ").is_err());
        assert!(ProjectConfig::new("Notes", "main.txt").is_err());
    }

    #[test]
    fn recent_projects_are_deduplicated_pinned_and_bounded() {
        let mut recent = RecentProjects::default();
        for index in 0..6 {
            recent.record(format!("Project {index}"), format!("project-{index}"), index);
        }
        recent.set_pinned("project-1", true);
        recent.record("New name", "project-1", 9);

        assert_eq!(recent.entries.len(), RECENT_PROJECT_LIMIT);
        assert_eq!(recent.entries[0].path, "project-1");
        assert!(recent.entries[0].pinned);
        assert_eq!(recent.entries[0].name, "New name");
        assert!(!recent.entries.iter().any(|entry| entry.path == "project-0"));
    }

    #[test]
    fn legacy_recent_project_paths_are_migrated() {
        let mut recent: RecentProjects =
            serde_json::from_str(r#"{"max_entries":10,"paths":["/work/Notes","/work/Plan"]}"#)
                .expect("legacy recent projects");

        recent.migrate_legacy_paths();

        assert_eq!(recent.entries[0].name, "Notes");
        assert_eq!(recent.entries[1].name, "Plan");
        assert!(recent.entries.iter().all(|entry| !entry.pinned));
    }
}
