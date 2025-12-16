//! System log file watcher for real-time log streaming.
//!
//! This module tails system log files (like /var/adm/messages on illumos)
//! and publishes new log lines to the EventBus for streaming to the UI.
//!
//! # Behavior
//! - On startup, preloads the last 500 lines for immediate history
//! - Seeks to end of file after preload
//! - Polls for new lines every 500ms
//! - Automatically reopens the file if it's rotated or truncated
//!
//! # Thread Model
//! Runs on a blocking thread via `spawn_blocking` since file I/O
//! should not block the async runtime.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use tracing::{error, info};

use crate::events::{EventBus, ExternalEvent};

/// Handle for controlling the log watcher task.
pub struct LogWatcherHandle {
    handle: tokio::task::JoinHandle<()>,
    shutdown: Arc<AtomicBool>,
}

impl LogWatcherHandle {
    /// Aborts the log watcher task.
    pub fn abort(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
        self.handle.abort();
    }
}

/// Starts a background task that tails a log file and publishes to the event bus.
///
/// # Arguments
/// * `path` - Path to the log file (e.g., /var/adm/messages)
/// * `bus` - EventBus to publish SystemLog events to
///
/// # Returns
/// LogWatcherHandle for the blocking task, or error if file cannot be opened.
pub fn start_system_log_watcher(
    path: &Path,
    bus: EventBus,
) -> Result<LogWatcherHandle, crate::error::AppError> {
    let path = path.to_path_buf();
    let log_path = path.clone();

    let mut first_file: Option<File> = None;
    let mut last_err: Option<std::io::Error> = None;
    for attempt in 1..=3 {
        match File::open(&path) {
            Ok(file) => {
                first_file = Some(file);
                break;
            }
            Err(err) => {
                last_err = Some(err);
                if attempt < 3 {
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                }
            }
        }
    }

    let file = first_file.ok_or_else(|| {
        let err = last_err
            .unwrap_or_else(|| std::io::Error::other("unknown log error"));
        crate::error::AppError::ServiceUnavailable(format!(
            "Failed to open system log file {}: {}",
            path.display(),
            err
        ))
    })?;

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    let handle = tokio::task::spawn_blocking(move || {
        let sleep = Duration::from_millis(500);
        let retry_delay = Duration::from_secs(2);
        let mut first_run = true;
        let mut current_file = Some(file);

        while !shutdown_clone.load(Ordering::Relaxed) {
            let file_handle = match current_file.take() {
                Some(f) => f,
                None => match File::open(&path) {
                    Ok(f) => f,
                    Err(e) => {
                        error!(target: "yanos::logs", error = ?e, path = ?path, "Failed to reopen system log file");
                        std::thread::sleep(retry_delay);
                        continue;
                    }
                },
            };

            let mut reader = BufReader::new(file_handle);

            if first_run {
                // Preload last N lines for history
                let preload_limit = 500;
                let mut preload: VecDeque<String> = VecDeque::with_capacity(preload_limit);
                let mut buf = String::new();
                loop {
                    if shutdown_clone.load(Ordering::Relaxed) {
                        return;
                    }
                    buf.clear();
                    match reader.read_line(&mut buf) {
                        Ok(0) => break,
                        Ok(_) => {
                            let line = buf.trim_end_matches(&['\r', '\n'][..]).to_string();
                            if !line.is_empty() {
                                if preload.len() == preload_limit {
                                    let _ = preload.pop_front();
                                }
                                preload.push_back(line);
                            }
                        }
                        Err(e) => {
                            error!(target: "yanos::logs", error = ?e, path = ?path, "Error preloading log file");
                            break;
                        }
                    }
                }

                for line in preload {
                    bus.publish(ExternalEvent::SystemLog { line });
                }

                first_run = false;
            }

            if let Err(e) = reader.get_mut().seek(SeekFrom::End(0)) {
                error!(target: "yanos::logs", error = ?e, path = ?path, "Failed to seek to end of log file");
            }

            let mut buf = String::new();
            loop {
                if shutdown_clone.load(Ordering::Relaxed) {
                    return;
                }
                buf.clear();
                match reader.read_line(&mut buf) {
                    Ok(0) => {
                        std::thread::sleep(sleep);
                    }
                    Ok(_) => {
                        let line = buf.trim_end_matches(&['\r', '\n'][..]).to_string();
                        if line.is_empty() {
                            continue;
                        }
                        bus.publish(ExternalEvent::SystemLog { line: line.clone() });
                    }
                    Err(e) => {
                        error!(target: "yanos::logs", error = ?e, path = ?path, "Error reading log file, retrying");
                        break; // Break inner loop to reopen/retry
                    }
                }
            }

            std::thread::sleep(retry_delay);
        }
    });

    info!(target: "yanos::logs", path = ?log_path, "System log watcher started");
    Ok(LogWatcherHandle { handle, shutdown })
}
