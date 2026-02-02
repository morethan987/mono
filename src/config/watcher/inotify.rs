use crate::config::watcher::traits::{FileEvent, FileWatcher};
use crate::error::{MonoError, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::unix::AsyncFd;
use tokio::sync::Mutex;

const IN_MODIFY: u32 = 0x0000_0002;
const IN_ATTRIB: u32 = 0x0000_0004;
const IN_CLOSE_WRITE: u32 = 0x0000_0008;
const IN_MOVED_FROM: u32 = 0x0000_0040;
const IN_MOVED_TO: u32 = 0x0000_0080;
const IN_CREATE: u32 = 0x0000_0100;
const IN_DELETE: u32 = 0x0000_0200;
const IN_DELETE_SELF: u32 = 0x0000_0400;
const IN_MOVE_SELF: u32 = 0x0000_0800;

const WATCH_MASK: u32 = IN_MODIFY
    | IN_ATTRIB
    | IN_CLOSE_WRITE
    | IN_MOVED_FROM
    | IN_MOVED_TO
    | IN_CREATE
    | IN_DELETE
    | IN_DELETE_SELF
    | IN_MOVE_SELF;

const INOTIFY_EVENT_SIZE: usize = std::mem::size_of::<InotifyEventRaw>();

#[repr(C)]
struct InotifyEventRaw {
    wd: i32,
    mask: u32,
    cookie: u32,
    len: u32,
}

struct InotifyFd(i32);

impl InotifyFd {
    fn new() -> Result<Self> {
        let fd = unsafe { libc::inotify_init1(libc::IN_NONBLOCK | libc::IN_CLOEXEC) };
        if fd < 0 {
            return Err(MonoError::FileWatch(format!(
                "inotify_init1 failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        Ok(Self(fd))
    }

    fn add_watch(&self, path: &Path) -> Result<i32> {
        let c_path = CString::new(path.as_os_str().as_bytes()).map_err(|_| {
            MonoError::FileWatch(format!("invalid path: {}", path.display()))
        })?;

        let wd = unsafe { libc::inotify_add_watch(self.0, c_path.as_ptr(), WATCH_MASK) };
        if wd < 0 {
            return Err(MonoError::FileWatch(format!(
                "inotify_add_watch failed for {}: {}",
                path.display(),
                std::io::Error::last_os_error()
            )));
        }
        Ok(wd)
    }

    fn remove_watch(&self, wd: i32) -> Result<()> {
        if unsafe { libc::inotify_rm_watch(self.0, wd) } < 0 {
            return Err(MonoError::FileWatch(format!(
                "inotify_rm_watch failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        Ok(())
    }

    fn as_raw_fd(&self) -> i32 {
        self.0
    }
}

impl Drop for InotifyFd {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.0);
        }
    }
}

impl std::os::unix::io::AsRawFd for InotifyFd {
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        self.0
    }
}

pub struct InotifyWatcher {
    fd: Arc<AsyncFd<InotifyFd>>,
    watches: Mutex<HashMap<PathBuf, i32>>,
    active: AtomicBool,
}

impl InotifyWatcher {
    pub fn new() -> Result<Self> {
        let inotify_fd = InotifyFd::new()?;
        let async_fd = AsyncFd::new(inotify_fd)
            .map_err(|e| MonoError::FileWatch(format!("AsyncFd creation failed: {}", e)))?;

        Ok(Self {
            fd: Arc::new(async_fd),
            watches: Mutex::new(HashMap::new()),
            active: AtomicBool::new(true),
        })
    }

    fn parse_event(mask: u32) -> Option<FileEvent> {
        if mask & IN_CLOSE_WRITE != 0 {
            Some(FileEvent::ClosedWrite)
        } else if mask & IN_MODIFY != 0 {
            Some(FileEvent::Modified)
        } else if mask & IN_CREATE != 0 {
            Some(FileEvent::Created)
        } else if mask & (IN_DELETE | IN_DELETE_SELF) != 0 {
            Some(FileEvent::Deleted)
        } else if mask & (IN_MOVED_FROM | IN_MOVED_TO | IN_MOVE_SELF) != 0 {
            Some(FileEvent::Renamed)
        } else if mask & IN_ATTRIB != 0 {
            Some(FileEvent::AttributeChanged)
        } else {
            None
        }
    }
}

#[async_trait]
impl FileWatcher for InotifyWatcher {
    async fn watch(&self, path: &Path) -> Result<()> {
        if !self.active.load(Ordering::Acquire) {
            return Err(MonoError::FileWatch("watcher is not active".to_string()));
        }

        let canonical = path.canonicalize().map_err(|e| {
            MonoError::FileWatch(format!("failed to canonicalize {}: {}", path.display(), e))
        })?;

        let mut watches = self.watches.lock().await;
        if watches.contains_key(&canonical) {
            return Ok(());
        }

        let wd = self.fd.get_ref().add_watch(&canonical)?;
        watches.insert(canonical, wd);
        Ok(())
    }

    async fn unwatch(&self, path: &Path) -> Result<()> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        let mut watches = self.watches.lock().await;
        if let Some(wd) = watches.remove(&canonical) {
            self.fd.get_ref().remove_watch(wd)?;
        }
        Ok(())
    }

    async fn next_event(&self) -> Result<Option<FileEvent>> {
        if !self.active.load(Ordering::Acquire) {
            return Ok(None);
        }

        let mut buf = [0u8; 4096];

        loop {
            if !self.active.load(Ordering::Acquire) {
                return Ok(None);
            }

            let mut guard = self
                .fd
                .readable()
                .await
                .map_err(|e| MonoError::FileWatch(format!("readable failed: {}", e)))?;

            let result = guard.try_io(|inner| {
                let fd = inner.get_ref().as_raw_fd();
                let n = unsafe {
                    libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len())
                };
                if n < 0 {
                    Err(std::io::Error::last_os_error())
                } else {
                    Ok(n as usize)
                }
            });

            match result {
                Ok(Ok(n)) if n >= INOTIFY_EVENT_SIZE => {
                    let event_ptr = buf.as_ptr() as *const InotifyEventRaw;
                    let event = unsafe { &*event_ptr };
                    if let Some(file_event) = Self::parse_event(event.mask) {
                        return Ok(Some(file_event));
                    }
                }
                Ok(Ok(_)) => continue,
                Ok(Err(e)) => {
                    return Err(MonoError::FileWatch(format!("read failed: {}", e)));
                }
                Err(_would_block) => continue,
            }
        }
    }

    async fn stop(&self) -> Result<()> {
        self.active.store(false, Ordering::Release);

        let mut watches = self.watches.lock().await;
        for (_, wd) in watches.drain() {
            let _ = self.fd.get_ref().remove_watch(wd);
        }

        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_inotify_watcher_creation() {
        let watcher = InotifyWatcher::new();
        assert!(watcher.is_ok());
        assert!(watcher.unwrap().is_active());
    }

    #[tokio::test]
    async fn test_watch_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "initial").unwrap();

        let watcher = InotifyWatcher::new().unwrap();
        let result = watcher.watch(&file_path).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_watch_nonexistent_file() {
        let watcher = InotifyWatcher::new().unwrap();
        let result = watcher.watch(Path::new("/nonexistent/path/file.txt")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stop_watcher() {
        let watcher = InotifyWatcher::new().unwrap();
        assert!(watcher.is_active());

        watcher.stop().await.unwrap();
        assert!(!watcher.is_active());
    }

    #[tokio::test]
    async fn test_parse_events() {
        assert_eq!(
            InotifyWatcher::parse_event(IN_MODIFY),
            Some(FileEvent::Modified)
        );
        assert_eq!(
            InotifyWatcher::parse_event(IN_CREATE),
            Some(FileEvent::Created)
        );
        assert_eq!(
            InotifyWatcher::parse_event(IN_DELETE),
            Some(FileEvent::Deleted)
        );
        assert_eq!(
            InotifyWatcher::parse_event(IN_CLOSE_WRITE),
            Some(FileEvent::ClosedWrite)
        );
        assert_eq!(
            InotifyWatcher::parse_event(IN_MOVED_TO),
            Some(FileEvent::Renamed)
        );
    }
}
