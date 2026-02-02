//! File watcher abstraction layer.
//!
//! This module defines the `FileWatcher` trait that provides a platform-agnostic
//! interface for watching file system changes. Different platforms can implement
//! this trait using their native file notification mechanisms (e.g., inotify on Linux,
//! kqueue on macOS/BSD, ReadDirectoryChangesW on Windows).

use crate::error::Result;
use async_trait::async_trait;
use std::path::Path;

/// Events that can occur on a watched file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileEvent {
    /// File content was modified.
    Modified,
    /// File was created.
    Created,
    /// File was deleted.
    Deleted,
    /// File was renamed or moved.
    Renamed,
    /// File attributes changed (permissions, ownership, etc.).
    AttributeChanged,
    /// File was closed after being written to.
    ClosedWrite,
}

/// A platform-agnostic file watcher trait.
///
/// Implementations should use the operating system's native file notification
/// mechanism rather than polling for better performance and lower resource usage.
///
/// # Example
///
/// ```ignore
/// use mono::config::watcher::{FileWatcher, FileEvent};
///
/// let watcher = InotifyWatcher::new()?;
/// watcher.watch("/path/to/config.toml").await?;
///
/// loop {
///     match watcher.next_event().await? {
///         Some(event) => println!("File event: {:?}", event),
///         None => break, // Watcher was stopped
///     }
/// }
/// ```
#[async_trait]
pub trait FileWatcher: Send + Sync {
    /// Start watching a file for changes.
    ///
    /// # Arguments
    /// * `path` - The path to the file to watch.
    ///
    /// # Errors
    /// Returns an error if the file doesn't exist or cannot be watched.
    async fn watch(&self, path: &Path) -> Result<()>;

    /// Stop watching a file.
    ///
    /// # Arguments
    /// * `path` - The path to the file to stop watching.
    async fn unwatch(&self, path: &Path) -> Result<()>;

    /// Wait for and return the next file event.
    ///
    /// This method blocks until an event occurs or the watcher is stopped.
    ///
    /// # Returns
    /// * `Ok(Some(event))` - A file event occurred.
    /// * `Ok(None)` - The watcher was stopped.
    /// * `Err(_)` - An error occurred while waiting for events.
    async fn next_event(&self) -> Result<Option<FileEvent>>;

    /// Stop the watcher and release all resources.
    ///
    /// After calling this method, `next_event` will return `None`.
    async fn stop(&self) -> Result<()>;

    /// Check if the watcher is currently active.
    fn is_active(&self) -> bool;
}
