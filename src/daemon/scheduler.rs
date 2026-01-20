use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, info};

use crate::daemon::DaemonState;

pub struct Scheduler {
    state: Arc<DaemonState>,
    shutdown_rx: broadcast::Receiver<()>,
}

impl Scheduler {
    pub fn new(state: Arc<DaemonState>, shutdown_rx: broadcast::Receiver<()>) -> Self {
        Self { state, shutdown_rx }
    }

    pub async fn run(&mut self) {
        let interval = Duration::from_secs(self.state.settings.daemon.check_interval_secs);
        let mut tick_interval = tokio::time::interval(interval);

        info!("Scheduler started with {}s interval", interval.as_secs());

        loop {
            tokio::select! {
                _ = tick_interval.tick() => {
                    self.tick().await;
                }
                _ = self.shutdown_rx.recv() => {
                    info!("Scheduler shutting down");
                    break;
                }
            }
        }
    }

    async fn tick(&self) {
        debug!("Scheduler tick at {}", chrono::Utc::now());
        // Phase 1: Stub - just log
        // Phase 2 will add: check for upcoming tasks, trigger notifications
    }
}
