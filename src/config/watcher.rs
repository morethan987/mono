mod inotify;
mod traits;

pub use inotify::InotifyWatcher;
pub use traits::{FileEvent, FileWatcher};
