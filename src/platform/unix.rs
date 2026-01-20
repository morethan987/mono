use crate::error::{MonoError, Result};
use crate::platform::traits::Platform;
use std::fs;
use std::path::Path;

pub struct UnixPlatform;

impl UnixPlatform {
    pub fn new() -> Self {
        Self
    }
}

impl Default for UnixPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for UnixPlatform {
    fn is_process_running(&self, pid: u32) -> bool {
        Path::new(&format!("/proc/{}", pid)).exists()
    }

    fn kill_process(&self, pid: u32) -> Result<()> {
        use std::process::Command;

        let status = Command::new("kill")
            .arg(pid.to_string())
            .status()
            .map_err(|e| MonoError::Platform(format!("Failed to kill process {}: {}", pid, e)))?;

        if status.success() {
            Ok(())
        } else {
            Err(MonoError::ProcessNotFound { pid })
        }
    }

    fn read_pid_file(&self, path: &Path) -> Result<Option<u32>> {
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(path).map_err(|e| MonoError::PidFileRead {
            path: path.to_path_buf(),
            source: e,
        })?;

        let pid: u32 = content
            .trim()
            .parse()
            .map_err(|_| MonoError::InvalidPid { content })?;

        Ok(Some(pid))
    }

    fn write_pid_file(&self, path: &Path, pid: u32) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| MonoError::DirectoryCreation {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        fs::write(path, pid.to_string()).map_err(|e| MonoError::PidFileWrite {
            path: path.to_path_buf(),
            source: e,
        })?;

        Ok(())
    }

    fn remove_pid_file(&self, path: &Path) -> Result<()> {
        if path.exists() {
            fs::remove_file(path).map_err(|e| MonoError::PidFileWrite {
                path: path.to_path_buf(),
                source: e,
            })?;
        }
        Ok(())
    }

    fn current_pid(&self) -> u32 {
        std::process::id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_pid_file_operations() {
        let platform = UnixPlatform::new();
        let dir = tempdir().unwrap();
        let pid_file = dir.path().join("test.pid");

        assert!(platform.read_pid_file(&pid_file).unwrap().is_none());

        platform.write_pid_file(&pid_file, 12345).unwrap();
        assert_eq!(platform.read_pid_file(&pid_file).unwrap(), Some(12345));

        platform.remove_pid_file(&pid_file).unwrap();
        assert!(platform.read_pid_file(&pid_file).unwrap().is_none());
    }

    #[test]
    fn test_current_process_running() {
        let platform = UnixPlatform::new();
        let current_pid = platform.current_pid();
        assert!(platform.is_process_running(current_pid));
    }
}
