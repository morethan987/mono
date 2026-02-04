use async_trait::async_trait;

use crate::error::Result;
use crate::models::{Feedback, Schedule, Task, TaskStatus};

#[derive(Debug, Clone)]
pub struct TaskTypeStats {
    pub task_type: String,
    pub total_scheduled: u32,
    pub total_completed: u32,
    pub total_postponed: u32,
    pub total_skipped: u32,
    pub avg_completion_rate: f64,
    pub avg_duration_minutes: Option<f64>,
    pub sum_duration: f64,
    pub sum_duration_sq: f64,
    pub duration_count: u32,
    pub best_time_slots: Vec<String>,
    pub model_weights: String,
}

impl TaskTypeStats {
    pub fn new(task_type: String) -> Self {
        Self {
            task_type,
            total_scheduled: 0,
            total_completed: 0,
            total_postponed: 0,
            total_skipped: 0,
            avg_completion_rate: 0.0,
            avg_duration_minutes: None,
            sum_duration: 0.0,
            sum_duration_sq: 0.0,
            duration_count: 0,
            best_time_slots: Vec::new(),
            model_weights: "{}".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimeSlotStats {
    pub id: i64,
    pub task_type: String,
    pub hour_of_day: u32,
    pub day_of_week: u32,
    pub success_count: u32,
    pub total_count: u32,
    pub avg_rating: Option<f64>,
}

#[async_trait]
pub trait TaskRepository: Send + Sync {
    async fn create(&self, task: &Task) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Option<Task>>;
    async fn get_by_short_id(&self, short_id: &str) -> Result<Option<Task>>;
    async fn list(&self, status: Option<TaskStatus>, limit: Option<u32>) -> Result<Vec<Task>>;
    async fn list_pending(&self) -> Result<Vec<Task>>;
    async fn list_in_progress(&self) -> Result<Vec<Task>>;
    async fn list_today(&self) -> Result<Vec<Task>>;
    async fn list_ready_for_notification(&self) -> Result<Vec<Task>>;
    async fn update(&self, task: &Task) -> Result<()>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn get_children(&self, parent_id: &str) -> Result<Vec<Task>>;
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

#[async_trait]
pub trait LearningRepository: Send + Sync {
    async fn get_task_type_stats(&self, task_type: &str) -> Result<Option<TaskTypeStats>>;
    async fn upsert_task_type_stats(&self, stats: &TaskTypeStats) -> Result<()>;
    async fn list_all_task_type_stats(&self) -> Result<Vec<TaskTypeStats>>;

    async fn get_time_slot_stats(
        &self,
        task_type: &str,
        hour: u32,
        day: u32,
    ) -> Result<Option<TimeSlotStats>>;
    async fn upsert_time_slot_stats(&self, stats: &TimeSlotStats) -> Result<()>;
    async fn list_time_slot_stats_for_type(&self, task_type: &str) -> Result<Vec<TimeSlotStats>>;
}
