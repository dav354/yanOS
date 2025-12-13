use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use time::OffsetDateTime;
use tokio::sync::broadcast;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExternalEvent {
    // Config Changes
    ConfigChanged {
        #[schema(value_type = String)]
        path: PathBuf,
    },

    // Service Events
    ServiceStarted { fmri: String },
    ServiceStopped { fmri: String },
    ServiceFailed { fmri: String },

    // ZFS Events
    DatasetCreated { name: String },
    DatasetDestroyed { name: String },

    // Network
    LinkUp { name: String },
    LinkDown { name: String },

    // System logs (e.g., /var/adm/messages lines)
    SystemLog { line: String },

    // Task Events
    TaskStarted {
        id: String,
        name: String,
        started_at: String,
    },
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
