use crate::learning::LearningManager;
use crate::models::Task;
use crate::scheduling::policy::{SchedulingContext, SchedulingPolicy};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct LearningPolicy {
    manager: Arc<RwLock<LearningManager>>,
    weight: f64,
}

impl LearningPolicy {
    pub fn new(manager: Arc<RwLock<LearningManager>>) -> Self {
        Self {
            manager,
            weight: 1.0,
        }
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}

impl SchedulingPolicy for LearningPolicy {
    fn name(&self) -> &'static str {
        "learning"
    }

    fn weight(&self) -> f64 {
        self.weight
    }

    fn score(&self, task: &Task, context: &SchedulingContext) -> f64 {
        let manager = match self.manager.try_read() {
            Ok(guard) => guard,
            Err(_) => return 0.5,
        };

        manager.predict_success(task, context.now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Priority;

    #[tokio::test]
    async fn test_learning_policy_default() {
        let manager = Arc::new(RwLock::new(LearningManager::new()));
        let policy = LearningPolicy::new(manager);

        let task = Task::new("Test".to_string()).with_priority(Priority::Medium);
        let context = SchedulingContext::default();

        let score = policy.score(&task, &context);
        assert!((score - 0.5).abs() < 0.1);
    }
}
