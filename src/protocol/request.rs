use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::{Priority, TaskStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    Ping,
    Shutdown,

    AddTask {
        title: String,
        description: Option<String>,
        priority: Option<Priority>,
        tags: Vec<String>,
        estimated_minutes: Option<u32>,
        deadline: Option<DateTime<Utc>>,
    },

    GetTask {
        id: String,
    },

    ListTasks {
        status: Option<TaskStatus>,
        limit: Option<u32>,
    },

    ListToday,

    GetCurrentTask,

    UpdateTaskStatus {
        id: String,
        status: TaskStatus,
    },

    CompleteTask {
        id: String,
        actual_minutes: Option<u32>,
    },

    PostponeTask {
        id: String,
        minutes: u32,
    },

    DeleteTask {
        id: String,
    },

    UpdateTask {
        id: String,
        title: Option<String>,
        description: Option<String>,
        priority: Option<Priority>,
        tags: Option<Vec<String>>,
        estimated_minutes: Option<u32>,
        deadline: Option<DateTime<Utc>>,
    },

    SubmitFeedback {
        task_id: String,
        rating: Option<u8>,
        difficulty: Option<u8>,
        energy_level: Option<u8>,
        notes: Option<String>,
    },

    GetLearningStats {
        task_type: Option<String>,
    },

    GetTimeSlotRecommendation {
        task_id: String,
    },

    GetDaemonStatus,

    Replan,
}

impl Request {
    pub fn name(&self) -> &'static str {
        match self {
            Request::Ping => "ping",
            Request::Shutdown => "shutdown",
            Request::AddTask { .. } => "add_task",
            Request::GetTask { .. } => "get_task",
            Request::ListTasks { .. } => "list_tasks",
            Request::ListToday => "list_today",
            Request::GetCurrentTask => "get_current_task",
            Request::UpdateTaskStatus { .. } => "update_task_status",
            Request::CompleteTask { .. } => "complete_task",
            Request::PostponeTask { .. } => "postpone_task",
            Request::DeleteTask { .. } => "delete_task",
            Request::UpdateTask { .. } => "update_task",
            Request::SubmitFeedback { .. } => "submit_feedback",
            Request::GetLearningStats { .. } => "get_learning_stats",
            Request::GetTimeSlotRecommendation { .. } => "get_time_slot_recommendation",
            Request::GetDaemonStatus => "get_daemon_status",
            Request::Replan => "replan",
        }
    }
}
