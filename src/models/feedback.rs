use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeedbackType {
    Completed,
    Postponed,
    Skipped,
    Interrupted,
    UserChoice,
}

impl FeedbackType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FeedbackType::Completed => "completed",
            FeedbackType::Postponed => "postponed",
            FeedbackType::Skipped => "skipped",
            FeedbackType::Interrupted => "interrupted",
            FeedbackType::UserChoice => "user_choice",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "completed" => Some(FeedbackType::Completed),
            "postponed" => Some(FeedbackType::Postponed),
            "skipped" => Some(FeedbackType::Skipped),
            "interrupted" => Some(FeedbackType::Interrupted),
            "user_choice" => Some(FeedbackType::UserChoice),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback {
    pub id: String,
    pub task_id: String,
    pub schedule_id: Option<String>,
    pub feedback_type: FeedbackType,
    pub rating: Option<u8>,
    pub actual_duration_minutes: Option<u32>,
    pub difficulty_rating: Option<u8>,
    pub energy_level: Option<u8>,
    pub notes: Option<String>,
    pub postpone_minutes: Option<u32>,
    pub created_at: DateTime<Utc>,
}

impl Feedback {
    pub fn new(task_id: String, feedback_type: FeedbackType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            task_id,
            schedule_id: None,
            feedback_type,
            rating: None,
            actual_duration_minutes: None,
            difficulty_rating: None,
            energy_level: None,
            notes: None,
            postpone_minutes: None,
            created_at: Utc::now(),
        }
    }

    pub fn completed(task_id: String, actual_minutes: u32) -> Self {
        let mut feedback = Self::new(task_id, FeedbackType::Completed);
        feedback.actual_duration_minutes = Some(actual_minutes);
        feedback
    }

    pub fn postponed(task_id: String, minutes: u32) -> Self {
        let mut feedback = Self::new(task_id, FeedbackType::Postponed);
        feedback.postpone_minutes = Some(minutes);
        feedback
    }

    pub fn skipped(task_id: String) -> Self {
        Self::new(task_id, FeedbackType::Skipped)
    }

    pub fn interrupted(task_id: String) -> Self {
        Self::new(task_id, FeedbackType::Interrupted)
    }

    pub fn with_rating(mut self, rating: u8) -> Self {
        self.rating = Some(rating.min(5));
        self
    }

    pub fn with_notes(mut self, notes: String) -> Self {
        self.notes = Some(notes);
        self
    }

    pub fn with_difficulty(mut self, difficulty: u8) -> Self {
        self.difficulty_rating = Some(difficulty.min(5));
        self
    }

    pub fn with_energy_level(mut self, energy: u8) -> Self {
        self.energy_level = Some(energy.min(5));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feedback_creation() {
        let feedback = Feedback::completed("task-1".to_string(), 30);
        assert_eq!(feedback.feedback_type, FeedbackType::Completed);
        assert_eq!(feedback.actual_duration_minutes, Some(30));
    }

    #[test]
    fn test_feedback_postponed() {
        let feedback = Feedback::postponed("task-1".to_string(), 15);
        assert_eq!(feedback.feedback_type, FeedbackType::Postponed);
        assert_eq!(feedback.postpone_minutes, Some(15));
    }
}
