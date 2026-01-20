use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::task_type::TaskType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    #[default]
    Medium,
    High,
    Urgent,
}

impl Priority {
    pub fn as_i32(&self) -> i32 {
        match self {
            Priority::Low => 0,
            Priority::Medium => 1,
            Priority::High => 2,
            Priority::Urgent => 3,
        }
    }

    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => Priority::Low,
            1 => Priority::Medium,
            2 => Priority::High,
            3 => Priority::Urgent,
            _ => Priority::Medium,
        }
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Low => write!(f, "low"),
            Priority::Medium => write!(f, "medium"),
            Priority::High => write!(f, "high"),
            Priority::Urgent => write!(f, "urgent"),
        }
    }
}

impl std::str::FromStr for Priority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" | "l" => Ok(Priority::Low),
            "medium" | "med" | "m" => Ok(Priority::Medium),
            "high" | "h" => Ok(Priority::High),
            "urgent" | "u" => Ok(Priority::Urgent),
            _ => Err(format!("Invalid priority: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    Cancelled,
    Postponed,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Completed => "completed",
            TaskStatus::Cancelled => "cancelled",
            TaskStatus::Postponed => "postponed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => TaskStatus::Pending,
            "in_progress" => TaskStatus::InProgress,
            "completed" => TaskStatus::Completed,
            "cancelled" => TaskStatus::Cancelled,
            "postponed" => TaskStatus::Postponed,
            _ => TaskStatus::Pending,
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub priority: Priority,
    pub status: TaskStatus,
    pub tags: Vec<String>,
    pub estimated_minutes: Option<u32>,
    pub actual_minutes: Option<u32>,
    pub deadline: Option<DateTime<Utc>>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    pub fn new(title: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            description: None,
            priority: Priority::default(),
            status: TaskStatus::default(),
            tags: Vec::new(),
            estimated_minutes: None,
            actual_minutes: None,
            deadline: None,
            scheduled_at: None,
            started_at: None,
            completed_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_estimated_minutes(mut self, minutes: u32) -> Self {
        self.estimated_minutes = Some(minutes);
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn task_type(&self) -> TaskType {
        TaskType::from_tags(&self.tags)
    }

    pub fn is_overdue(&self) -> bool {
        if let Some(deadline) = self.deadline {
            self.status != TaskStatus::Completed && deadline < Utc::now()
        } else {
            false
        }
    }

    pub fn estimated_duration_display(&self) -> String {
        match self.estimated_minutes {
            Some(mins) if mins >= 60 => {
                let hours = mins / 60;
                let remaining = mins % 60;
                if remaining > 0 {
                    format!("{}h {}m", hours, remaining)
                } else {
                    format!("{}h", hours)
                }
            }
            Some(mins) => format!("{}m", mins),
            None => "未设置".to_string(),
        }
    }

    pub fn short_id(&self) -> &str {
        &self.id[..8.min(self.id.len())]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub tags: Vec<String>,
    pub estimated_minutes: Option<u32>,
    pub deadline: Option<DateTime<Utc>>,
}

impl From<CreateTaskRequest> for Task {
    fn from(req: CreateTaskRequest) -> Self {
        let mut task = Task::new(req.title);
        task.description = req.description;
        task.priority = req.priority.unwrap_or_default();
        task.tags = req.tags;
        task.estimated_minutes = req.estimated_minutes;
        task.deadline = req.deadline;
        task
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("Test task".to_string());
        assert_eq!(task.title, "Test task");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.priority, Priority::Medium);
    }

    #[test]
    fn test_priority_from_str() {
        assert_eq!("high".parse::<Priority>().unwrap(), Priority::High);
        assert_eq!("h".parse::<Priority>().unwrap(), Priority::High);
        assert_eq!("LOW".parse::<Priority>().unwrap(), Priority::Low);
    }

    #[test]
    fn test_duration_display() {
        let task = Task::new("Test".to_string()).with_estimated_minutes(90);
        assert_eq!(task.estimated_duration_display(), "1h 30m");

        let task = Task::new("Test".to_string()).with_estimated_minutes(60);
        assert_eq!(task.estimated_duration_display(), "1h");

        let task = Task::new("Test".to_string()).with_estimated_minutes(45);
        assert_eq!(task.estimated_duration_display(), "45m");
    }
}
