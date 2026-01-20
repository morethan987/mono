use chrono::Utc;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::daemon::DaemonState;
use crate::error::{MonoError, Result};
use crate::models::{Priority, Task, TaskStatus};
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
                        Ok(()) => Response::task(task),
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
                    Ok(()) => Response::task(task),
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
    }
}
