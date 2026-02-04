use crate::models::{Feedback, FeedbackType};

pub fn feedback_to_label(feedback: &Feedback) -> f64 {
    match feedback.feedback_type {
        FeedbackType::Completed => 1.0,
        FeedbackType::Postponed => 0.3,
        FeedbackType::Skipped => 0.0,
        FeedbackType::Interrupted => 0.2,
        FeedbackType::UserChoice => 0.5,
    }
}

pub fn is_success(feedback: &Feedback) -> bool {
    matches!(feedback.feedback_type, FeedbackType::Completed)
}
