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

    /// Iterate over non-zero features.
    pub fn iter(&self) -> impl Iterator<Item = (&u32, &f64)> {
        self.features.iter()
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

    // Behavioral features (indices 205-208)
    // Postpone count for this task type (normalized 0.0-1.0)
    pub const POSTPONE_COUNT: u32 = 205;
    // Time since last interruption (minutes, normalized)
    pub const TIME_SINCE_INTERRUPTION: u32 = 206;
    // Current completion streak
    pub const CURRENT_STREAK: u32 = 207;
    // Tasks completed today (normalized)
    pub const DAILY_COMPLETED_COUNT: u32 = 208;

    // Reserved for future features
    pub const _RESERVED_BASE: u32 = 300;
}

/// Behavioral context for feature extraction.
#[derive(Debug, Clone, Default)]
pub struct BehavioralContext {
    /// Number of times this task type has been postponed recently
    pub postpone_count: u32,
    /// Minutes since last interruption (None if no interruptions today)
    pub minutes_since_interruption: Option<u32>,
    /// Current consecutive completion streak
    pub current_streak: u32,
    /// Number of tasks completed today
    pub daily_completed_count: u32,
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

    /// Extract features from a task and scheduling context (legacy, without behavioral data).
    pub fn extract(&self, task: &Task, now: DateTime<Utc>) -> FeatureVector {
        self.extract_with_behavior(task, now, &BehavioralContext::default())
    }

    /// Extract features from a task with behavioral context.
    pub fn extract_with_behavior(
        &self,
        task: &Task,
        now: DateTime<Utc>,
        behavior: &BehavioralContext,
    ) -> FeatureVector {
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

        // Behavioral features (normalized to 0.0-1.0 range)
        // Postpone count: normalize by capping at 10
        let postpone_normalized = (behavior.postpone_count.min(10) as f64) / 10.0;
        features.add(feature_index::POSTPONE_COUNT, postpone_normalized);

        // Time since last interruption: normalize by capping at 120 minutes (2 hours)
        let time_since_norm = behavior
            .minutes_since_interruption
            .map(|m| (m.min(120) as f64) / 120.0)
            .unwrap_or(1.0); // No interruption = maximum value
        features.add(feature_index::TIME_SINCE_INTERRUPTION, time_since_norm);

        // Current streak: normalize by capping at 10
        let streak_normalized = (behavior.current_streak.min(10) as f64) / 10.0;
        features.add(feature_index::CURRENT_STREAK, streak_normalized);

        // Daily completed count: normalize by capping at 20 tasks
        let daily_normalized = (behavior.daily_completed_count.min(20) as f64) / 20.0;
        features.add(feature_index::DAILY_COMPLETED_COUNT, daily_normalized);

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
