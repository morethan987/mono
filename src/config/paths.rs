use crate::error::{MonoError, Result};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

#[derive(Clone)]
pub struct MonoPaths {
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
    pub database: PathBuf,
    pub socket: PathBuf,
    pub pid_file: PathBuf,
    pub log_file: PathBuf,
}

impl MonoPaths {
    pub fn new() -> Result<Self> {
        let proj_dirs = ProjectDirs::from("", "", "mono").ok_or(MonoError::ProjectDirsNotFound)?;

        let data_dir = proj_dirs.data_dir().to_path_buf();
        let config_dir = proj_dirs.config_dir().to_path_buf();

        let socket = std::env::var("XDG_RUNTIME_DIR")
            .map(|dir| PathBuf::from(dir).join("mono.sock"))
            .unwrap_or_else(|_| data_dir.join("mono.sock"));

        Ok(Self {
            database: data_dir.join("mono.db"),
            pid_file: data_dir.join("mono.pid"),
            log_file: data_dir.join("mono.log"),
            socket,
            data_dir,
            config_dir,
        })
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.data_dir).map_err(|e| MonoError::DirectoryCreation {
            path: self.data_dir.clone(),
            source: e,
        })?;

        fs::create_dir_all(&self.config_dir).map_err(|e| MonoError::DirectoryCreation {
            path: self.config_dir.clone(),
            source: e,
        })?;

        if let Some(parent) = self.socket.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| MonoError::DirectoryCreation {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
            }
        }

        Ok(())
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }
}

impl Default for MonoPaths {
    fn default() -> Self {
        Self::new().expect("Failed to initialize paths")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths_creation() {
        let paths = MonoPaths::new().unwrap();
        assert!(paths.database.ends_with("mono.db"));
        assert!(paths.pid_file.ends_with("mono.pid"));
        assert!(paths.socket.ends_with("mono.sock"));
    }
}
