//! Scheduling policy trait definitions.

use crate::models::Task;

/// Context for scheduling decisions.
#[derive(Debug, Clone)]
pub struct SchedulingContext {
    /// Current timestamp
    pub now: chrono::DateTime<chrono::Utc>,
    /// User's preferred work hours start (0-23)
    pub work_hours_start: u32,
    /// User's preferred work hours end (0-23)
    pub work_hours_end: u32,
    /// Duration of current work session in minutes
    pub current_session_duration: u32,
    /// Number of interruptions in current session
    pub session_interruptions: u32,
}

impl Default for SchedulingContext {
    fn default() -> Self {
        Self {
            now: chrono::Utc::now(),
            work_hours_start: 9,
            work_hours_end: 18,
            current_session_duration: 0,
            session_interruptions: 0,
        }
    }
}

impl SchedulingContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_now(mut self, now: chrono::DateTime<chrono::Utc>) -> Self {
        self.now = now;
        self
    }
}

/// A scored task with its scheduling score.
#[derive(Debug, Clone)]
pub struct ScoredTask {
    pub task: Task,
    pub score: f64,
    /// Breakdown of score components for debugging
    pub score_breakdown: Vec<(String, f64)>,
}

impl ScoredTask {
    pub fn new(task: Task, score: f64) -> Self {
        Self {
            task,
            score,
            score_breakdown: Vec::new(),
        }
    }

    pub fn with_breakdown(mut self, breakdown: Vec<(String, f64)>) -> Self {
        self.score_breakdown = breakdown;
        self
    }
}

/// Trait for scheduling policies.
///
/// Each policy implements a specific scheduling strategy (e.g., priority-based,
/// deadline-based) and assigns scores to tasks. Higher scores indicate higher
/// urgency to work on a task.
pub trait SchedulingPolicy: Send + Sync {
    /// Returns the name of this policy for logging/debugging.
    fn name(&self) -> &'static str;

    /// Returns the weight of this policy when combining with others.
    /// Default is 1.0. Policies with higher weights have more influence.
    fn weight(&self) -> f64 {
        1.0
    }

    /// Scores a single task based on this policy.
    ///
    /// Returns a score between 0.0 and 1.0, where:
    /// - 1.0 = highest urgency (should do immediately)
    /// - 0.0 = lowest urgency (can wait indefinitely)
    fn score(&self, task: &Task, context: &SchedulingContext) -> f64;

    /// Scores multiple tasks and returns them sorted by score (descending).
    fn rank(&self, tasks: Vec<Task>, context: &SchedulingContext) -> Vec<ScoredTask> {
        let mut scored: Vec<ScoredTask> = tasks
            .into_iter()
            .map(|task| {
                let score = self.score(&task, context);
                ScoredTask::new(task, score)
            })
            .collect();

        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        scored
    }
}
