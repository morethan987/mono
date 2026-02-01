use chrono::{Duration, Utc};
use tracing::{debug, info};

use crate::error::Result;
use crate::models::{Task, TaskStatus};
use crate::storage::TaskRepository;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationResponse {
    Start,
    Postpone { minutes: u32 },
    Skip,
    Dismissed,
}

impl NotificationResponse {
    pub fn from_action_id(action_id: Option<&str>) -> Self {
        match action_id {
            Some("start") => Self::Start,
            Some("postpone_15") => Self::Postpone { minutes: 15 },
            Some("postpone_30") => Self::Postpone { minutes: 30 },
            Some("postpone_60") => Self::Postpone { minutes: 60 },
            Some("skip") => Self::Skip,
            Some(id) if id.starts_with("postpone_") => {
                let mins = id.strip_prefix("postpone_").and_then(|s| s.parse().ok());
                Self::Postpone {
                    minutes: mins.unwrap_or(15),
                }
            }
            _ => Self::Dismissed,
        }
    }
}

pub struct ActionHandler;

impl ActionHandler {
    pub async fn handle<R: TaskRepository>(
        repo: &R,
        task: &Task,
        response: NotificationResponse,
    ) -> Result<Task> {
        match response {
            NotificationResponse::Start => Self::start_task(repo, task).await,
            NotificationResponse::Postpone { minutes } => {
                Self::postpone_task(repo, task, minutes).await
            }
            NotificationResponse::Skip => Self::skip_task(repo, task).await,
            NotificationResponse::Dismissed => {
                debug!("Notification dismissed for task: {}", task.short_id());
                Ok(task.clone())
            }
        }
    }

    async fn start_task<R: TaskRepository>(repo: &R, task: &Task) -> Result<Task> {
        info!("Starting task: {} ({})", task.title, task.short_id());

        let mut updated = task.clone();
        updated.status = TaskStatus::InProgress;
        updated.started_at = Some(Utc::now());
        updated.updated_at = Utc::now();

        repo.update(&updated).await?;
        Ok(updated)
    }

    async fn postpone_task<R: TaskRepository>(
        repo: &R,
        task: &Task,
        minutes: u32,
    ) -> Result<Task> {
        info!(
            "Postponing task: {} ({}) by {} minutes",
            task.title,
            task.short_id(),
            minutes
        );

        let mut updated = task.clone();
        updated.scheduled_at = Some(Utc::now() + Duration::minutes(minutes as i64));
        updated.status = TaskStatus::Postponed;
        updated.updated_at = Utc::now();

        repo.update(&updated).await?;
        Ok(updated)
    }

    async fn skip_task<R: TaskRepository>(repo: &R, task: &Task) -> Result<Task> {
        info!("Skipping task: {} ({})", task.title, task.short_id());

        let mut updated = task.clone();
        updated.status = TaskStatus::Postponed;
        updated.scheduled_at = None;
        updated.updated_at = Utc::now();

        repo.update(&updated).await?;
        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_from_action_id() {
        assert_eq!(
            NotificationResponse::from_action_id(Some("start")),
            NotificationResponse::Start
        );
        assert_eq!(
            NotificationResponse::from_action_id(Some("postpone_15")),
            NotificationResponse::Postpone { minutes: 15 }
        );
        assert_eq!(
            NotificationResponse::from_action_id(Some("skip")),
            NotificationResponse::Skip
        );
        assert_eq!(
            NotificationResponse::from_action_id(None),
            NotificationResponse::Dismissed
        );
        assert_eq!(
            NotificationResponse::from_action_id(Some("postpone_45")),
            NotificationResponse::Postpone { minutes: 45 }
        );
    }
}
