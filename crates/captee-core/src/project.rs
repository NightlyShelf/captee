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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub formatting: FormattingSettings,
    pub capture: CaptureSettings,
    pub preview: PreviewSettings,
    #[serde(default)]
    pub keybindings: KeybindingSettings,
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
            capture: "<Primary><Shift>c".to_owned(),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentProjects {
    pub max_entries: usize,
    pub paths: Vec<String>,
}

impl Default for RecentProjects {
    fn default() -> Self {
        Self { max_entries: 10, paths: Vec::new() }
    }
}

impl RecentProjects {
    pub fn record(&mut self, path: impl Into<String>) {
        let path = path.into();
        self.paths.retain(|existing| existing != &path);
        self.paths.insert(0, path);
        self.paths.truncate(self.max_entries);
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
    fn older_project_settings_receive_default_keybindings() {
        let json = r#"{
          "version": 1,
          "name": "Notes",
          "entry_document": "main.typ",
          "settings": {
            "formatting": { "line_width": 90, "format_on_save": false },
            "capture": { "portal_enabled": true, "fallback_enabled": true },
            "preview": { "auto_render": true, "zoom_percent": 100 }
          }
        }"#;
        let config = ProjectConfig::from_json(json).expect("old config remains readable");
        assert_eq!(config.settings.keybindings, KeybindingSettings::default());
    }

    #[test]
    fn invalid_entry_documents_are_rejected() {
        assert!(ProjectConfig::new("Notes", "../main.typ").is_err());
        assert!(ProjectConfig::new("Notes", "main.txt").is_err());
    }

    #[test]
    fn recent_projects_are_deduplicated_and_bounded() {
        let mut recent = RecentProjects { max_entries: 2, paths: Vec::new() };
        recent.record("a");
        recent.record("b");
        recent.record("a");
        assert_eq!(recent.paths, vec!["a", "b"]);
        recent.record("c");
        assert_eq!(recent.paths, vec!["c", "a"]);
    }
}
