use crate::error::{MonoError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub daemon: DaemonSettings,
    #[serde(default)]
    pub notification: NotificationSettings,
    #[serde(default)]
    pub scheduling: SchedulingSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonSettings {
    #[serde(default = "default_check_interval")]
    pub check_interval_secs: u64,
    #[serde(default = "default_ipc_timeout")]
    pub ipc_timeout_secs: u64,
}

fn default_check_interval() -> u64 {
    60
}

fn default_ipc_timeout() -> u64 {
    5
}

impl Default for DaemonSettings {
    fn default() -> Self {
        Self {
            check_interval_secs: default_check_interval(),
            ipc_timeout_secs: default_ipc_timeout(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_sound")]
    pub sound: bool,
    #[serde(default = "default_persist")]
    pub persist: bool,
}

fn default_enabled() -> bool {
    true
}

fn default_sound() -> bool {
    true
}

fn default_persist() -> bool {
    true
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            sound: default_sound(),
            persist: default_persist(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingSettings {
    #[serde(default = "default_work_start_hour")]
    pub work_start_hour: u8,
    #[serde(default = "default_work_end_hour")]
    pub work_end_hour: u8,
    #[serde(default = "default_default_duration_mins")]
    pub default_duration_mins: u32,
}

fn default_work_start_hour() -> u8 {
    9
}

fn default_work_end_hour() -> u8 {
    18
}

fn default_default_duration_mins() -> u32 {
    30
}

impl Default for SchedulingSettings {
    fn default() -> Self {
        Self {
            work_start_hour: default_work_start_hour(),
            work_end_hour: default_work_end_hour(),
            default_duration_mins: default_default_duration_mins(),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            daemon: DaemonSettings::default(),
            notification: NotificationSettings::default(),
            scheduling: SchedulingSettings::default(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path).map_err(|e| MonoError::DirectoryCreation {
            path: path.to_path_buf(),
            source: e,
        })?;

        let settings: Settings = toml::from_str(&content)?;
        Ok(settings)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| MonoError::DirectoryCreation {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(path, content).map_err(|e| MonoError::DirectoryCreation {
            path: path.to_path_buf(),
            source: e,
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.daemon.check_interval_secs, 60);
        assert!(settings.notification.enabled);
        assert_eq!(settings.scheduling.work_start_hour, 9);
    }

    #[test]
    fn test_settings_serialization() {
        let settings = Settings::default();
        let toml_str = toml::to_string(&settings).unwrap();
        let parsed: Settings = toml::from_str(&toml_str).unwrap();
        assert_eq!(
            parsed.daemon.check_interval_secs,
            settings.daemon.check_interval_secs
        );
    }
}
