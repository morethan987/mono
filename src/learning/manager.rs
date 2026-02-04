use crate::learning::{
    bandit::{TimeSlotArm, TimeSlotBandit},
    duration::{BayesianDurationPredictor, DurationStats},
    features::FeatureExtractor,
    ftrl::FtrlModel,
    reward::{feedback_to_label, is_success},
};
use crate::models::{Feedback, Task, TaskType};
use chrono::{DateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTypeLearningModel {
    pub task_type: TaskType,
    pub time_slot_bandit: TimeSlotBandit,
    pub ftrl_model: FtrlModel,
    pub duration_stats: DurationStats,
    pub total_scheduled: u32,
    pub total_completed: u32,
    pub total_postponed: u32,
    pub total_skipped: u32,
    pub total_interrupted: u32,
    pub avg_duration_minutes: Option<f64>,
}

impl TaskTypeLearningModel {
    pub fn new(task_type: TaskType) -> Self {
        Self {
            task_type,
            time_slot_bandit: TimeSlotBandit::new(),
            ftrl_model: FtrlModel::new(),
            duration_stats: DurationStats::new(),
            total_scheduled: 0,
            total_completed: 0,
            total_postponed: 0,
            total_skipped: 0,
            total_interrupted: 0,
            avg_duration_minutes: None,
        }
    }

    pub fn completion_rate(&self) -> f64 {
        if self.total_scheduled == 0 {
            0.5
        } else {
            self.total_completed as f64 / self.total_scheduled as f64
        }
    }

    pub fn update_from_feedback(
        &mut self,
        task: &Task,
        feedback: &Feedback,
        scheduled_at: DateTime<Utc>,
        feature_extractor: &FeatureExtractor,
    ) {
        self.total_scheduled += 1;

        match feedback.feedback_type {
            crate::models::FeedbackType::Completed => self.total_completed += 1,
            crate::models::FeedbackType::Postponed => self.total_postponed += 1,
            crate::models::FeedbackType::Skipped => self.total_skipped += 1,
            crate::models::FeedbackType::Interrupted | crate::models::FeedbackType::UserChoice => {}
        }

        let arm = TimeSlotArm::from_hour(scheduled_at.hour());
        self.time_slot_bandit.update(arm, is_success(feedback));

        let features = feature_extractor.extract(task, scheduled_at);
        let label = feedback_to_label(feedback);
        self.ftrl_model.update(&features, label);

        if let Some(actual) = feedback.actual_duration_minutes {
            self.duration_stats.update(actual);
            self.avg_duration_minutes = Some(match self.avg_duration_minutes {
                Some(avg) => avg * 0.9 + actual as f64 * 0.1,
                None => actual as f64,
            });
        }
    }

    pub fn predict_duration(&self) -> (f64, f64) {
        BayesianDurationPredictor::predict(&self.duration_stats)
    }

    /// Apply time-based decay to this model
    pub fn apply_time_decay(&mut self, factor: f64) {
        self.total_scheduled = (self.total_scheduled as f64 * factor).round() as u32;
        self.total_completed = (self.total_completed as f64 * factor).round() as u32;
        self.total_postponed = (self.total_postponed as f64 * factor).round() as u32;
        self.total_skipped = (self.total_skipped as f64 * factor).round() as u32;

        self.time_slot_bandit.apply_decay(factor);
        self.duration_stats.apply_decay(factor);
    }

    pub fn best_time_slot(&self) -> TimeSlotArm {
        if self.total_scheduled < 10 {
            self.time_slot_bandit.select_arm()
        } else {
            self.time_slot_bandit.select_arm_greedy()
        }
    }

    pub fn ftrl_weights_count(&self) -> usize {
        self.ftrl_model.weights_count()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalLearningModel {
    pub ftrl_model: FtrlModel,
    pub time_slot_bandit: TimeSlotBandit,
    pub total_tasks: u32,
}

impl Default for GlobalLearningModel {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalLearningModel {
    pub fn new() -> Self {
        Self {
            ftrl_model: FtrlModel::new(),
            time_slot_bandit: TimeSlotBandit::new(),
            total_tasks: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningManagerState {
    pub global_model: GlobalLearningModel,
    pub models: HashMap<String, TaskTypeLearningModel>,
    pub version: u32,
}

impl Default for LearningManagerState {
    fn default() -> Self {
        Self {
            global_model: GlobalLearningModel::new(),
            models: HashMap::new(),
            version: 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LearningManager {
    models: HashMap<String, TaskTypeLearningModel>,
    global_model: GlobalLearningModel,
    feature_extractor: FeatureExtractor,
    cold_start_threshold: u32,
}

impl Default for LearningManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LearningManager {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            global_model: GlobalLearningModel::new(),
            feature_extractor: FeatureExtractor::new(),
            cold_start_threshold: 10,
        }
    }

    pub fn from_state(state: LearningManagerState) -> Self {
        Self {
            models: state.models,
            global_model: state.global_model,
            feature_extractor: FeatureExtractor::new(),
            cold_start_threshold: 10,
        }
    }

    pub fn to_state(&self) -> LearningManagerState {
        LearningManagerState {
            global_model: self.global_model.clone(),
            models: self.models.clone(),
            version: 1,
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.to_state())
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let state: LearningManagerState = serde_json::from_str(json)?;
        Ok(Self::from_state(state))
    }

    pub fn predict_success(&self, task: &Task, scheduled_at: DateTime<Utc>) -> f64 {
        let features = self.feature_extractor.extract(task, scheduled_at);
        let task_type = task.task_type();

        // Sqrt scaling: confidence reaches 0.5 at 2.5 samples, 1.0 at threshold (10)
        let (type_pred, confidence) = if let Some(model) = self.models.get(&task_type.name) {
            let pred = model.ftrl_model.predict(&features);
            let n = model.total_scheduled as f64;
            let conf = (n / self.cold_start_threshold as f64).sqrt().min(1.0);
            (pred, conf)
        } else {
            (0.5, 0.0)
        };

        let global_pred = self.global_model.ftrl_model.predict(&features);

        type_pred * confidence + global_pred * (1.0 - confidence)
    }

    pub fn suggest_time_slot(&self, task: &Task) -> TimeSlotArm {
        let task_type = task.task_type();

        if let Some(model) = self.models.get(&task_type.name)
            && model.total_scheduled >= self.cold_start_threshold
        {
            return model.best_time_slot();
        }

        self.global_model.time_slot_bandit.select_arm()
    }

    pub fn update_from_feedback(&mut self, task: &Task, feedback: &Feedback) {
        let scheduled_at = task.scheduled_at.unwrap_or_else(Utc::now);
        let task_type = task.task_type();

        let model = self
            .models
            .entry(task_type.name.clone())
            .or_insert_with(|| TaskTypeLearningModel::new(task_type.clone()));

        model.update_from_feedback(task, feedback, scheduled_at, &self.feature_extractor);

        let features = self.feature_extractor.extract(task, scheduled_at);
        let label = feedback_to_label(feedback);
        self.global_model.ftrl_model.update(&features, label);

        let arm = TimeSlotArm::from_hour(scheduled_at.hour());
        self.global_model
            .time_slot_bandit
            .update(arm, is_success(feedback));
        self.global_model.total_tasks += 1;
    }

    pub fn record_interruption(&mut self, task: &Task) {
        let task_type = task.task_type();
        let model = self
            .models
            .entry(task_type.name.clone())
            .or_insert_with(|| TaskTypeLearningModel::new(task_type.clone()));

        model.total_interrupted += 1;

        // Also update bandit as a failure for this time slot
        let now = Utc::now();
        let arm = TimeSlotArm::from_hour(now.hour());
        model.time_slot_bandit.update(arm, false);
        self.global_model.time_slot_bandit.update(arm, false);
    }

    pub fn get_model(&self, task_type: &str) -> Option<&TaskTypeLearningModel> {
        self.models.get(task_type)
    }

    pub fn all_models(&self) -> impl Iterator<Item = (&String, &TaskTypeLearningModel)> {
        self.models.iter()
    }

    pub fn global_stats(&self) -> &GlobalLearningModel {
        &self.global_model
    }

    pub fn reset(&mut self, task_type: Option<&str>) {
        match task_type {
            Some(tt) => {
                self.models.remove(tt);
            }
            None => {
                self.models.clear();
                self.global_model = GlobalLearningModel::new();
            }
        }
    }

    pub fn set_time_slot_preference(&mut self, task_type: &str, arm: TimeSlotArm, strength: u32) {
        let task_type_obj = TaskType::new(task_type);
        let model = self
            .models
            .entry(task_type.to_string())
            .or_insert_with(|| TaskTypeLearningModel::new(task_type_obj));

        let strength = strength.min(10) as f64;
        for _ in 0..strength as u32 {
            model.time_slot_bandit.update(arm, true);
            model.total_scheduled += 1;
            model.total_completed += 1;
        }

        for _ in 0..strength as u32 {
            self.global_model.time_slot_bandit.update(arm, true);
            self.global_model.total_tasks += 1;
        }
    }

    pub fn import_state(&mut self, state: LearningManagerState, merge: bool) {
        if merge {
            for (key, model) in state.models {
                self.models.insert(key, model);
            }
            self.global_model.total_tasks += state.global_model.total_tasks;
        } else {
            self.models = state.models;
            self.global_model = state.global_model;
        }
    }

    pub fn get_global_ftrl_weights_count(&self) -> usize {
        self.global_model.ftrl_model.weights_count()
    }

    /// Apply time-based decay to all models
    pub fn apply_time_decay(&mut self, factor: f64) {
        self.global_model.time_slot_bandit.apply_decay(factor);
        self.global_model.total_tasks =
            (self.global_model.total_tasks as f64 * factor).round() as u32;

        for model in self.models.values_mut() {
            model.apply_time_decay(factor);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Priority;

    fn create_test_task(name: &str, tag: &str) -> Task {
        Task::new(name.to_string())
            .with_tags(vec![tag.to_string()])
            .with_priority(Priority::Medium)
    }

    #[test]
    fn test_task_type_model_creation() {
        let task_type = TaskType::new("work");
        let model = TaskTypeLearningModel::new(task_type);

        assert_eq!(model.total_scheduled, 0);
        assert_eq!(model.completion_rate(), 0.5);
    }

    #[test]
    fn test_learning_manager_cold_start() {
        let manager = LearningManager::new();
        let task = create_test_task("Test task", "work");
        let now = Utc::now();

        let prediction = manager.predict_success(&task, now);
        assert!((prediction - 0.5).abs() < 0.1);
    }

    #[test]
    fn test_learning_from_feedback() {
        let mut manager = LearningManager::new();
        let task = create_test_task("Test task", "work");

        for i in 0..20 {
            let feedback = if i % 3 == 0 {
                Feedback::postponed(task.id.clone(), 30)
            } else {
                Feedback::completed(task.id.clone(), 25)
            };
            manager.update_from_feedback(&task, &feedback);
        }

        let model = manager.get_model("work").unwrap();
        assert_eq!(model.total_scheduled, 20);
        assert!(model.total_completed > model.total_postponed);
    }

    #[test]
    fn test_blended_prediction() {
        let mut manager = LearningManager::new();

        let work_task = create_test_task("Work task", "work");
        for _ in 0..15 {
            let feedback = Feedback::completed(work_task.id.clone(), 30);
            manager.update_from_feedback(&work_task, &feedback);
        }

        let new_task = create_test_task("New work", "work");
        let prediction = manager.predict_success(&new_task, Utc::now());

        assert!(
            prediction > 0.5,
            "Should predict higher after positive feedback"
        );
    }
}
