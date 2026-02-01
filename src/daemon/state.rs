use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::RwLock;

use crate::config::{MonoPaths, Settings};
use crate::learning::LearningManager;
use crate::notification::LinuxNotificationBackend;
use crate::scheduling::SchedulingEngine;
use crate::scheduling::policy::LearningPolicy;
use crate::storage::SqliteStorage;

const SAVE_AFTER_N_UPDATES: u32 = 10;

pub struct DaemonState {
    pub storage: SqliteStorage,
    pub paths: MonoPaths,
    pub settings: Settings,
    pub started_at: DateTime<Utc>,
    pub scheduler: SchedulingEngine,
    pub notification_backend: Option<LinuxNotificationBackend>,
    pub learning_manager: Arc<RwLock<LearningManager>>,
    shutdown_requested: Arc<RwLock<bool>>,
    learning_updates_since_save: AtomicU32,
}

impl DaemonState {
    pub async fn new(storage: SqliteStorage, paths: MonoPaths, settings: Settings) -> Self {
        let learning_manager = match storage.load_learning_manager().await {
            Ok(Some(manager)) => {
                tracing::info!("Loaded learning model from database");
                Arc::new(RwLock::new(manager))
            }
            Ok(None) => {
                tracing::info!("No existing learning model, starting fresh");
                Arc::new(RwLock::new(LearningManager::new()))
            }
            Err(e) => {
                tracing::warn!("Failed to load learning model: {}, starting fresh", e);
                Arc::new(RwLock::new(LearningManager::new()))
            }
        };

        let mut scheduler = SchedulingEngine::with_default_policies();
        scheduler.add_policy(Box::new(
            LearningPolicy::new(Arc::clone(&learning_manager)).with_weight(0.5),
        ));

        Self {
            storage,
            paths,
            settings,
            started_at: Utc::now(),
            scheduler,
            notification_backend: None,
            learning_manager,
            shutdown_requested: Arc::new(RwLock::new(false)),
            learning_updates_since_save: AtomicU32::new(0),
        }
    }

    pub async fn init_notification_backend(&mut self) {
        if self.settings.notification.enabled {
            match LinuxNotificationBackend::new().await {
                Ok(backend) => {
                    tracing::info!("Notification backend initialized");
                    self.notification_backend = Some(backend);
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize notification backend: {}", e);
                }
            }
        }
    }

    pub async fn save_learning_model(&self) {
        let manager = self.learning_manager.read().await;
        if let Err(e) = self.storage.save_learning_manager(&manager).await {
            tracing::error!("Failed to save learning model: {}", e);
        } else {
            self.learning_updates_since_save.store(0, Ordering::Relaxed);
            tracing::debug!("Learning model saved to database");
        }
    }

    pub async fn maybe_save_learning_model(&self) {
        let count = self
            .learning_updates_since_save
            .fetch_add(1, Ordering::Relaxed)
            + 1;
        if count >= SAVE_AFTER_N_UPDATES {
            self.save_learning_model().await;
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        (Utc::now() - self.started_at).num_seconds().max(0) as u64
    }

    pub async fn request_shutdown(&self) {
        self.save_learning_model().await;
        let mut shutdown = self.shutdown_requested.write().await;
        *shutdown = true;
    }

    pub async fn is_shutdown_requested(&self) -> bool {
        *self.shutdown_requested.read().await
    }
}
