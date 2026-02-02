mod scheduler;
mod server;
mod state;

pub use scheduler::Scheduler;
pub use server::DaemonServer;
pub use state::DaemonState;

use std::fs::File;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info};

use crate::config::{ConfigWatcher, MonoPaths};
use crate::error::{MonoError, Result};
use crate::platform::{Platform, UnixPlatform};
use crate::storage::{SqliteStorage, create_pool, run_migrations};

pub async fn run_daemon_foreground(paths: &MonoPaths) -> Result<()> {
    let platform = UnixPlatform::new();

    if let Some(pid) = platform.read_pid_file(&paths.pid_file)? {
        if platform.is_process_running(pid) {
            return Err(MonoError::DaemonAlreadyRunning { pid });
        }
        platform.remove_pid_file(&paths.pid_file)?;
    }

    platform.write_pid_file(&paths.pid_file, platform.current_pid())?;
    let result = run_daemon_main(paths).await;
    platform.remove_pid_file(&paths.pid_file)?;
    result
}

pub fn run_daemon_background(paths: &MonoPaths) -> Result<()> {
    use daemonize2::Daemonize;

    let platform = UnixPlatform::new();

    if let Some(pid) = platform.read_pid_file(&paths.pid_file)? {
        if platform.is_process_running(pid) {
            return Err(MonoError::DaemonAlreadyRunning { pid });
        }
        platform.remove_pid_file(&paths.pid_file)?;
    }

    let stdout = File::create(&paths.log_file)
        .map_err(|e| MonoError::DaemonStart(format!("Failed to create log file: {}", e)))?;
    let stderr = stdout
        .try_clone()
        .map_err(|e| MonoError::DaemonStart(format!("Failed to clone log file handle: {}", e)))?;

    let daemonize = Daemonize::new()
        .pid_file(&paths.pid_file)
        .working_directory(&paths.data_dir)
        .stdout(stdout)
        .stderr(stderr);

    unsafe {
        daemonize.start().map_err(MonoError::Daemonize)?;
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let paths_clone = paths.clone();
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| MonoError::DaemonStart(format!("Failed to create tokio runtime: {}", e)))?;

    rt.block_on(async {
        let result = run_daemon_main(&paths_clone).await;
        platform.remove_pid_file(&paths_clone.pid_file)?;
        result
    })
}

async fn run_daemon_main(paths: &MonoPaths) -> Result<()> {
    let config_watcher = Arc::new(ConfigWatcher::new(paths.config_file()).await?);
    let settings = config_watcher.current_settings();
    let settings_rx = config_watcher.subscribe();

    let pool = create_pool(&paths.database).await?;
    run_migrations(&pool).await?;

    let storage = SqliteStorage::new(pool);
    let mut daemon_state = DaemonState::new(storage, paths.clone(), settings, settings_rx).await;

    daemon_state.init_notification_backend().await;

    let state = Arc::new(daemon_state);

    let (shutdown_tx, _) = broadcast::channel::<()>(1);

    let server = Arc::new(DaemonServer::new(Arc::clone(&state)));
    let mut scheduler = Scheduler::new(Arc::clone(&state), shutdown_tx.subscribe());

    let server_clone = Arc::clone(&server);
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server_clone.run().await {
            error!("Server error: {}", e);
        }
    });

    let scheduler_handle = tokio::spawn(async move {
        scheduler.run().await;
    });

    let config_watcher_clone = Arc::clone(&config_watcher);
    let state_clone = Arc::clone(&state);
    let config_watcher_handle = tokio::spawn(async move {
        let mut settings_rx = state_clone.settings_receiver();
        loop {
            tokio::select! {
                result = settings_rx.changed() => {
                    if result.is_ok() {
                        let new_settings = settings_rx.borrow_and_update().clone();
                        state_clone.update_settings(new_settings).await;
                    } else {
                        break;
                    }
                }
                _ = config_watcher_clone.run() => {
                    break;
                }
            }
        }
    });

    let state_clone = Arc::clone(&state);
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        setup_signal_handler(state_clone, shutdown_tx_clone).await;
    });

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        if state.is_shutdown_requested().await {
            info!("Shutdown requested, stopping daemon...");
            server.trigger_shutdown();
            let _ = shutdown_tx.send(());
            let _ = config_watcher.stop().await;
            break;
        }
    }

    let _ = tokio::time::timeout(tokio::time::Duration::from_secs(5), async {
        let _ = server_handle.await;
        let _ = scheduler_handle.await;
        config_watcher_handle.abort();
    })
    .await;

    info!("Daemon stopped");
    Ok(())
}

async fn setup_signal_handler(state: Arc<DaemonState>, _shutdown_tx: broadcast::Sender<()>) {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to setup SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to setup SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT");
            }
        }

        state.request_shutdown().await;
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to setup Ctrl+C handler");
        info!("Received Ctrl+C");
        state.request_shutdown().await;
    }
}

pub fn stop_daemon(paths: &MonoPaths) -> Result<()> {
    let platform = UnixPlatform::new();

    match platform.read_pid_file(&paths.pid_file)? {
        Some(pid) => {
            if platform.is_process_running(pid) {
                platform.kill_process(pid)?;
                platform.remove_pid_file(&paths.pid_file)?;
                Ok(())
            } else {
                platform.remove_pid_file(&paths.pid_file)?;
                Err(MonoError::DaemonNotRunning)
            }
        }
        None => Err(MonoError::DaemonNotRunning),
    }
}

pub fn daemon_status(paths: &MonoPaths) -> Result<Option<u32>> {
    let platform = UnixPlatform::new();

    match platform.read_pid_file(&paths.pid_file)? {
        Some(pid) => {
            if platform.is_process_running(pid) {
                Ok(Some(pid))
            } else {
                platform.remove_pid_file(&paths.pid_file)?;
                Ok(None)
            }
        }
        None => Ok(None),
    }
}
