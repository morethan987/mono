//! Integration tests for secretary evolution features
//!
//! Tests task relationships, spawning, and parent-child completion

use chrono::Utc;
use mono::models::{Priority, Task, TaskStatus};
use mono::scheduling::InferenceEngine;
use mono::storage::repository::TaskTypeStats;

#[test]
fn test_task_spawning_relationship() {
    // Create parent task
    let parent = Task::new("Parent Task".to_string()).with_priority(Priority::High);

    // Create child task linked to parent
    let child = Task::new("Child Task".to_string()).spawned_from(parent.id.clone());

    // Verify relationship
    assert_eq!(child.spawned_from_task_id, Some(parent.id.clone()));
    assert!(child.parent_task_id.is_none()); // spawned_from is different from parent
}

#[test]
fn test_task_parent_child_relationship() {
    // Create parent task
    let parent = Task::new("Parent Task".to_string());

    // Create child with parent_task_id
    let child = Task::new("Child Task".to_string()).with_parent(parent.id.clone());

    // Verify relationship
    assert_eq!(child.parent_task_id, Some(parent.id.clone()));
}

#[test]
fn test_inference_priority_from_title() {
    let engine = InferenceEngine::new();
    let task_type = mono::models::TaskType::from_tags(&["work".to_string()]);

    // Urgent keywords
    assert_eq!(
        engine.infer_priority(&task_type, "URGENT: Fix production bug", None),
        Priority::Urgent
    );

    // High priority keywords
    assert_eq!(
        engine.infer_priority(&task_type, "Important meeting preparation", None),
        Priority::High
    );

    // Low priority keywords
    assert_eq!(
        engine.infer_priority(&task_type, "Maybe look at this someday", None),
        Priority::Low
    );

    // Default
    assert_eq!(
        engine.infer_priority(&task_type, "Regular task", None),
        Priority::Medium
    );
}

#[test]
fn test_inference_duration_with_stats() {
    let engine = InferenceEngine::new();
    let task_type = mono::models::TaskType::from_tags(&["work".to_string()]);

    // Without stats - default 30 minutes
    assert_eq!(engine.infer_duration(&task_type, None), 30);

    // With stats - use average
    let stats = TaskTypeStats {
        task_type: "work".to_string(),
        total_scheduled: 10,
        total_completed: 8,
        total_postponed: 2,
        total_skipped: 0,
        avg_completion_rate: 0.8,
        avg_duration_minutes: Some(45.5),
        best_time_slots: vec![],
        model_weights: "{}".to_string(),
    };

    let duration = engine.infer_duration(&task_type, Some(&stats));
    assert_eq!(duration, 46); // 45.5 rounded
}

#[test]
fn test_inference_duration_bounds() {
    let engine = InferenceEngine::new();
    let task_type = mono::models::TaskType::from_tags(&[]);

    // Test minimum bound (5 minutes)
    let stats_low = TaskTypeStats {
        task_type: "test".to_string(),
        total_scheduled: 1,
        total_completed: 1,
        total_postponed: 0,
        total_skipped: 0,
        avg_completion_rate: 1.0,
        avg_duration_minutes: Some(2.0), // Below minimum
        best_time_slots: vec![],
        model_weights: "{}".to_string(),
    };
    assert_eq!(engine.infer_duration(&task_type, Some(&stats_low)), 5);

    // Test maximum bound (480 minutes = 8 hours)
    let stats_high = TaskTypeStats {
        task_type: "test".to_string(),
        total_scheduled: 1,
        total_completed: 1,
        total_postponed: 0,
        total_skipped: 0,
        avg_completion_rate: 1.0,
        avg_duration_minutes: Some(600.0), // Above maximum
        best_time_slots: vec![],
        model_weights: "{}".to_string(),
    };
    assert_eq!(engine.infer_duration(&task_type, Some(&stats_high)), 480);
}

#[test]
fn test_task_status_transitions() {
    let mut task = Task::new("Test Task".to_string());

    // Initial status
    assert_eq!(task.status, TaskStatus::Pending);

    // Start task
    task.status = TaskStatus::InProgress;
    task.started_at = Some(Utc::now());
    assert_eq!(task.status, TaskStatus::InProgress);

    // Complete task
    task.status = TaskStatus::Completed;
    task.completed_at = Some(Utc::now());
    assert_eq!(task.status, TaskStatus::Completed);
}
