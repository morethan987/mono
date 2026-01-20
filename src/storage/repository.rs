use async_trait::async_trait;

use crate::error::Result;
use crate::models::{Feedback, Schedule, Task, TaskStatus};

#[async_trait]
pub trait TaskRepository: Send + Sync {
    async fn create(&self, task: &Task) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Option<Task>>;
    async fn get_by_short_id(&self, short_id: &str) -> Result<Option<Task>>;
    async fn list(&self, status: Option<TaskStatus>, limit: Option<u32>) -> Result<Vec<Task>>;
    async fn list_pending(&self) -> Result<Vec<Task>>;
    async fn list_today(&self) -> Result<Vec<Task>>;
    async fn update(&self, task: &Task) -> Result<()>;
    async fn delete(&self, id: &str) -> Result<()>;
}

#[async_trait]
pub trait ScheduleRepository: Send + Sync {
    async fn create(&self, schedule: &Schedule) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Option<Schedule>>;
    async fn get_current(&self) -> Result<Option<Schedule>>;
    async fn list_for_task(&self, task_id: &str) -> Result<Vec<Schedule>>;
    async fn list_today(&self) -> Result<Vec<Schedule>>;
    async fn update(&self, schedule: &Schedule) -> Result<()>;
    async fn delete(&self, id: &str) -> Result<()>;
}

#[async_trait]
pub trait FeedbackRepository: Send + Sync {
    async fn create(&self, feedback: &Feedback) -> Result<()>;
    async fn list_for_task(&self, task_id: &str) -> Result<Vec<Feedback>>;
    async fn list_recent(&self, limit: u32) -> Result<Vec<Feedback>>;
}
