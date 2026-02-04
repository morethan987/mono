//! Interruption tracking models
//!
//! Defines types and structures for tracking task interruptions,
//! including automatic detection from context changes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Types of interruptions that can occur during task execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InterruptionType {
    /// Emergency insertion - urgent task that must be handled immediately
    Emergency,
    /// Distraction - loss of focus (social media, unrelated browsing, etc.)
    Distraction,
    /// Rest - intentional break for recovery
    Rest,
    /// External - external interruption (meeting, message, phone call, etc.)
    External,
}

impl Default for InterruptionType {
    fn default() -> Self {
        InterruptionType::External
    }
}

/// Records an interruption event during task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interruption {
    /// Unique identifier for this interruption
    pub id: String,
    /// ID of the task that was interrupted
    pub task_id: String,
    /// Type of interruption
    pub interruption_type: InterruptionType,
    /// When the interruption started
    pub started_at: DateTime<Utc>,
    /// When the interruption ended (None if still ongoing)
    pub ended_at: Option<DateTime<Utc>>,
    /// Optional user-provided reason or notes
    pub reason: Option<String>,
}

impl Interruption {
    /// Create a new interruption
    pub fn new(id: String, task_id: String, interruption_type: InterruptionType) -> Self {
        Self {
            id,
            task_id,
            interruption_type,
            started_at: Utc::now(),
            ended_at: None,
            reason: None,
        }
    }

    /// Mark the interruption as ended
    pub fn end(&mut self) {
        self.ended_at = Some(Utc::now());
    }

    /// Get the duration of the interruption (if ended)
    pub fn duration(&self) -> Option<chrono::Duration> {
        self.ended_at
            .map(|end| end.signed_duration_since(self.started_at))
    }
}
