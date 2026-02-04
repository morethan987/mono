use crate::models::Task;
use crate::scheduling::policy::{SchedulingContext, SchedulingPolicy};

/// Policy that encourages rest based on session duration and interruptions
pub struct EnergyPolicy;

impl EnergyPolicy {
    pub fn new() -> Self {
        Self
    }
}

impl SchedulingPolicy for EnergyPolicy {
    fn name(&self) -> &'static str {
        "energy"
    }

    fn score(&self, task: &Task, context: &SchedulingContext) -> f64 {
        let is_rest = task.tags.iter().any(|t| t == "rest");

        if is_rest {
            // Boost rest tasks if session is long (> 90 mins) or has many interruptions
            let duration_bonus = (context.current_session_duration as f64 / 90.0).min(1.0) * 0.5;
            let interruption_bonus = (context.session_interruptions as f64 / 3.0).min(1.0) * 0.3;

            (0.5 + duration_bonus + interruption_bonus).min(1.0)
        } else {
            // Slightly penalize work tasks if session is very long
            if context.current_session_duration > 120 {
                0.4
            } else {
                0.5
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Priority;
    use chrono::Utc;

    #[test]
    fn test_energy_policy_rest_boost() {
        let policy = EnergyPolicy::new();
        let mut context = SchedulingContext::new();
        context.current_session_duration = 100; // > 90 mins

        let rest_task = Task::new("Nap".to_string()).with_tags(vec!["rest".to_string()]);
        let work_task = Task::new("Work".to_string()).with_tags(vec!["work".to_string()]);

        let rest_score = policy.score(&rest_task, &context);
        let work_score = policy.score(&work_task, &context);

        assert!(rest_score > work_score);
    }
}
