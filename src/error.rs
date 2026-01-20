//! Error types for the mono application.
//!
//! This module defines custom error types using `thiserror` for library-style errors
//! that can be easily converted and propagated throughout the application.

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for the mono application.
#[derive(Error, Debug)]
pub enum MonoError {
    // ========== Configuration Errors ==========
    #[error("Failed to determine project directories")]
    ProjectDirsNotFound,

    #[error("Failed to create directory: {path}")]
    DirectoryCreation {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Configuration file not found: {path}")]
    ConfigNotFound { path: PathBuf },

    #[error("Failed to parse configuration: {0}")]
    ConfigParse(#[from] toml::de::Error),

    #[error("Failed to serialize configuration: {0}")]
    ConfigSerialize(#[from] toml::ser::Error),

    // ========== Database Errors ==========
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Database migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("Task not found: {id}")]
    TaskNotFound { id: String },

    #[error("Schedule not found: {id}")]
    ScheduleNotFound { id: String },

    // ========== Daemon Errors ==========
    #[error("Daemon is not running")]
    DaemonNotRunning,

    #[error("Daemon is already running (PID: {pid})")]
    DaemonAlreadyRunning { pid: u32 },

    #[error("Failed to start daemon: {0}")]
    DaemonStart(String),

    #[error("Failed to stop daemon: {0}")]
    DaemonStop(String),

    #[error("Daemon communication error: {0}")]
    DaemonCommunication(String),

    #[error("Daemonize error: {0}")]
    Daemonize(#[from] daemonize2::Error),

    // ========== IPC Errors ==========
    #[error("IPC connection error: {0}")]
    IpcConnection(#[source] std::io::Error),

    #[error("IPC send error: {0}")]
    IpcSend(#[source] std::io::Error),

    #[error("IPC receive error: {0}")]
    IpcReceive(#[source] std::io::Error),

    #[error("IPC protocol error: {message}")]
    IpcProtocol { message: String },

    #[error("IPC timeout: no response within {timeout_secs} seconds")]
    IpcTimeout { timeout_secs: u64 },

    #[error("Socket not found: {path}")]
    SocketNotFound { path: PathBuf },

    // ========== Serialization Errors ==========
    #[error("JSON serialization error: {0}")]
    JsonSerialization(#[from] serde_json::Error),

    // ========== Platform Errors ==========
    #[error("Platform error: {0}")]
    Platform(String),

    #[error("Process not found: {pid}")]
    ProcessNotFound { pid: u32 },

    #[error("Failed to read PID file: {path}")]
    PidFileRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to write PID file: {path}")]
    PidFileWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Invalid PID in file: {content}")]
    InvalidPid { content: String },

    // ========== Notification Errors ==========
    #[error("Notification error: {0}")]
    Notification(String),

    #[error("DBus error: {0}")]
    DBus(#[from] zbus::Error),

    // ========== CLI Errors ==========
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Command not implemented: {command}")]
    NotImplemented { command: String },

    // ========== General I/O Errors ==========
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type alias using MonoError.
pub type Result<T> = std::result::Result<T, MonoError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = MonoError::TaskNotFound {
            id: "abc123".to_string(),
        };
        assert_eq!(err.to_string(), "Task not found: abc123");
    }

    #[test]
    fn test_error_conversion_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let mono_err: MonoError = io_err.into();
        assert!(matches!(mono_err, MonoError::Io(_)));
    }
}
