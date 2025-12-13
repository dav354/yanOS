//! Event bus for system-wide notifications.
//!
//! This module provides a broadcast-based event system for streaming
//! external changes to WebSocket clients. Events include:
//! - Configuration file changes (via notify watchers)
//! - SMF service state changes
//! - ZFS dataset operations
//! - Network link state changes
//! - System log entries
//! - Background task progress
//!
//! Events are timestamped and kept in a rolling history buffer for
//! new clients to receive recent events on connection.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use time::OffsetDateTime;
use tokio::sync::broadcast;
use utoipa::ToSchema;

/// External events that can be broadcast to UI clients.
/// Tagged enum serializes with a "type" field for easy client-side dispatch.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExternalEvent {
    // --- Configuration Changes ---
    /// A watched config file was modified externally
    ConfigChanged {
        #[schema(value_type = String)]
        path: PathBuf,
    },

    // --- SMF Service Events ---
    /// An SMF service transitioned to online state
    ServiceStarted { fmri: String },
    /// An SMF service transitioned to offline/disabled state
    ServiceStopped { fmri: String },
    /// An SMF service entered maintenance state
    ServiceFailed { fmri: String },

    // --- ZFS Events ---
    /// A new ZFS dataset was created
    DatasetCreated { name: String },
    /// A ZFS dataset was destroyed
    DatasetDestroyed { name: String },

    // --- Network Events ---
    /// A network link came up
    LinkUp { name: String },
    /// A network link went down
    LinkDown { name: String },

    // --- System Logging ---
    /// A new line from /var/adm/messages or similar system log
    SystemLog { line: String },

    // --- Background Task Events ---
    /// A long-running task started (e.g., package update check)
    TaskStarted {
        id: String,
        name: String,
        started_at: String,
    },
    /// A background task completed (success or failure)
    TaskCompleted {
        id: String,
        name: String,
        started_at: String,
        duration_ms: u64,
        status: String,
    },
}

/// A simple broadcast bus for streaming external change events to subscribers.
#[derive(Clone, Debug)]
pub struct EventBus {
    sender: broadcast::Sender<LoggedEvent>,
    history: Arc<Mutex<VecDeque<LoggedEvent>>>,
}

impl EventBus {
    pub fn new(buffer: usize) -> Self {
        let capacity = buffer.max(1000);
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            history: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
        }
    }

    pub fn sender(&self) -> broadcast::Sender<LoggedEvent> {
        self.sender.clone()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LoggedEvent> {
        self.sender.subscribe()
    }

    pub fn publish(&self, event: ExternalEvent) {
        let ts = OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339);
        let entry = LoggedEvent {
            ts: ts.unwrap_or_else(|_| "unknown".into()),
            event,
        };

        if let Ok(mut history) = self.history.lock() {
            if history.len() == history.capacity() {
                let _ = history.pop_front();
            }
            history.push_back(entry.clone());
        }

        let _ = self.sender.send(entry);
    }

    pub fn snapshot(&self, limit: usize) -> Vec<LoggedEvent> {
        if let Ok(history) = self.history.lock() {
            let len = history.len();
            let start = len.saturating_sub(limit);
            return history.iter().skip(start).cloned().collect();
        }
        Vec::new()
    }

    pub fn snapshot_before(&self, before: &str, limit: usize) -> Vec<LoggedEvent> {
        if let Ok(history) = self.history.lock() {
            let mut collected = Vec::new();
            for entry in history.iter().rev() {
                if entry.ts.as_str() < before {
                    collected.push(entry.clone());
                    if collected.len() == limit {
                        break;
                    }
                }
            }
            collected.reverse();
            return collected;
        }
        Vec::new()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[derive(ToSchema)]
pub struct LoggedEvent {
    pub ts: String,
    pub event: ExternalEvent,
}
