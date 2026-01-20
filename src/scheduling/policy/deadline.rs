use chrono::Utc;

use crate::models::Task;
use crate::scheduling::policy::{SchedulingContext, SchedulingPolicy};

const SCORE_OVERDUE: f64 = 1.0;
const SCORE_CRITICAL: f64 = 0.95;
const SCORE_WARNING_MAX: f64 = 0.95;
const SCORE_WARNING_MIN: f64 = 0.7;
const SCORE_UPCOMING_MAX: f64 = 0.7;
const SCORE_UPCOMING_MIN: f64 = 0.4;
const SCORE_FAR_FUTURE: f64 = 0.2;
const SCORE_NO_DEADLINE: f64 = 0.3;

const HOURS_THREE_DAYS: i64 = 72;

pub struct DeadlinePolicy {
    critical_threshold_hours: i64,
    warning_threshold_hours: i64,
}

impl DeadlinePolicy {
    pub fn new() -> Self {
        Self {
            critical_threshold_hours: 4,
            warning_threshold_hours: 24,
        }
    }

    pub fn with_thresholds(critical_hours: i64, warning_hours: i64) -> Self {
        Self {
            critical_threshold_hours: critical_hours,
            warning_threshold_hours: warning_hours,
        }
    }
}

impl Default for DeadlinePolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl SchedulingPolicy for DeadlinePolicy {
    fn name(&self) -> &'static str {
        "deadline"
    }

    fn weight(&self) -> f64 {
        1.5
    }

    fn score(&self, task: &Task, context: &SchedulingContext) -> f64 {
        let Some(deadline) = task.deadline else {
            return SCORE_NO_DEADLINE;
        };

        let hours_until = (deadline - context.now).num_hours();

        if hours_until <= 0 {
            SCORE_OVERDUE
        } else if hours_until <= self.critical_threshold_hours {
            SCORE_CRITICAL
        } else if hours_until <= self.warning_threshold_hours {
            let ratio = 1.0 - (hours_until as f64 / self.warning_threshold_hours as f64);
            SCORE_WARNING_MIN + (ratio * (SCORE_WARNING_MAX - SCORE_WARNING_MIN))
        } else if hours_until <= HOURS_THREE_DAYS {
            let ratio = 1.0 - (hours_until as f64 / HOURS_THREE_DAYS as f64);
            SCORE_UPCOMING_MIN + (ratio * (SCORE_UPCOMING_MAX - SCORE_UPCOMING_MIN))
        } else {
            SCORE_FAR_FUTURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_overdue_task() {
        let policy = DeadlinePolicy::new();
        let context = SchedulingContext::default();

        let mut task = Task::new("Overdue".to_string());
        task.deadline = Some(Utc::now() - Duration::hours(1));

        assert_eq!(policy.score(&task, &context), SCORE_OVERDUE);
    }

    #[test]
    fn test_critical_deadline() {
        let policy = DeadlinePolicy::new();
        let context = SchedulingContext::default();

        let mut task = Task::new("Critical".to_string());
        task.deadline = Some(Utc::now() + Duration::hours(2));

        assert_eq!(policy.score(&task, &context), SCORE_CRITICAL);
    }

    #[test]
    fn test_no_deadline() {
        let policy = DeadlinePolicy::new();
        let context = SchedulingContext::default();

        let task = Task::new("No deadline".to_string());

        assert_eq!(policy.score(&task, &context), SCORE_NO_DEADLINE);
    }

    #[test]
    fn test_far_future_deadline() {
        let policy = DeadlinePolicy::new();
        let context = SchedulingContext::default();

        let mut task = Task::new("Far future".to_string());
        task.deadline = Some(Utc::now() + Duration::days(30));

        assert_eq!(policy.score(&task, &context), SCORE_FAR_FUTURE);
    }
}
