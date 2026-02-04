//! Inference engine for automatic priority and duration suggestions
//!
//! Provides intelligent defaults based on historical data and title analysis

use crate::models::{Priority, TaskType};
use crate::storage::repository::TaskTypeStats;

/// Engine for inferring task properties based on context and history
pub struct InferenceEngine;

impl InferenceEngine {
    pub fn new() -> Self {
        Self
    }

    /// Infer priority from task type history and title keywords
    pub fn infer_priority(
        &self,
        _task_type: &TaskType,
        title: &str,
        _type_stats: Option<&TaskTypeStats>,
    ) -> Priority {
        let title_lower = title.to_lowercase();

        if title_lower.contains("urgent")
            || title_lower.contains("asap")
            || title_lower.contains("immediately")
            || title_lower.contains("紧急")
        {
            return Priority::Urgent;
        }

        if title_lower.contains("important")
            || title_lower.contains("critical")
            || title_lower.contains("重要")
        {
            return Priority::High;
        }

        if title_lower.contains("maybe")
            || title_lower.contains("someday")
            || title_lower.contains("optional")
            || title_lower.contains("可能")
        {
            return Priority::Low;
        }

        Priority::Medium
    }

    /// Infer duration from task type historical average
    pub fn infer_duration(&self, _task_type: &TaskType, type_stats: Option<&TaskTypeStats>) -> u32 {
        if let Some(stats) = type_stats {
            if let Some(avg) = stats.avg_duration_minutes {
                let duration: u32 = (avg as f64).round() as u32;
                return duration.max(5).min(480);
            }
        }

        30
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_priority_from_title() {
        let engine = InferenceEngine::new();
        let task_type = TaskType::from_tags(&["work".to_string()]);

        assert_eq!(
            engine.infer_priority(&task_type, "Urgent fix needed", None),
            Priority::Urgent
        );

        assert_eq!(
            engine.infer_priority(&task_type, "This is important", None),
            Priority::High
        );

        assert_eq!(
            engine.infer_priority(&task_type, "Maybe look at this someday", None),
            Priority::Low
        );

        assert_eq!(
            engine.infer_priority(&task_type, "Regular task", None),
            Priority::Medium
        );
    }

    #[test]
    fn test_infer_duration_default() {
        let engine = InferenceEngine::new();
        let task_type = TaskType::from_tags(&[]);

        assert_eq!(engine.infer_duration(&task_type, None), 30);
    }
}
