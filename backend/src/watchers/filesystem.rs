use std::path::Path;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tracing::{error, info};

use crate::events::{EventBus, ExternalEvent};

/// Starts a blocking filesystem watcher on the given path and forwards events into the broadcast bus.
pub async fn start_filesystem_watcher(
    path: &Path,
    bus: EventBus,
) -> notify::Result<RecommendedWatcher> {
    let path = path.to_path_buf();

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
                    info!(target: "zos::watcher", path = ?p, "External config change detected");
                }
            }
        }
        Err(err) => {
            error!(target: "zos::watcher", error = ?err, "Watcher error");
        }
    })?;

    watcher.watch(&path, RecursiveMode::Recursive)?;

    Ok(watcher)
}
