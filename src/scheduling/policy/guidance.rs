use crate::models::Task;
use crate::scheduling::policy::{SchedulingContext, SchedulingPolicy};

/// Policy that nudges tasks that have been repeatedly postponed
pub struct GuidancePolicy;

impl GuidancePolicy {
    pub fn new() -> Self {
        Self
    }
}

impl SchedulingPolicy for GuidancePolicy {
    fn name(&self) -> &'static str {
        "guidance"
    }

    fn score(&self, task: &Task, _context: &SchedulingContext) -> f64 {
        // Nudge tasks that have been around for a long time or have dependencies
        // (Simple version for now: boost slightly based on age since creation)
        let now = chrono::Utc::now();
        let age_days = (now - task.created_at).num_days().max(0) as f64;

        // Gentle boost: reaches 0.1 at 7 days
        let boost = (age_days / 70.0).min(0.2);

        0.5 + boost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Priority;
    use chrono::{Duration, Utc};

    #[test]
    fn test_guidance_policy_age_boost() {
        let policy = GuidancePolicy::new();
        let context = SchedulingContext::default();

        let mut old_task = Task::new("Old".to_string());
        old_task.created_at = Utc::now() - Duration::days(10);

        let new_task = Task::new("New".to_string());

        let old_score = policy.score(&old_task, &context);
        let new_score = policy.score(&new_task, &context);

        assert!(old_score > new_score);
    }
}
