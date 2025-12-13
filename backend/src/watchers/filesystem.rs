use std::path::{Path, PathBuf};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tracing::{error, info};

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

    // notify requires a blocking callback; we forward into the async channel.
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| match res {
        Ok(event) => {
            if matches!(
                event.kind,
                EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
            ) {
                let target = event.paths.get(0).cloned();
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

    for path in paths {
        watcher.watch(Path::new(path), RecursiveMode::NonRecursive)?;
        info!(target: "yanos::watcher", path = %path.display(), "Watching path");
    }

    Ok(Some(watcher))
}
