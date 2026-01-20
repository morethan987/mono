-- Feedback table: User feedback on task completion
CREATE TABLE IF NOT EXISTS feedback (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL,
    schedule_id TEXT,
    feedback_type TEXT NOT NULL,
    rating INTEGER,
    actual_duration_minutes INTEGER,
    difficulty_rating INTEGER,
    energy_level INTEGER,
    notes TEXT,
    postpone_minutes INTEGER,
    created_at TEXT NOT NULL,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (schedule_id) REFERENCES schedules(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_feedback_task_id ON feedback(task_id);
CREATE INDEX IF NOT EXISTS idx_feedback_created_at ON feedback(created_at);
