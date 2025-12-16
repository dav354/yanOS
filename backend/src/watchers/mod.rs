pub mod filesystem;
pub mod logs;
pub mod network;

pub use filesystem::start_filesystem_watcher;
pub use logs::{start_system_log_watcher, LogWatcherHandle};
pub use network::start_network_event_watcher;
