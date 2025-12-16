//! Filesystem watcher for configuration file changes.
//!
//! Monitors system configuration files and publishes change events
//! to the EventBus for UI refresh.

use std::path::{Path, PathBuf};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tracing::{debug, error, info};

use crate::events::{EventBus, ExternalEvent};

/// Starts a blocking filesystem watcher on the given path and forwards events into the broadcast bus.
pub async fn start_filesystem_watcher(
    paths: &[PathBuf],
    bus: EventBus,
) -> notify::Result<Option<RecommendedWatcher>> {
    if paths.is_empty() {
        info!(target: "yanos::watcher", "Filesystem watcher disabled (no paths configured)");
        return Ok(None);
    }

    debug!(target: "yanos::watcher", path_count = paths.len(), "Initializing filesystem watcher");

    // notify requires a blocking callback; we forward into the async channel.
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| match res {
        Ok(event) => {
            debug!(target: "yanos::watcher", kind = ?event.kind, paths = ?event.paths, "Filesystem event");
            if matches!(
                event.kind,
                EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
            ) {
                let target = event.paths.first().cloned();
                if let Some(p) = target {
                    bus.publish(ExternalEvent::ConfigChanged { path: p.clone() });
                    info!(target: "yanos::watcher", path = ?p, "External config change detected");
                }
            }
        }
        Err(err) => {
            error!(target: "yanos::watcher", error = ?err, "Watcher error");
        }
    })?;

    let mut watched_any = false;

    for path in paths {
        if !path.exists() {
            info!(
                target: "yanos::watcher",
                path = %path.display(),
                "Skipping watch (path does not exist)"
            );
            continue;
        }

        match watcher.watch(Path::new(path), RecursiveMode::NonRecursive) {
            Ok(_) => {
                watched_any = true;
                info!(target: "yanos::watcher", path = %path.display(), "Watching path");
            }
            Err(err) => {
                error!(
                    target: "yanos::watcher",
                    path = %path.display(),
                    error = ?err,
                    "Failed to watch path"
                );
            }
        }
    }

    if watched_any {
        Ok(Some(watcher))
    } else {
        info!(target: "yanos::watcher", "No existing paths to watch; watcher disabled");
        Ok(None)
    }
}
