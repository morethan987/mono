use chrono::Utc;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::daemon::DaemonState;
use crate::error::{MonoError, Result};
use crate::models::{Feedback, Priority, Task, TaskStatus};
use crate::protocol::{Request, Response, decode_request, encode_response};
use crate::storage::TaskRepository;

pub struct DaemonServer {
    state: Arc<DaemonState>,
    shutdown_tx: broadcast::Sender<()>,
}

impl DaemonServer {
    pub fn new(state: Arc<DaemonState>) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self { state, shutdown_tx }
    }

    pub async fn run(&self) -> Result<()> {
        let socket_path = &self.state.paths.socket;

        if socket_path.exists() {
            std::fs::remove_file(socket_path)
                .map_err(|e| MonoError::Platform(format!("Failed to remove old socket: {}", e)))?;
        }

        let listener = UnixListener::bind(socket_path).map_err(|e| MonoError::IpcConnection(e))?;

        info!("Daemon server listening on {:?}", socket_path);

        let mut shutdown_rx = self.shutdown_tx.subscribe();

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, _)) => {
                            let state = Arc::clone(&self.state);
                            let mut conn_shutdown_rx = self.shutdown_tx.subscribe();
                            tokio::spawn(async move {
                                tokio::select! {
                                    _ = handle_connection(stream, state) => {}
                                    _ = conn_shutdown_rx.recv() => {
                                        debug!("Connection closed due to shutdown");
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            error!("Accept error: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Server shutdown signal received");
                    break;
                }
            }
        }

        if socket_path.exists() {
            let _ = std::fs::remove_file(socket_path);
        }

        info!("Daemon server stopped");
        Ok(())
    }

    pub fn trigger_shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

async fn handle_connection(stream: UnixStream, state: Arc<DaemonState>) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                debug!("Client disconnected");
                break;
            }
            Ok(_) => {
                let response = match decode_request(&line) {
                    Ok(request) => {
                        debug!("Received request: {}", request.name());
                        handle_request(request, &state).await
                    }
                    Err(e) => {
                        warn!("Invalid request: {}", e);
                        Response::error(format!("Invalid request: {}", e))
                    }
                };

                match encode_response(&response) {
                    Ok(encoded) => {
                        if let Err(e) = writer.write_all(encoded.as_bytes()).await {
                            error!("Write error: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Failed to encode response: {}", e);
                        break;
                    }
                }
            }
            Err(e) => {
                error!("Read error: {}", e);
                break;
            }
        }
    }
}

async fn handle_request(request: Request, state: &DaemonState) -> Response {
    match request {
        Request::Ping => Response::Pong,

        Request::Shutdown => {
            state.request_shutdown().await;
            Response::ok()
        }

        Request::AddTask {
            title,
            description,
            priority,
            tags,
            estimated_minutes,
            deadline,
        } => {
            let mut task = Task::new(title);
            task.description = description;
            task.priority = priority.unwrap_or(Priority::Medium);
            task.tags = tags;
            task.estimated_minutes = estimated_minutes;
            task.deadline = deadline;

            match state.storage.create(&task).await {
                Ok(()) => Response::task(task),
                Err(e) => Response::error(format!("Failed to create task: {}", e)),
            }
        }

        Request::GetTask { id } => match state.storage.get_by_short_id(&id).await {
            Ok(Some(task)) => Response::task(task),
            Ok(None) => Response::error(format!("Task not found: {}", id)),
            Err(e) => Response::error(format!("Database error: {}", e)),
        },

        Request::ListTasks { status, limit } => match state.storage.list(status, limit).await {
            Ok(tasks) => Response::task_list(tasks),
            Err(e) => Response::error(format!("Failed to list tasks: {}", e)),
        },

        Request::ListToday => match state.storage.list_today().await {
            Ok(tasks) => Response::task_list(tasks),
            Err(e) => Response::error(format!("Failed to list today's tasks: {}", e)),
        },

        Request::GetCurrentTask => match state.storage.list_pending().await {
            Ok(tasks) => {
                let current = state.scheduler.get_next_task(tasks).map(|st| st.task);
                Response::CurrentTask { task: current }
            }
            Err(e) => Response::error(format!("Failed to get current task: {}", e)),
        },

        Request::UpdateTaskStatus { id, status } => {
            match state.storage.get_by_short_id(&id).await {
                Ok(Some(mut task)) => {
                    task.status = status;
                    task.updated_at = Utc::now();
                    if status == TaskStatus::Completed {
                        task.completed_at = Some(Utc::now());
                    }
                    match state.storage.update(&task).await {
                        Ok(()) => Response::task(task),
                        Err(e) => Response::error(format!("Failed to update task: {}", e)),
                    }
                }
                Ok(None) => Response::error(format!("Task not found: {}", id)),
                Err(e) => Response::error(format!("Database error: {}", e)),
            }
        }

        Request::CompleteTask { id, actual_minutes } => {
            match state.storage.get_by_short_id(&id).await {
                Ok(Some(mut task)) => {
                    task.status = TaskStatus::Completed;
                    task.completed_at = Some(Utc::now());
                    task.actual_minutes = actual_minutes;
                    task.updated_at = Utc::now();
                    match state.storage.update(&task).await {
                        Ok(()) => {
                            let feedback = Feedback::completed(
                                task.id.clone(),
                                actual_minutes.unwrap_or(0),
                            );
                            let mut lm = state.learning_manager.write().await;
                            lm.update_from_feedback(&task, &feedback);
                            drop(lm);
                            state.maybe_save_learning_model().await;
                            debug!("Updated learning model for completed task: {}", task.short_id());
                            Response::task(task)
                        }
                        Err(e) => Response::error(format!("Failed to complete task: {}", e)),
                    }
                }
                Ok(None) => Response::error(format!("Task not found: {}", id)),
                Err(e) => Response::error(format!("Database error: {}", e)),
            }
        }

        Request::PostponeTask { id, minutes } => match state.storage.get_by_short_id(&id).await {
            Ok(Some(mut task)) => {
                task.status = TaskStatus::Postponed;
                if let Some(scheduled) = task.scheduled_at {
                    task.scheduled_at = Some(scheduled + chrono::Duration::minutes(minutes as i64));
                }
                task.updated_at = Utc::now();
                match state.storage.update(&task).await {
                    Ok(()) => {
                        let feedback = Feedback::postponed(task.id.clone(), minutes);
                        let mut lm = state.learning_manager.write().await;
                        lm.update_from_feedback(&task, &feedback);
                        drop(lm);
                        state.maybe_save_learning_model().await;
                        debug!("Updated learning model for postponed task: {}", task.short_id());
                        Response::task(task)
                    }
                    Err(e) => Response::error(format!("Failed to postpone task: {}", e)),
                }
            }
            Ok(None) => Response::error(format!("Task not found: {}", id)),
            Err(e) => Response::error(format!("Database error: {}", e)),
        },

        Request::DeleteTask { id } => match state.storage.get_by_short_id(&id).await {
            Ok(Some(task)) => match state.storage.delete(&task.id).await {
                Ok(()) => Response::ok(),
                Err(e) => Response::error(format!("Failed to delete task: {}", e)),
            },
            Ok(None) => Response::error(format!("Task not found: {}", id)),
            Err(e) => Response::error(format!("Database error: {}", e)),
        },

        Request::UpdateTask {
            id,
            title,
            description,
            priority,
            tags,
            estimated_minutes,
            deadline,
        } => match state.storage.get_by_short_id(&id).await {
            Ok(Some(mut task)) => {
                if let Some(t) = title {
                    task.title = t;
                }
                if description.is_some() {
                    task.description = description;
                }
                if let Some(p) = priority {
                    task.priority = p;
                }
                if let Some(t) = tags {
                    task.tags = t;
                }
                if estimated_minutes.is_some() {
                    task.estimated_minutes = estimated_minutes;
                }
                if deadline.is_some() {
                    task.deadline = deadline;
                }
                task.updated_at = Utc::now();

                match state.storage.update(&task).await {
                    Ok(()) => Response::task(task),
                    Err(e) => Response::error(format!("Failed to update task: {}", e)),
                }
            }
            Ok(None) => Response::error(format!("Task not found: {}", id)),
            Err(e) => Response::error(format!("Database error: {}", e)),
        },

        Request::GetDaemonStatus => {
            let task_count = state
                .storage
                .list(None, None)
                .await
                .map(|t| t.len() as u64)
                .unwrap_or(0);

            Response::DaemonStatus {
                running: true,
                pid: std::process::id(),
                uptime_secs: state.uptime_secs(),
                task_count,
                started_at: state.started_at,
            }
        }

        Request::Replan => match state.storage.list_pending().await {
            Ok(tasks) => {
                let ranked = state
                    .scheduler
                    .rank_tasks(tasks, &crate::scheduling::SchedulingContext::new());
                let ranked_tasks = ranked
                    .into_iter()
                    .map(|st| crate::protocol::RankedTask {
                        task: st.task,
                        score: st.score,
                    })
                    .collect();
                Response::RankedTasks {
                    tasks: ranked_tasks,
                }
            }
            Err(e) => Response::error(format!("Failed to replan: {}", e)),
        },

        Request::SubmitFeedback {
            task_id,
            rating,
            difficulty,
            energy_level,
            notes,
        } => match state.storage.get_by_short_id(&task_id).await {
            Ok(Some(task)) => {
                let mut feedback = Feedback::new(task.id.clone(), crate::models::FeedbackType::Completed);
                feedback.rating = rating;
                feedback.difficulty_rating = difficulty;
                feedback.energy_level = energy_level;
                feedback.notes = notes;

                let mut lm = state.learning_manager.write().await;
                lm.update_from_feedback(&task, &feedback);
                drop(lm);
                state.maybe_save_learning_model().await;
                debug!("Submitted detailed feedback for task: {}", task.short_id());
                Response::ok()
            }
            Ok(None) => Response::error(format!("Task not found: {}", task_id)),
            Err(e) => Response::error(format!("Database error: {}", e)),
        },

        Request::GetLearningStats { task_type } => {
            let lm = state.learning_manager.read().await;

            let task_type_stats: Vec<crate::protocol::TaskTypeStatsData> = if let Some(ref tt) = task_type {
                lm.get_model(tt)
                    .map(|m| vec![model_to_stats_data(m)])
                    .unwrap_or_default()
            } else {
                lm.all_models()
                    .map(|(_, m)| model_to_stats_data(m))
                    .collect()
            };

            let global = lm.global_stats();
            let time_slot_stats = bandit_to_time_slot_stats(&global.time_slot_bandit);

            Response::LearningStats {
                stats: crate::protocol::LearningStatsData {
                    total_tasks_learned: global.total_tasks,
                    task_type_stats,
                    time_slot_stats,
                },
            }
        }

        Request::GetTimeSlotRecommendation { task_id } => {
            match state.storage.get_by_short_id(&task_id).await {
                Ok(Some(task)) => {
                    let lm = state.learning_manager.read().await;
                    let task_type = task.task_type();
                    let recommended = lm.suggest_time_slot(&task);
                    let confidence = lm
                        .get_model(&task_type.name)
                        .map(|m| (m.total_scheduled as f64 / 10.0).min(1.0))
                        .unwrap_or(0.0);
                    Response::TimeSlotRecommendation {
                        task_id: task.short_id().to_string(),
                        task_type: task_type.name,
                        recommended_slot: recommended.to_string(),
                        confidence,
                    }
                }
                Ok(None) => Response::error(format!("Task not found: {}", task_id)),
                Err(e) => Response::error(format!("Database error: {}", e)),
            }
        }
    }
}

fn model_to_stats_data(model: &crate::learning::TaskTypeLearningModel) -> crate::protocol::TaskTypeStatsData {
    crate::protocol::TaskTypeStatsData {
        task_type: model.task_type.name.clone(),
        total_scheduled: model.total_scheduled,
        total_completed: model.total_completed,
        total_postponed: model.total_postponed,
        completion_rate: model.completion_rate(),
        best_time_slot: format!("{:?}", model.best_time_slot()),
        avg_duration_minutes: model.avg_duration_minutes,
    }
}

fn bandit_to_time_slot_stats(bandit: &crate::learning::TimeSlotBandit) -> crate::protocol::TimeSlotStatsData {
    use crate::learning::TimeSlotArm;

    let slot_detail = |arm: TimeSlotArm| {
        let stats = bandit.get_stats(arm);
        crate::protocol::TimeSlotDetail {
            successes: (stats.successes - 1.0).max(0.0) as u32,
            failures: (stats.failures - 1.0).max(0.0) as u32,
            success_rate: stats.success_rate(),
        }
    };

    crate::protocol::TimeSlotStatsData {
        morning: slot_detail(TimeSlotArm::Morning),
        afternoon: slot_detail(TimeSlotArm::Afternoon),
        evening: slot_detail(TimeSlotArm::Evening),
        night: slot_detail(TimeSlotArm::Night),
    }
}
