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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedTask {
    pub task: Task,
    pub score: f64,
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
