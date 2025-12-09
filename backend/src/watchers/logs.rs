use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use std::time::Duration;

use tracing::{error, info};

use crate::events::{EventBus, ExternalEvent};

/// Tails a system log file (e.g., /var/adm/messages) and forwards new lines to the event bus.
pub fn start_system_log_watcher(
    path: &Path,
    bus: EventBus,
) -> std::io::Result<tokio::task::JoinHandle<()>> {
    let path = path.to_path_buf();
    let log_path = path.clone();
    let handle = tokio::task::spawn_blocking(move || {
        let sleep = Duration::from_millis(500);
        let file = match File::open(&path) {
            Ok(f) => f,
            Err(e) => {
                error!(target: "zos::logs", error = ?e, path = ?path, "Failed to open system log file");
                return;
            }
        };

        let mut reader = BufReader::new(file);

        // Preload last N lines for history
        let preload_limit = 500;
        let mut preload: VecDeque<String> = VecDeque::with_capacity(preload_limit);
        let mut buf = String::new();
        loop {
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
                    error!(target: "zos::logs", error = ?e, path = ?path, "Error preloading log file");
                    break;
                }
            }
        }

        for line in preload {
            bus.publish(ExternalEvent::SystemLog { line });
        }

        if let Err(e) = reader.get_mut().seek(SeekFrom::End(0)) {
            error!(target: "zos::logs", error = ?e, path = ?path, "Failed to seek to end of log file");
        }

        let mut buf = String::new();
        loop {
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
                    error!(target: "zos::logs", error = ?e, path = ?path, "Error reading log file");
                    std::thread::sleep(sleep);
                }
            }
        }
    });

    info!(target: "zos::logs", path = ?log_path, "System log watcher started");
    Ok(handle)
}
