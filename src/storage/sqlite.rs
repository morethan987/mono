use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Pool, Row, Sqlite};

use crate::error::{MonoError, Result};
use crate::models::{Feedback, FeedbackType, Priority, Schedule, ScheduleStatus, Task, TaskStatus};
use crate::storage::repository::{FeedbackRepository, LearningRepository, ScheduleRepository, TaskRepository, TaskTypeStats, TimeSlotStats};

#[derive(Clone)]
pub struct SqliteStorage {
    pool: Pool<Sqlite>,
}

impl SqliteStorage {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }
}

fn parse_tags(tags_str: &str) -> Vec<String> {
    serde_json::from_str(tags_str).unwrap_or_default()
}

fn serialize_tags(tags: &[String]) -> String {
    serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string())
}

fn parse_datetime(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.to_utc())
}

fn format_datetime(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

fn format_datetime_opt(dt: &Option<DateTime<Utc>>) -> Option<String> {
    dt.as_ref().map(format_datetime)
}

#[async_trait]
impl TaskRepository for SqliteStorage {
    async fn create(&self, task: &Task) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO tasks (
                id, title, description, priority, status, tags,
                estimated_minutes, actual_minutes, deadline, scheduled_at,
                started_at, completed_at, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&task.id)
        .bind(&task.title)
        .bind(&task.description)
        .bind(task.priority.as_i32())
        .bind(task.status.as_str())
        .bind(serialize_tags(&task.tags))
        .bind(task.estimated_minutes.map(|m| m as i32))
        .bind(task.actual_minutes.map(|m| m as i32))
        .bind(format_datetime_opt(&task.deadline))
        .bind(format_datetime_opt(&task.scheduled_at))
        .bind(format_datetime_opt(&task.started_at))
        .bind(format_datetime_opt(&task.completed_at))
        .bind(format_datetime(&task.created_at))
        .bind(format_datetime(&task.updated_at))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<Task>> {
        let row = sqlx::query(
            r#"
            SELECT id, title, description, priority, status, tags,
                   estimated_minutes, actual_minutes, deadline, scheduled_at,
                   started_at, completed_at, created_at, updated_at
            FROM tasks WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| row_to_task(&r)))
    }

    async fn get_by_short_id(&self, short_id: &str) -> Result<Option<Task>> {
        let pattern = format!("{}%", short_id);
        let row = sqlx::query(
            r#"
            SELECT id, title, description, priority, status, tags,
                   estimated_minutes, actual_minutes, deadline, scheduled_at,
                   started_at, completed_at, created_at, updated_at
            FROM tasks WHERE id LIKE ?
            LIMIT 1
            "#,
        )
        .bind(&pattern)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| row_to_task(&r)))
    }

    async fn list(&self, status: Option<TaskStatus>, limit: Option<u32>) -> Result<Vec<Task>> {
        let limit_val = limit.unwrap_or(100) as i32;

        let rows = match status {
            Some(s) => {
                sqlx::query(
                    r#"
                    SELECT id, title, description, priority, status, tags,
                           estimated_minutes, actual_minutes, deadline, scheduled_at,
                           started_at, completed_at, created_at, updated_at
                    FROM tasks
                    WHERE status = ?
                    ORDER BY priority DESC, deadline ASC, created_at DESC
                    LIMIT ?
                    "#,
                )
                .bind(s.as_str())
                .bind(limit_val)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query(
                    r#"
                    SELECT id, title, description, priority, status, tags,
                           estimated_minutes, actual_minutes, deadline, scheduled_at,
                           started_at, completed_at, created_at, updated_at
                    FROM tasks
                    ORDER BY priority DESC, deadline ASC, created_at DESC
                    LIMIT ?
                    "#,
                )
                .bind(limit_val)
                .fetch_all(&self.pool)
                .await?
            }
        };

        Ok(rows.iter().map(row_to_task).collect())
    }

    async fn list_pending(&self) -> Result<Vec<Task>> {
        self.list(Some(TaskStatus::Pending), None).await
    }

    async fn list_today(&self) -> Result<Vec<Task>> {
        let today_start = Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap();
        let today_end = Utc::now().date_naive().and_hms_opt(23, 59, 59).unwrap();

        let rows = sqlx::query(
            r#"
            SELECT id, title, description, priority, status, tags,
                   estimated_minutes, actual_minutes, deadline, scheduled_at,
                   started_at, completed_at, created_at, updated_at
            FROM tasks
            WHERE (scheduled_at BETWEEN ? AND ?)
               OR (deadline BETWEEN ? AND ?)
               OR (status IN ('pending', 'in_progress'))
            ORDER BY priority DESC, deadline ASC
            "#,
        )
        .bind(today_start.to_string())
        .bind(today_end.to_string())
        .bind(today_start.to_string())
        .bind(today_end.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(row_to_task).collect())
    }

    async fn list_ready_for_notification(&self) -> Result<Vec<Task>> {
        let now = format_datetime(&Utc::now());
        let rows = sqlx::query(
            r#"
            SELECT id, title, description, priority, status, tags,
                   estimated_minutes, actual_minutes, deadline, scheduled_at,
                   started_at, completed_at, created_at, updated_at
            FROM tasks
            WHERE status = 'pending'
              AND (scheduled_at IS NULL OR scheduled_at <= ?)
            ORDER BY priority DESC, deadline ASC
            LIMIT 1
            "#,
        )
        .bind(&now)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(row_to_task).collect())
    }

    async fn update(&self, task: &Task) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE tasks SET
                title = ?, description = ?, priority = ?, status = ?, tags = ?,
                estimated_minutes = ?, actual_minutes = ?, deadline = ?, scheduled_at = ?,
                started_at = ?, completed_at = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&task.title)
        .bind(&task.description)
        .bind(task.priority.as_i32())
        .bind(task.status.as_str())
        .bind(serialize_tags(&task.tags))
        .bind(task.estimated_minutes.map(|m| m as i32))
        .bind(task.actual_minutes.map(|m| m as i32))
        .bind(format_datetime_opt(&task.deadline))
        .bind(format_datetime_opt(&task.scheduled_at))
        .bind(format_datetime_opt(&task.started_at))
        .bind(format_datetime_opt(&task.completed_at))
        .bind(format_datetime(&Utc::now()))
        .bind(&task.id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(MonoError::TaskNotFound {
                id: task.id.clone(),
            });
        }

        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(MonoError::TaskNotFound { id: id.to_string() });
        }

        Ok(())
    }
}

fn row_to_task(row: &sqlx::sqlite::SqliteRow) -> Task {
    Task {
        id: row.get("id"),
        title: row.get("title"),
        description: row.get("description"),
        priority: Priority::from_i32(row.get("priority")),
        status: TaskStatus::from_str(row.get("status")),
        tags: parse_tags(row.get("tags")),
        estimated_minutes: row
            .get::<Option<i32>, _>("estimated_minutes")
            .map(|m| m as u32),
        actual_minutes: row
            .get::<Option<i32>, _>("actual_minutes")
            .map(|m| m as u32),
        deadline: row
            .get::<Option<String>, _>("deadline")
            .and_then(|s| parse_datetime(&s)),
        scheduled_at: row
            .get::<Option<String>, _>("scheduled_at")
            .and_then(|s| parse_datetime(&s)),
        started_at: row
            .get::<Option<String>, _>("started_at")
            .and_then(|s| parse_datetime(&s)),
        completed_at: row
            .get::<Option<String>, _>("completed_at")
            .and_then(|s| parse_datetime(&s)),
        created_at: parse_datetime(row.get("created_at")).unwrap_or_else(Utc::now),
        updated_at: parse_datetime(row.get("updated_at")).unwrap_or_else(Utc::now),
    }
}

#[async_trait]
impl ScheduleRepository for SqliteStorage {
    async fn create(&self, schedule: &Schedule) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO schedules (
                id, task_id, scheduled_start, scheduled_end,
                actual_start, actual_end, status, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&schedule.id)
        .bind(&schedule.task_id)
        .bind(format_datetime(&schedule.scheduled_start))
        .bind(format_datetime(&schedule.scheduled_end))
        .bind(format_datetime_opt(&schedule.actual_start))
        .bind(format_datetime_opt(&schedule.actual_end))
        .bind(schedule.status.as_str())
        .bind(format_datetime(&schedule.created_at))
        .bind(format_datetime(&schedule.updated_at))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<Schedule>> {
        let row = sqlx::query(
            r#"
            SELECT id, task_id, scheduled_start, scheduled_end,
                   actual_start, actual_end, status, created_at, updated_at
            FROM schedules WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| row_to_schedule(&r)))
    }

    async fn get_current(&self) -> Result<Option<Schedule>> {
        let now = format_datetime(&Utc::now());
        let row = sqlx::query(
            r#"
            SELECT id, task_id, scheduled_start, scheduled_end,
                   actual_start, actual_end, status, created_at, updated_at
            FROM schedules
            WHERE scheduled_start <= ? AND scheduled_end > ?
              AND status IN ('planned', 'active')
            ORDER BY scheduled_start ASC
            LIMIT 1
            "#,
        )
        .bind(&now)
        .bind(&now)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| row_to_schedule(&r)))
    }

    async fn list_for_task(&self, task_id: &str) -> Result<Vec<Schedule>> {
        let rows = sqlx::query(
            r#"
            SELECT id, task_id, scheduled_start, scheduled_end,
                   actual_start, actual_end, status, created_at, updated_at
            FROM schedules WHERE task_id = ?
            ORDER BY scheduled_start DESC
            "#,
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(row_to_schedule).collect())
    }

    async fn list_today(&self) -> Result<Vec<Schedule>> {
        let today_start = Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap();
        let today_end = Utc::now().date_naive().and_hms_opt(23, 59, 59).unwrap();

        let rows = sqlx::query(
            r#"
            SELECT id, task_id, scheduled_start, scheduled_end,
                   actual_start, actual_end, status, created_at, updated_at
            FROM schedules
            WHERE scheduled_start BETWEEN ? AND ?
            ORDER BY scheduled_start ASC
            "#,
        )
        .bind(today_start.to_string())
        .bind(today_end.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(row_to_schedule).collect())
    }

    async fn update(&self, schedule: &Schedule) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE schedules SET
                scheduled_start = ?, scheduled_end = ?,
                actual_start = ?, actual_end = ?, status = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(format_datetime(&schedule.scheduled_start))
        .bind(format_datetime(&schedule.scheduled_end))
        .bind(format_datetime_opt(&schedule.actual_start))
        .bind(format_datetime_opt(&schedule.actual_end))
        .bind(schedule.status.as_str())
        .bind(format_datetime(&Utc::now()))
        .bind(&schedule.id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(MonoError::ScheduleNotFound {
                id: schedule.id.clone(),
            });
        }

        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM schedules WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

fn row_to_schedule(row: &sqlx::sqlite::SqliteRow) -> Schedule {
    Schedule {
        id: row.get("id"),
        task_id: row.get("task_id"),
        scheduled_start: parse_datetime(row.get("scheduled_start")).unwrap_or_else(Utc::now),
        scheduled_end: parse_datetime(row.get("scheduled_end")).unwrap_or_else(Utc::now),
        actual_start: row
            .get::<Option<String>, _>("actual_start")
            .and_then(|s| parse_datetime(&s)),
        actual_end: row
            .get::<Option<String>, _>("actual_end")
            .and_then(|s| parse_datetime(&s)),
        status: ScheduleStatus::from_str(row.get("status")),
        created_at: parse_datetime(row.get("created_at")).unwrap_or_else(Utc::now),
        updated_at: parse_datetime(row.get("updated_at")).unwrap_or_else(Utc::now),
    }
}

#[async_trait]
impl FeedbackRepository for SqliteStorage {
    async fn create(&self, feedback: &Feedback) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO feedback (
                id, task_id, schedule_id, feedback_type, rating,
                actual_duration_minutes, difficulty_rating, energy_level,
                notes, postpone_minutes, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&feedback.id)
        .bind(&feedback.task_id)
        .bind(&feedback.schedule_id)
        .bind(feedback.feedback_type.as_str())
        .bind(feedback.rating.map(|r| r as i32))
        .bind(feedback.actual_duration_minutes.map(|m| m as i32))
        .bind(feedback.difficulty_rating.map(|r| r as i32))
        .bind(feedback.energy_level.map(|e| e as i32))
        .bind(&feedback.notes)
        .bind(feedback.postpone_minutes.map(|m| m as i32))
        .bind(format_datetime(&feedback.created_at))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list_for_task(&self, task_id: &str) -> Result<Vec<Feedback>> {
        let rows = sqlx::query(
            r#"
            SELECT id, task_id, schedule_id, feedback_type, rating,
                   actual_duration_minutes, difficulty_rating, energy_level,
                   notes, postpone_minutes, created_at
            FROM feedback WHERE task_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(row_to_feedback).collect())
    }

    async fn list_recent(&self, limit: u32) -> Result<Vec<Feedback>> {
        let rows = sqlx::query(
            r#"
            SELECT id, task_id, schedule_id, feedback_type, rating,
                   actual_duration_minutes, difficulty_rating, energy_level,
                   notes, postpone_minutes, created_at
            FROM feedback
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i32)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(row_to_feedback).collect())
    }
}

fn row_to_feedback(row: &sqlx::sqlite::SqliteRow) -> Feedback {
    Feedback {
        id: row.get("id"),
        task_id: row.get("task_id"),
        schedule_id: row.get("schedule_id"),
        feedback_type: FeedbackType::from_str(row.get("feedback_type"))
            .unwrap_or(FeedbackType::Completed),
        rating: row.get::<Option<i32>, _>("rating").map(|r| r as u8),
        actual_duration_minutes: row
            .get::<Option<i32>, _>("actual_duration_minutes")
            .map(|m| m as u32),
        difficulty_rating: row
            .get::<Option<i32>, _>("difficulty_rating")
            .map(|r| r as u8),
        energy_level: row.get::<Option<i32>, _>("energy_level").map(|e| e as u8),
        notes: row.get("notes"),
        postpone_minutes: row
            .get::<Option<i32>, _>("postpone_minutes")
            .map(|m| m as u32),
        created_at: parse_datetime(row.get("created_at")).unwrap_or_else(Utc::now),
    }
}

#[async_trait]
impl LearningRepository for SqliteStorage {
    async fn get_task_type_stats(&self, task_type: &str) -> Result<Option<TaskTypeStats>> {
        let row = sqlx::query(
            r#"
            SELECT task_type, total_scheduled, total_completed, total_postponed, total_skipped,
                   avg_completion_rate, avg_duration_minutes, best_time_slots, model_weights
            FROM task_type_stats WHERE task_type = ?
            "#,
        )
        .bind(task_type)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| row_to_task_type_stats(&r)))
    }

    async fn upsert_task_type_stats(&self, stats: &TaskTypeStats) -> Result<()> {
        let best_time_slots_json = serde_json::to_string(&stats.best_time_slots)
            .unwrap_or_else(|_| "[]".to_string());

        sqlx::query(
            r#"
            INSERT INTO task_type_stats (
                task_type, total_scheduled, total_completed, total_postponed, total_skipped,
                avg_completion_rate, avg_duration_minutes, best_time_slots, model_weights, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(task_type) DO UPDATE SET
                total_scheduled = excluded.total_scheduled,
                total_completed = excluded.total_completed,
                total_postponed = excluded.total_postponed,
                total_skipped = excluded.total_skipped,
                avg_completion_rate = excluded.avg_completion_rate,
                avg_duration_minutes = excluded.avg_duration_minutes,
                best_time_slots = excluded.best_time_slots,
                model_weights = excluded.model_weights,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&stats.task_type)
        .bind(stats.total_scheduled as i32)
        .bind(stats.total_completed as i32)
        .bind(stats.total_postponed as i32)
        .bind(stats.total_skipped as i32)
        .bind(stats.avg_completion_rate)
        .bind(stats.avg_duration_minutes)
        .bind(&best_time_slots_json)
        .bind(&stats.model_weights)
        .bind(format_datetime(&Utc::now()))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list_all_task_type_stats(&self) -> Result<Vec<TaskTypeStats>> {
        let rows = sqlx::query(
            r#"
            SELECT task_type, total_scheduled, total_completed, total_postponed, total_skipped,
                   avg_completion_rate, avg_duration_minutes, best_time_slots, model_weights
            FROM task_type_stats
            ORDER BY total_scheduled DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(row_to_task_type_stats).collect())
    }

    async fn get_time_slot_stats(
        &self,
        task_type: &str,
        hour: u32,
        day: u32,
    ) -> Result<Option<TimeSlotStats>> {
        let row = sqlx::query(
            r#"
            SELECT id, task_type, hour_of_day, day_of_week, success_count, total_count, avg_rating
            FROM time_slot_stats
            WHERE task_type = ? AND hour_of_day = ? AND day_of_week = ?
            "#,
        )
        .bind(task_type)
        .bind(hour as i32)
        .bind(day as i32)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| row_to_time_slot_stats(&r)))
    }

    async fn upsert_time_slot_stats(&self, stats: &TimeSlotStats) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO time_slot_stats (
                task_type, hour_of_day, day_of_week, success_count, total_count, avg_rating, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(task_type, hour_of_day, day_of_week) DO UPDATE SET
                success_count = excluded.success_count,
                total_count = excluded.total_count,
                avg_rating = excluded.avg_rating,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&stats.task_type)
        .bind(stats.hour_of_day as i32)
        .bind(stats.day_of_week as i32)
        .bind(stats.success_count as i32)
        .bind(stats.total_count as i32)
        .bind(stats.avg_rating)
        .bind(format_datetime(&Utc::now()))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list_time_slot_stats_for_type(&self, task_type: &str) -> Result<Vec<TimeSlotStats>> {
        let rows = sqlx::query(
            r#"
            SELECT id, task_type, hour_of_day, day_of_week, success_count, total_count, avg_rating
            FROM time_slot_stats
            WHERE task_type = ?
            ORDER BY hour_of_day, day_of_week
            "#,
        )
        .bind(task_type)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(row_to_time_slot_stats).collect())
    }
}

fn row_to_task_type_stats(row: &sqlx::sqlite::SqliteRow) -> TaskTypeStats {
    let best_time_slots_str: String = row.get("best_time_slots");
    let best_time_slots: Vec<String> =
        serde_json::from_str(&best_time_slots_str).unwrap_or_default();

    TaskTypeStats {
        task_type: row.get("task_type"),
        total_scheduled: row.get::<i32, _>("total_scheduled") as u32,
        total_completed: row.get::<i32, _>("total_completed") as u32,
        total_postponed: row.get::<i32, _>("total_postponed") as u32,
        total_skipped: row.get::<i32, _>("total_skipped") as u32,
        avg_completion_rate: row.get("avg_completion_rate"),
        avg_duration_minutes: row.get("avg_duration_minutes"),
        best_time_slots,
        model_weights: row.get("model_weights"),
    }
}

fn row_to_time_slot_stats(row: &sqlx::sqlite::SqliteRow) -> TimeSlotStats {
    TimeSlotStats {
        id: row.get("id"),
        task_type: row.get("task_type"),
        hour_of_day: row.get::<i32, _>("hour_of_day") as u32,
        day_of_week: row.get::<i32, _>("day_of_week") as u32,
        success_count: row.get::<i32, _>("success_count") as u32,
        total_count: row.get::<i32, _>("total_count") as u32,
        avg_rating: row.get("avg_rating"),
    }
}

impl SqliteStorage {
    pub async fn load_learning_manager(&self) -> Result<Option<crate::learning::LearningManager>> {
        let row = sqlx::query(
            r#"SELECT global_model_json, task_type_models_json FROM learning_models WHERE id = 'global'"#,
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let global_json: String = row.get("global_model_json");
                let models_json: String = row.get("task_type_models_json");

                if global_json == "{}" && models_json == "{}" {
                    return Ok(None);
                }

                let global_model: crate::learning::GlobalLearningModel =
                    serde_json::from_str(&global_json)?;

                let models: std::collections::HashMap<
                    String,
                    crate::learning::TaskTypeLearningModel,
                > = serde_json::from_str(&models_json)?;

                let state = crate::learning::LearningManagerState {
                    global_model,
                    models,
                    version: 1,
                };

                Ok(Some(crate::learning::LearningManager::from_state(state)))
            }
            None => Ok(None),
        }
    }

    pub async fn save_learning_manager(
        &self,
        manager: &crate::learning::LearningManager,
    ) -> Result<()> {
        let state = manager.to_state();

        let global_json = serde_json::to_string(&state.global_model)?;

        let models_json = serde_json::to_string(&state.models)?;

        let now = format_datetime(&Utc::now());

        sqlx::query(
            r#"
            INSERT INTO learning_models (id, global_model_json, task_type_models_json, version, created_at, updated_at)
            VALUES ('global', ?1, ?2, 1, ?3, ?3)
            ON CONFLICT(id) DO UPDATE SET
                global_model_json = ?1,
                task_type_models_json = ?2,
                updated_at = ?3
            "#,
        )
        .bind(&global_json)
        .bind(&models_json)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
