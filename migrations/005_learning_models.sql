-- Serialized learning models storage
-- Stores the complete LearningManager state as JSON for persistence across restarts

CREATE TABLE IF NOT EXISTS learning_models (
    id TEXT PRIMARY KEY NOT NULL DEFAULT 'global',
    global_model_json TEXT NOT NULL,
    task_type_models_json TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Insert default empty state
INSERT OR IGNORE INTO learning_models (id, global_model_json, task_type_models_json, version, created_at, updated_at)
VALUES ('global', '{}', '{}', 1, datetime('now'), datetime('now'));
