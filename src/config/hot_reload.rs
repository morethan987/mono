use crate::config::watcher::{FileEvent, FileWatcher, InotifyWatcher};
use crate::config::Settings;
use crate::error::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::watch;
use tracing::{error, info, warn};

pub struct ConfigWatcher {
    config_path: PathBuf,
    watcher: Arc<InotifyWatcher>,
    settings_tx: watch::Sender<Settings>,
    settings_rx: watch::Receiver<Settings>,
}

impl ConfigWatcher {
    pub async fn new(config_path: PathBuf) -> Result<Self> {
        let initial_settings = Settings::load(&config_path)?;
        let (settings_tx, settings_rx) = watch::channel(initial_settings);

        let watcher = Arc::new(InotifyWatcher::new()?);

        if config_path.exists() {
            watcher.watch(&config_path).await?;
        } else if let Some(parent) = config_path.parent() {
            if parent.exists() {
                watcher.watch(parent).await?;
            }
        }

        Ok(Self {
            config_path,
            watcher,
            settings_tx,
            settings_rx,
        })
    }

    pub fn subscribe(&self) -> watch::Receiver<Settings> {
        self.settings_rx.clone()
    }

    pub fn current_settings(&self) -> Settings {
        self.settings_rx.borrow().clone()
    }

    pub async fn run(&self) {
        info!("Config watcher started for: {}", self.config_path.display());

        while self.watcher.is_active() {
            match self.watcher.next_event().await {
                Ok(Some(event)) => {
                    if self.should_reload(&event) {
                        self.reload_config().await;
                    }
                }
                Ok(None) => {
                    info!("Config watcher stopped");
                    break;
                }
                Err(e) => {
                    error!("Config watcher error: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    fn should_reload(&self, event: &FileEvent) -> bool {
        matches!(
            event,
            FileEvent::Modified | FileEvent::ClosedWrite | FileEvent::Created
        )
    }

    async fn reload_config(&self) {
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        match Settings::load(&self.config_path) {
            Ok(new_settings) => {
                if self.settings_tx.send(new_settings).is_ok() {
                    info!("Configuration reloaded successfully");
                }
            }
            Err(e) => {
                warn!("Failed to reload configuration: {}", e);
            }
        }
    }

    pub async fn stop(&self) -> Result<()> {
        self.watcher.stop().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_config_watcher_creation() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(&config_path, "[daemon]\ncheck_interval_secs = 30").unwrap();

        let watcher = ConfigWatcher::new(config_path).await;
        assert!(watcher.is_ok());

        let watcher = watcher.unwrap();
        let settings = watcher.current_settings();
        assert_eq!(settings.daemon.check_interval_secs, 30);
    }

    #[tokio::test]
    async fn test_config_watcher_default_when_missing() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("nonexistent.toml");

        let watcher = ConfigWatcher::new(config_path).await;
        assert!(watcher.is_ok());

        let watcher = watcher.unwrap();
        let settings = watcher.current_settings();
        assert_eq!(settings.daemon.check_interval_secs, 60);
    }

    #[tokio::test]
    async fn test_subscribe() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(&config_path, "").unwrap();

        let watcher = ConfigWatcher::new(config_path).await.unwrap();
        let mut rx = watcher.subscribe();

        let settings = rx.borrow_and_update().clone();
        assert_eq!(settings.daemon.check_interval_secs, 60);
    }
}
