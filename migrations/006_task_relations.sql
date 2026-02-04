-- Migration 006: Add task relations support
-- Adds parent_task_id and spawned_from_task_id to tasks table
-- Creates task_relations table for generic task associations

-- Add parent_task_id column to tasks table
ALTER TABLE tasks ADD COLUMN parent_task_id TEXT REFERENCES tasks(id);

-- Add spawned_from_task_id column to tasks table (for tracking spawned tasks)
ALTER TABLE tasks ADD COLUMN spawned_from_task_id TEXT REFERENCES tasks(id);

-- Create task_relations table for generic task associations
CREATE TABLE task_relations (
    id TEXT PRIMARY KEY,
    source_task_id TEXT NOT NULL REFERENCES tasks(id),
    related_task_id TEXT NOT NULL REFERENCES tasks(id),
    relation_type TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for efficient lookups
CREATE INDEX idx_tasks_parent ON tasks(parent_task_id);
CREATE INDEX idx_tasks_spawned ON tasks(spawned_from_task_id);
CREATE INDEX idx_relations_source ON task_relations(source_task_id);
CREATE INDEX idx_relations_related ON task_relations(related_task_id);
