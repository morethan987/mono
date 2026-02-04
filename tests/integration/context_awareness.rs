//! Integration tests for context awareness and autopilot features
//!
//! Tests app classification, niri integration (basic), and dynamic scheduling

use mono::models::{Priority, Task, TaskStatus};
use mono::scheduling::context::AppClassifier;
use mono::scheduling::DynamicScheduler;
use mono::scheduling::SchedulingEngine;

#[test]
fn test_app_classification_refinement() {
    let classifier = AppClassifier::new();

    // Browsers should be study by default but work if title matches
    assert_eq!(classifier.classify("firefox").to_string(), "study");
    assert_eq!(
        classifier
            .classify_with_title("firefox", "GitHub - mono/src")
            .to_string(),
        "work"
    );
    assert_eq!(
        classifier
            .classify_with_title("chromium", "YouTube - Rust Tutorial")
            .to_string(),
        "work"
    );
    assert_eq!(
        classifier
            .classify_with_title("chromium", "YouTube - Cat Videos")
            .to_string(),
        "rest"
    );

    // Terminals/Editors should be work
    assert_eq!(classifier.classify("wezterm").to_string(), "work");
    assert_eq!(classifier.classify("code").to_string(), "work");
    assert_eq!(
        classifier
            .classify_with_title("code", "mono/src/main.rs")
            .to_string(),
        "work"
    );
}

#[test]
fn test_dynamic_scheduler_recommendations() {
    let engine = SchedulingEngine::with_default_policies();
    let scheduler = DynamicScheduler::new(engine);

    let task1 = Task::new("Work task".to_string())
        .with_priority(Priority::High)
        .with_tags(vec!["work".to_string()]);
    let task2 = Task::new("Study task".to_string())
        .with_priority(Priority::Medium)
        .with_tags(vec!["study".to_string()]);

    let available = vec![task1.clone(), task2.clone()];

    // Should recommend high priority task
    let next = scheduler.recommend_next(available.clone());
    assert!(next.is_some());
    assert_eq!(next.unwrap().task.title, "Work task");

    // After completion of task1, should recommend task2
    let next_after = scheduler.on_task_completed(&task1, vec![task2.clone()]);
    assert!(next_after.is_some());
    assert_eq!(next_after.unwrap().task.title, "Study task");
}

#[test]
fn test_interruption_detection_logic() {
    // This tests the logic used by the detector
    let classifier = AppClassifier::new();
    let task = Task::new("Work".to_string()).with_tags(vec!["work".to_string()]);
    let task_type = task.task_type();

    // Context matches
    let context_work = classifier.classify_with_title("wezterm", "");
    assert_eq!(context_work.name, task_type.name);

    // Context mismatch (interruption)
    let context_rest = classifier.classify_with_title("firefox", "YouTube");
    assert_ne!(context_rest.name, task_type.name);
    assert_eq!(context_rest.name, "rest");
}
