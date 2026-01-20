use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::{MonoPaths, Settings};
use crate::scheduling::SchedulingEngine;
use crate::storage::SqliteStorage;

pub struct DaemonState {
    pub storage: SqliteStorage,
    pub paths: MonoPaths,
    pub settings: Settings,
    pub started_at: DateTime<Utc>,
    pub scheduler: SchedulingEngine,
    shutdown_requested: Arc<RwLock<bool>>,
}

impl DaemonState {
    pub fn new(storage: SqliteStorage, paths: MonoPaths, settings: Settings) -> Self {
        Self {
            storage,
            paths,
            settings,
            started_at: Utc::now(),
            scheduler: SchedulingEngine::with_default_policies(),
            shutdown_requested: Arc::new(RwLock::new(false)),
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        (Utc::now() - self.started_at).num_seconds().max(0) as u64
    }

    pub async fn request_shutdown(&self) {
        let mut shutdown = self.shutdown_requested.write().await;
        *shutdown = true;
    }

    pub async fn is_shutdown_requested(&self) -> bool {
        *self.shutdown_requested.read().await
    }
}
