-- Learning data table: Task type learning models storage
CREATE TABLE IF NOT EXISTS task_type_stats (
    task_type TEXT PRIMARY KEY NOT NULL,
    total_scheduled INTEGER NOT NULL DEFAULT 0,
    total_completed INTEGER NOT NULL DEFAULT 0,
    total_postponed INTEGER NOT NULL DEFAULT 0,
    total_skipped INTEGER NOT NULL DEFAULT 0,
    avg_completion_rate REAL NOT NULL DEFAULT 0.0,
    avg_duration_minutes REAL,
    best_time_slots TEXT NOT NULL DEFAULT '[]',
    model_weights TEXT NOT NULL DEFAULT '{}',
    updated_at TEXT NOT NULL
);

-- Time slot performance tracking
CREATE TABLE IF NOT EXISTS time_slot_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_type TEXT NOT NULL,
    hour_of_day INTEGER NOT NULL,
    day_of_week INTEGER NOT NULL,
    success_count INTEGER NOT NULL DEFAULT 0,
    total_count INTEGER NOT NULL DEFAULT 0,
    avg_rating REAL,
    updated_at TEXT NOT NULL,
    UNIQUE(task_type, hour_of_day, day_of_week)
);

CREATE INDEX IF NOT EXISTS idx_time_slot_stats_task_type ON time_slot_stats(task_type);
