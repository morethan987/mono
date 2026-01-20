use crate::models::{Priority, Task};
use crate::scheduling::policy::{SchedulingContext, SchedulingPolicy};

pub struct PriorityPolicy;

impl PriorityPolicy {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PriorityPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl SchedulingPolicy for PriorityPolicy {
    fn name(&self) -> &'static str {
        "priority"
    }

    fn weight(&self) -> f64 {
        1.0
    }

    fn score(&self, task: &Task, _context: &SchedulingContext) -> f64 {
        match task.priority {
            Priority::Urgent => 1.0,
            Priority::High => 0.75,
            Priority::Medium => 0.5,
            Priority::Low => 0.25,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Task;

    #[test]
    fn test_priority_scoring() {
        let policy = PriorityPolicy::new();
        let context = SchedulingContext::default();

        let urgent = Task::new("Urgent".to_string()).with_priority(Priority::Urgent);
        let high = Task::new("High".to_string()).with_priority(Priority::High);
        let medium = Task::new("Medium".to_string()).with_priority(Priority::Medium);
        let low = Task::new("Low".to_string()).with_priority(Priority::Low);

        assert_eq!(policy.score(&urgent, &context), 1.0);
        assert_eq!(policy.score(&high, &context), 0.75);
        assert_eq!(policy.score(&medium, &context), 0.5);
        assert_eq!(policy.score(&low, &context), 0.25);
    }

    #[test]
    fn test_ranking() {
        let policy = PriorityPolicy::new();
        let context = SchedulingContext::default();

        let tasks = vec![
            Task::new("Low".to_string()).with_priority(Priority::Low),
            Task::new("Urgent".to_string()).with_priority(Priority::Urgent),
            Task::new("Medium".to_string()).with_priority(Priority::Medium),
        ];

        let ranked = policy.rank(tasks, &context);

        assert_eq!(ranked[0].task.title, "Urgent");
        assert_eq!(ranked[1].task.title, "Medium");
        assert_eq!(ranked[2].task.title, "Low");
    }
}
