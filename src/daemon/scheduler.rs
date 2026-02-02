use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

use crate::config::Settings;
use crate::daemon::DaemonState;
use crate::notification::{
    ActionHandler, NotificationAction, NotificationBackend, NotificationResponse,
};
use crate::storage::TaskRepository;

pub struct Scheduler {
    state: Arc<DaemonState>,
    shutdown_rx: broadcast::Receiver<()>,
    settings_rx: watch::Receiver<Settings>,
}

impl Scheduler {
    pub fn new(
        state: Arc<DaemonState>,
        shutdown_rx: broadcast::Receiver<()>,
        settings_rx: watch::Receiver<Settings>,
    ) -> Self {
        Self {
            state,
            shutdown_rx,
            settings_rx,
        }
    }

    pub async fn run(&mut self) {
        let settings = self.settings_rx.borrow().clone();
        let mut current_interval_secs = settings.daemon.check_interval_secs;
        let mut tick_interval = tokio::time::interval(Duration::from_secs(current_interval_secs));

        info!("Scheduler started with {}s interval", current_interval_secs);

        loop {
            tokio::select! {
                _ = tick_interval.tick() => {
                    self.tick().await;
                }
                result = self.settings_rx.changed() => {
                    if result.is_ok() {
                        let new_settings = self.settings_rx.borrow_and_update().clone();
                        let new_interval = new_settings.daemon.check_interval_secs;
                        if new_interval != current_interval_secs {
                            info!(
                                "Check interval changed: {}s -> {}s",
                                current_interval_secs, new_interval
                            );
                            current_interval_secs = new_interval;
                            tick_interval = tokio::time::interval(Duration::from_secs(new_interval));
                        }
                    }
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

        let settings = self.settings_rx.borrow().clone();
        if !settings.notification.enabled {
            debug!("Notifications disabled");
            return;
        }

        self.state.ensure_notification_backend().await;

        let backend_guard = self.state.notification_backend.read().await;
        let notification_backend = match backend_guard.as_ref() {
            Some(backend) => backend,
            None => {
                debug!("Notification backend not available");
                return;
            }
        };

        // avoid sending multiple notifications simultaneously
        let in_progress_tasks = match self.state.storage.list_in_progress().await {
            Ok(tasks) => tasks,
            Err(e) => {
                error!("Failed to list in-progress tasks: {}", e);
                return;
            }
        };

        if !in_progress_tasks.is_empty() {
            debug!(
                "Task already in progress: {} ({}), skipping notification",
                in_progress_tasks[0].title,
                in_progress_tasks[0].short_id()
            );
            return;
        }

        let ready_tasks = match self.state.storage.list_ready_for_notification().await {
            Ok(tasks) => tasks,
            Err(e) => {
                error!("Failed to query tasks for notification: {}", e);
                return;
            }
        };

        if ready_tasks.is_empty() {
            debug!("No tasks ready for notification");
            return;
        }

        let pending_tasks = match self.state.storage.list_pending().await {
            Ok(tasks) => tasks,
            Err(e) => {
                error!("Failed to list pending tasks: {}", e);
                return;
            }
        };

        let next_task = match self.state.scheduler.get_next_task(pending_tasks) {
            Some(scored) => scored.task,
            None => {
                debug!("No next task from scheduler");
                return;
            }
        };

        info!(
            "Sending notification for task: {} ({})",
            next_task.title,
            next_task.short_id()
        );

        let actions = NotificationAction::default_task_actions();
        let action_id: Option<String> = match notification_backend
            .send_task_notification(&next_task, &actions)
            .await
        {
            Ok(action) => action,
            Err(e) => {
                warn!("Failed to send notification: {}", e);
                return;
            }
        };

        let response = NotificationResponse::from_action_id(action_id.as_deref());
        info!("User response: {:?}", response);

        if let Err(e) = ActionHandler::handle(&self.state.storage, &next_task, response).await {
            error!("Failed to handle notification response: {}", e);
        }
    }
}
