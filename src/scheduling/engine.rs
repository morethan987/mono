use crate::models::Task;
use crate::scheduling::policy::{
    DeadlinePolicy, PriorityPolicy, SchedulingContext, SchedulingPolicy, ScoredTask,
};
use crate::scheduling::queue::TaskQueue;

pub struct SchedulingEngine {
    policies: Vec<Box<dyn SchedulingPolicy>>,
}

impl SchedulingEngine {
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
        }
    }

    pub fn with_default_policies() -> Self {
        let mut engine = Self::new();
        engine.add_policy(Box::new(PriorityPolicy::new()));
        engine.add_policy(Box::new(DeadlinePolicy::new()));
        engine
    }

    pub fn add_policy(&mut self, policy: Box<dyn SchedulingPolicy>) {
        self.policies.push(policy);
    }

    pub fn score_task(&self, task: &Task, context: &SchedulingContext) -> ScoredTask {
        if self.policies.is_empty() {
            return ScoredTask::new(task.clone(), 0.5);
        }

        let mut total_score = 0.0;
        let mut total_weight = 0.0;
        let mut breakdown = Vec::new();

        for policy in &self.policies {
            let score = policy.score(task, context);
            let weight = policy.weight();
            let weighted_score = score * weight;

            breakdown.push((policy.name().to_string(), score));

            total_score += weighted_score;
            total_weight += weight;
        }

        let final_score = if total_weight > 0.0 {
            total_score / total_weight
        } else {
            0.5
        };

        ScoredTask::new(task.clone(), final_score).with_breakdown(breakdown)
    }

    pub fn rank_tasks(&self, tasks: Vec<Task>, context: &SchedulingContext) -> Vec<ScoredTask> {
        let mut scored: Vec<ScoredTask> = tasks
            .into_iter()
            .map(|task| self.score_task(&task, context))
            .collect();

        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        scored
    }

    pub fn get_next_task(&self, tasks: Vec<Task>) -> Option<ScoredTask> {
        let context = SchedulingContext::new();
        self.rank_tasks(tasks, &context).into_iter().next()
    }

    pub fn build_queue(&self, tasks: Vec<Task>, context: &SchedulingContext) -> TaskQueue {
        let scored = self.rank_tasks(tasks, context);
        scored.into_iter().collect()
    }
}

impl Default for SchedulingEngine {
    fn default() -> Self {
        Self::with_default_policies()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Priority;
    use chrono::{Duration, Utc};

    #[test]
    fn test_combined_scoring() {
        let engine = SchedulingEngine::with_default_policies();
        let context = SchedulingContext::default();

        let urgent_no_deadline =
            Task::new("Urgent no deadline".to_string()).with_priority(Priority::Urgent);

        let low_overdue = {
            let mut t = Task::new("Low but overdue".to_string()).with_priority(Priority::Low);
            t.deadline = Some(Utc::now() - Duration::hours(1));
            t
        };

        let score_urgent = engine.score_task(&urgent_no_deadline, &context);
        let score_overdue = engine.score_task(&low_overdue, &context);

        assert!(
            score_overdue.score > score_urgent.score,
            "Overdue task ({:.2}) should beat urgent no-deadline ({:.2})",
            score_overdue.score,
            score_urgent.score
        );
    }

    #[test]
    fn test_ranking() {
        let engine = SchedulingEngine::with_default_policies();
        let context = SchedulingContext::default();

        let tasks = vec![
            Task::new("Low priority".to_string()).with_priority(Priority::Low),
            Task::new("Urgent".to_string()).with_priority(Priority::Urgent),
            {
                let mut t =
                    Task::new("Medium with deadline".to_string()).with_priority(Priority::Medium);
                t.deadline = Some(Utc::now() + Duration::hours(2));
                t
            },
        ];

        let ranked = engine.rank_tasks(tasks, &context);

        assert_eq!(ranked[0].task.title, "Medium with deadline");
        assert_eq!(ranked[1].task.title, "Urgent");
        assert_eq!(ranked[2].task.title, "Low priority");
    }

    #[test]
    fn test_get_next_task() {
        let engine = SchedulingEngine::with_default_policies();

        let tasks = vec![
            Task::new("Low".to_string()).with_priority(Priority::Low),
            Task::new("High".to_string()).with_priority(Priority::High),
        ];

        let next = engine.get_next_task(tasks).unwrap();
        assert_eq!(next.task.title, "High");
    }

    #[test]
    fn test_empty_tasks() {
        let engine = SchedulingEngine::with_default_policies();
        let next = engine.get_next_task(vec![]);
        assert!(next.is_none());
    }
}
