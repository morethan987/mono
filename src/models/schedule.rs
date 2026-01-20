use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ScheduleStatus {
    #[default]
    Planned,
    Active,
    Completed,
    Skipped,
}

impl ScheduleStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScheduleStatus::Planned => "planned",
            ScheduleStatus::Active => "active",
            ScheduleStatus::Completed => "completed",
            ScheduleStatus::Skipped => "skipped",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "planned" => ScheduleStatus::Planned,
            "active" => ScheduleStatus::Active,
            "completed" => ScheduleStatus::Completed,
            "skipped" => ScheduleStatus::Skipped,
            _ => ScheduleStatus::Planned,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub id: String,
    pub task_id: String,
    pub scheduled_start: DateTime<Utc>,
    pub scheduled_end: DateTime<Utc>,
    pub actual_start: Option<DateTime<Utc>>,
    pub actual_end: Option<DateTime<Utc>>,
    pub status: ScheduleStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Schedule {
    pub fn new(task_id: String, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            task_id,
            scheduled_start: start,
            scheduled_end: end,
            actual_start: None,
            actual_end: None,
            status: ScheduleStatus::default(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn duration_minutes(&self) -> i64 {
        (self.scheduled_end - self.scheduled_start).num_minutes()
    }

    pub fn is_current(&self) -> bool {
        let now = Utc::now();
        self.scheduled_start <= now && now < self.scheduled_end
    }

    pub fn is_upcoming(&self) -> bool {
        Utc::now() < self.scheduled_start
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_schedule_duration() {
        let now = Utc::now();
        let schedule = Schedule::new("task-1".to_string(), now, now + Duration::minutes(45));
        assert_eq!(schedule.duration_minutes(), 45);
    }
}
