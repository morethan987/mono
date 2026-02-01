use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::Task;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Pong,

    Ok,

    Error {
        message: String,
    },

    Task {
        task: Task,
    },

    TaskList {
        tasks: Vec<Task>,
    },

    CurrentTask {
        task: Option<Task>,
    },

    DaemonStatus {
        running: bool,
        pid: u32,
        uptime_secs: u64,
        task_count: u64,
        started_at: DateTime<Utc>,
    },

    RankedTasks {
        tasks: Vec<RankedTask>,
    },

    LearningStats {
        stats: LearningStatsData,
    },

    TimeSlotRecommendation {
        task_id: String,
        task_type: String,
        recommended_slot: String,
        confidence: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedTask {
    pub task: Task,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningStatsData {
    pub total_tasks_learned: u32,
    pub task_type_stats: Vec<TaskTypeStatsData>,
    pub time_slot_stats: TimeSlotStatsData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTypeStatsData {
    pub task_type: String,
    pub total_scheduled: u32,
    pub total_completed: u32,
    pub total_postponed: u32,
    pub completion_rate: f64,
    pub best_time_slot: String,
    pub avg_duration_minutes: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSlotStatsData {
    pub morning: TimeSlotDetail,
    pub afternoon: TimeSlotDetail,
    pub evening: TimeSlotDetail,
    pub night: TimeSlotDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSlotDetail {
    pub successes: u32,
    pub failures: u32,
    pub success_rate: f64,
}

impl Response {
    pub fn ok() -> Self {
        Response::Ok
    }

    pub fn error(message: impl Into<String>) -> Self {
        Response::Error {
            message: message.into(),
        }
    }

    pub fn task(task: Task) -> Self {
        Response::Task { task }
    }

    pub fn task_list(tasks: Vec<Task>) -> Self {
        Response::TaskList { tasks }
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Response::Error { .. })
    }
}
