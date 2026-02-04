use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::RwLock;

use crate::config::{MonoPaths, Settings};
use crate::learning::LearningManager;
use crate::notification::LinuxNotificationBackend;
use crate::scheduling::{SchedulingContext, SchedulingEngine};
use crate::scheduling::policy::LearningPolicy;
use crate::storage::{SqliteStorage, TaskRepository};

const SAVE_AFTER_N_UPDATES: u32 = 10;

pub struct DaemonState {
    pub storage: SqliteStorage,
    pub paths: MonoPaths,
    pub started_at: DateTime<Utc>,
    pub scheduler: SchedulingEngine,
    pub notification_backend: RwLock<Option<LinuxNotificationBackend>>,
    pub learning_manager: Arc<RwLock<LearningManager>>,
    pub mismatch_counter: std::sync::atomic::AtomicU32,
    shutdown_requested: Arc<RwLock<bool>>,
    learning_updates_since_save: AtomicU32,
    initial_notification_enabled: bool,
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
            started_at: Utc::now(),
            scheduler,
            notification_backend: RwLock::new(None),
            learning_manager,
            mismatch_counter: std::sync::atomic::AtomicU32::new(0),
            shutdown_requested: Arc::new(RwLock::new(false)),
            learning_updates_since_save: AtomicU32::new(0),
            initial_notification_enabled: settings.notification.enabled,
        }
    }

    pub async fn build_context(&self) -> SchedulingContext {
        let mut context = SchedulingContext::new();

        let in_progress = match self.storage.list_in_progress().await {
            Ok(tasks) => tasks,
            Err(_) => Vec::new(),
        };

        if let Some(task) = in_progress.first() {
            if let Some(start) = task.started_at {
                let duration = Utc::now().signed_duration_since(start);
                context.current_session_duration = duration.num_minutes().max(0) as u32;
            }
        }

        context.session_interruptions = self.mismatch_counter.load(std::sync::atomic::Ordering::Relaxed);
        context
    }

    pub async fn init_notification_backend(&self) {
        if self.initial_notification_enabled {
            match LinuxNotificationBackend::new().await {
                Ok(backend) => {
                    tracing::info!("Notification backend initialized");
                    *self.notification_backend.write().await = Some(backend);
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize notification backend: {}", e);
                }
            }
        }
    }

    pub async fn ensure_notification_backend(&self) {
        let mut backend = self.notification_backend.write().await;
        if backend.is_none() {
            match LinuxNotificationBackend::new().await {
                Ok(new_backend) => {
                    tracing::info!("Notification backend initialized on demand");
                    *backend = Some(new_backend);
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
