use crate::error::Result;

pub trait Platform: Send + Sync {
    fn is_process_running(&self, pid: u32) -> bool;
    fn kill_process(&self, pid: u32) -> Result<()>;
    fn read_pid_file(&self, path: &std::path::Path) -> Result<Option<u32>>;
    fn write_pid_file(&self, path: &std::path::Path, pid: u32) -> Result<()>;
    fn remove_pid_file(&self, path: &std::path::Path) -> Result<()>;
    fn current_pid(&self) -> u32;
}
