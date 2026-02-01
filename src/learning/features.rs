//! Feature engineering for machine learning models.
//!
//! Extracts relevant features from tasks, time context, and user preferences
//! for use in the FTRL prediction model.

use crate::models::{Priority, Task, TaskType};
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A sparse feature vector for ML models.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeatureVector {
    /// Sparse representation: feature_index -> value
    pub features: HashMap<u32, f64>,
}

impl FeatureVector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a feature with given index and value.
    pub fn add(&mut self, index: u32, value: f64) {
        if value.abs() > 1e-10 {
            self.features.insert(index, value);
        }
    }

    /// Add a binary feature (1.0) at the given index.
    pub fn add_binary(&mut self, index: u32) {
        self.features.insert(index, 1.0);
    }

    /// Get feature value, defaulting to 0.0.
    pub fn get(&self, index: u32) -> f64 {
        *self.features.get(&index).unwrap_or(&0.0)
    }

    /// Iterate over non-zero features.
    pub fn iter(&self) -> impl Iterator<Item = (&u32, &f64)> {
        self.features.iter()
    }

    /// Number of non-zero features.
    pub fn len(&self) -> usize {
        self.features.len()
    }

    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }
}

/// Feature index ranges (for organized feature namespace).
mod feature_index {
    // Hour of day: 0-23 (indices 0-23)
    pub const HOUR_BASE: u32 = 0;

    // Day of week: 0-6 (indices 24-30)
    pub const DAY_OF_WEEK_BASE: u32 = 24;

    // Priority: 0-3 (indices 31-34)
    pub const PRIORITY_BASE: u32 = 31;

    // Has deadline: index 35
    pub const HAS_DEADLINE: u32 = 35;

    // Deadline proximity buckets (indices 36-40)
    pub const DEADLINE_PROXIMITY_BASE: u32 = 36;

    // Duration buckets (indices 41-45)
    pub const DURATION_BASE: u32 = 41;

    // Task type hash base (indices 100-199)
    pub const TASK_TYPE_BASE: u32 = 100;

    // Time of day: morning/afternoon/evening/night (indices 200-203)
    pub const TIME_OF_DAY_BASE: u32 = 200;

    // Is weekend: index 204
    pub const IS_WEEKEND: u32 = 204;

    // Reserved for future features
    pub const _RESERVED_BASE: u32 = 300;
}

/// Feature extractor for tasks.
#[derive(Debug, Clone, Default)]
pub struct FeatureExtractor {
    /// Number of task type hash buckets
    task_type_buckets: u32,
}

impl FeatureExtractor {
    pub fn new() -> Self {
        Self {
            task_type_buckets: 100, // 100 buckets for task types
        }
    }

    /// Extract features from a task and scheduling context.
    pub fn extract(&self, task: &Task, now: DateTime<Utc>) -> FeatureVector {
        let mut features = FeatureVector::new();

        // Hour of day (one-hot: 24 features)
        let hour = now.hour();
        features.add_binary(feature_index::HOUR_BASE + hour);

        // Day of week (one-hot: 7 features)
        let weekday = now.weekday().num_days_from_monday();
        features.add_binary(feature_index::DAY_OF_WEEK_BASE + weekday);

        // Is weekend
        if weekday >= 5 {
            features.add_binary(feature_index::IS_WEEKEND);
        }

        // Time of day bucket (morning/afternoon/evening/night)
        let time_of_day_bucket = match hour {
            6..=11 => 0,  // morning
            12..=17 => 1, // afternoon
            18..=21 => 2, // evening
            _ => 3,       // night
        };
        features.add_binary(feature_index::TIME_OF_DAY_BASE + time_of_day_bucket);

        // Priority (one-hot: 4 features)
        let priority_idx = match task.priority {
            Priority::Low => 0,
            Priority::Medium => 1,
            Priority::High => 2,
            Priority::Urgent => 3,
        };
        features.add_binary(feature_index::PRIORITY_BASE + priority_idx);

        // Has deadline
        if task.deadline.is_some() {
            features.add_binary(feature_index::HAS_DEADLINE);

            // Deadline proximity (hours until deadline -> bucket)
            if let Some(deadline) = task.deadline {
                let hours_until = (deadline - now).num_hours();
                let proximity_bucket = match hours_until {
                    h if h < 0 => 0, // overdue
                    0..=4 => 1,      // critical (< 4h)
                    5..=24 => 2,     // today
                    25..=72 => 3,    // this week
                    _ => 4,          // later
                };
                features.add_binary(feature_index::DEADLINE_PROXIMITY_BASE + proximity_bucket);
            }
        }

        // Estimated duration bucket
        if let Some(minutes) = task.estimated_minutes {
            let duration_bucket = match minutes {
                0..=15 => 0,   // quick (< 15min)
                16..=30 => 1,  // short (15-30min)
                31..=60 => 2,  // medium (30-60min)
                61..=120 => 3, // long (1-2h)
                _ => 4,        // very long (> 2h)
            };
            features.add_binary(feature_index::DURATION_BASE + duration_bucket);
        }

        // Task type (hashed to bucket)
        let task_type = task.task_type();
        let type_hash = self.hash_task_type(&task_type);
        features.add_binary(feature_index::TASK_TYPE_BASE + type_hash);

        features
    }

    /// Hash a task type name to a bucket index.
    fn hash_task_type(&self, task_type: &TaskType) -> u32 {
        // Simple FNV-1a hash
        let mut hash: u32 = 2166136261;
        for byte in task_type.name.bytes() {
            hash ^= byte as u32;
            hash = hash.wrapping_mul(16777619);
        }
        hash % self.task_type_buckets
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_vector() {
        let mut fv = FeatureVector::new();
        fv.add(0, 1.0);
        fv.add(5, 0.5);
        fv.add_binary(10);

        assert_eq!(fv.get(0), 1.0);
        assert_eq!(fv.get(5), 0.5);
        assert_eq!(fv.get(10), 1.0);
        assert_eq!(fv.get(100), 0.0); // default
        assert_eq!(fv.len(), 3);
    }

    #[test]
    fn test_feature_extraction() {
        use chrono::TimeZone;

        let task = Task::new("Test task".to_string())
            .with_priority(Priority::High)
            .with_estimated_minutes(45)
            .with_tags(vec!["work".to_string()]);

        let now = Utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap(); // Monday afternoon

        let extractor = FeatureExtractor::new();
        let features = extractor.extract(&task, now);

        // Should have hour feature (14)
        assert_eq!(features.get(feature_index::HOUR_BASE + 14), 1.0);

        // Should have day of week feature (Monday = 0)
        assert_eq!(features.get(feature_index::DAY_OF_WEEK_BASE + 0), 1.0);

        // Should have priority feature (High = 2)
        assert_eq!(features.get(feature_index::PRIORITY_BASE + 2), 1.0);

        // Should have duration bucket (31-60min = 2)
        assert_eq!(features.get(feature_index::DURATION_BASE + 2), 1.0);

        // Should have time of day (afternoon = 1)
        assert_eq!(features.get(feature_index::TIME_OF_DAY_BASE + 1), 1.0);
    }

    #[test]
    fn test_deadline_proximity() {
        use chrono::TimeZone;

        let now = Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap();
        let deadline = Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap(); // 2 hours away

        let task = Task::new("Urgent task".to_string()).with_deadline(deadline);

        let extractor = FeatureExtractor::new();
        let features = extractor.extract(&task, now);

        // Should have deadline flag
        assert_eq!(features.get(feature_index::HAS_DEADLINE), 1.0);

        // Should have critical proximity bucket (0-4h = 1)
        assert_eq!(
            features.get(feature_index::DEADLINE_PROXIMITY_BASE + 1),
            1.0
        );
    }
}
