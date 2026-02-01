use crate::models::{Feedback, FeedbackType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardSignal {
    pub value: f64,
    pub is_positive: bool,
    pub confidence: f64,
}

impl RewardSignal {
    pub fn new(value: f64) -> Self {
        Self {
            value,
            is_positive: value >= 0.5,
            confidence: 1.0,
        }
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
}

pub fn compute_reward(feedback: &Feedback, estimated_minutes: Option<u32>) -> RewardSignal {
    let base_reward = match feedback.feedback_type {
        FeedbackType::Completed => compute_completion_reward(feedback, estimated_minutes),
        FeedbackType::Postponed => 0.2,
        FeedbackType::Skipped => 0.0,
        FeedbackType::Interrupted => 0.3,
    };

    let rating_bonus = feedback
        .rating
        .map(|r| (r as f64 - 3.0) * 0.1)
        .unwrap_or(0.0);

    let final_reward = (base_reward + rating_bonus).clamp(0.0, 1.0);

    let confidence = match feedback.feedback_type {
        FeedbackType::Completed => 1.0,
        FeedbackType::Postponed => 0.7,
        FeedbackType::Skipped => 0.5,
        FeedbackType::Interrupted => 0.6,
    };

    RewardSignal::new(final_reward).with_confidence(confidence)
}

fn compute_completion_reward(feedback: &Feedback, estimated_minutes: Option<u32>) -> f64 {
    let base = 1.0;

    let duration_factor = match (feedback.actual_duration_minutes, estimated_minutes) {
        (Some(actual), Some(estimated)) if estimated > 0 => {
            let ratio = actual as f64 / estimated as f64;
            if ratio <= 1.0 {
                1.0
            } else if ratio <= 1.5 {
                0.9
            } else if ratio <= 2.0 {
                0.7
            } else {
                0.5
            }
        }
        _ => 1.0,
    };

    base * duration_factor
}

pub fn feedback_to_label(feedback: &Feedback) -> f64 {
    match feedback.feedback_type {
        FeedbackType::Completed => 1.0,
        FeedbackType::Postponed => 0.3,
        FeedbackType::Skipped => 0.0,
        FeedbackType::Interrupted => 0.2,
    }
}

pub fn is_success(feedback: &Feedback) -> bool {
    matches!(feedback.feedback_type, FeedbackType::Completed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_reward() {
        let feedback = Feedback::completed("task-1".to_string(), 30);
        let reward = compute_reward(&feedback, Some(30));

        assert!((reward.value - 1.0).abs() < 0.01);
        assert!(reward.is_positive);
        assert_eq!(reward.confidence, 1.0);
    }

    #[test]
    fn test_completion_overtime_penalty() {
        let feedback = Feedback::completed("task-1".to_string(), 60);
        let reward = compute_reward(&feedback, Some(30));

        assert!(reward.value < 1.0);
        assert!(reward.value >= 0.5);
    }

    #[test]
    fn test_postponed_reward() {
        let feedback = Feedback::postponed("task-1".to_string(), 30);
        let reward = compute_reward(&feedback, None);

        assert!((reward.value - 0.2).abs() < 0.01);
        assert!(!reward.is_positive);
    }

    #[test]
    fn test_skipped_reward() {
        let feedback = Feedback::skipped("task-1".to_string());
        let reward = compute_reward(&feedback, None);

        assert_eq!(reward.value, 0.0);
        assert!(!reward.is_positive);
    }

    #[test]
    fn test_rating_bonus() {
        let feedback = Feedback::completed("task-1".to_string(), 30).with_rating(5);
        let reward = compute_reward(&feedback, Some(30));

        assert!(reward.value > 1.0 - 0.01);

        let feedback_low = Feedback::completed("task-2".to_string(), 30).with_rating(1);
        let reward_low = compute_reward(&feedback_low, Some(30));

        assert!(reward_low.value < reward.value);
    }
}
