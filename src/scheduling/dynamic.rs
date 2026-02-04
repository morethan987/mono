use crate::models::Task;
use crate::scheduling::policy::{SchedulingContext, ScoredTask};
use crate::scheduling::SchedulingEngine;

pub struct DynamicScheduler {
    engine: SchedulingEngine,
}

impl DynamicScheduler {
    pub fn new(engine: SchedulingEngine) -> Self {
        Self { engine }
    }

    pub fn on_task_completed(
        &self,
        _completed_task: &Task,
        available_tasks: Vec<Task>,
    ) -> Option<ScoredTask> {
        self.engine.get_next_task(available_tasks)
    }

    pub fn on_task_interrupted(
        &self,
        interrupted_task: &Task,
        available_tasks: Vec<Task>,
    ) -> Vec<ScoredTask> {
        let filtered: Vec<Task> = available_tasks
            .into_iter()
            .filter(|t| t.id != interrupted_task.id)
            .collect();

        let context = SchedulingContext::new();
        self.engine.rank_tasks(filtered, &context)
    }

    pub fn on_task_added(&self, _new_task: &Task, all_tasks: Vec<Task>) -> Vec<ScoredTask> {
        let context = SchedulingContext::new();
        self.engine.rank_tasks(all_tasks, &context)
    }

    pub fn recommend_next(&self, available_tasks: Vec<Task>) -> Option<ScoredTask> {
        self.engine.get_next_task(available_tasks)
    }
}

impl Default for DynamicScheduler {
    fn default() -> Self {
        Self::new(SchedulingEngine::new())
    }
}
