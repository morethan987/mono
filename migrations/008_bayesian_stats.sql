-- Migration 008: Add Bayesian duration prediction fields to task_type_stats
-- Tracks sum and sum of squares for execution times

ALTER TABLE task_type_stats ADD COLUMN sum_duration REAL DEFAULT 0.0;
ALTER TABLE task_type_stats ADD COLUMN sum_duration_sq REAL DEFAULT 0.0;
ALTER TABLE task_type_stats ADD COLUMN duration_count INTEGER DEFAULT 0;
