-- Migration 007: Add interruptions table for tracking task interruptions
-- Records when and why tasks are interrupted for pattern learning

CREATE TABLE IF NOT EXISTS interruptions (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    interruption_type TEXT NOT NULL,
    started_at DATETIME NOT NULL,
    ended_at DATETIME,
    reason TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_interruptions_task ON interruptions(task_id);
CREATE INDEX IF NOT EXISTS idx_interruptions_started ON interruptions(started_at);
CREATE INDEX IF NOT EXISTS idx_interruptions_type ON interruptions(interruption_type);
