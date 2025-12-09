use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::broadcast;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExternalEvent {
    // Config Changes
    ConfigChanged { path: PathBuf },

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
}

/// A simple broadcast bus for streaming external change events to subscribers.
#[derive(Clone, Debug)]
pub struct EventBus {
    sender: broadcast::Sender<ExternalEvent>,
}

impl EventBus {
    pub fn new(buffer: usize) -> Self {
        let (sender, _) = broadcast::channel(buffer);
        Self { sender }
    }

    pub fn sender(&self) -> broadcast::Sender<ExternalEvent> {
        self.sender.clone()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ExternalEvent> {
        self.sender.subscribe()
    }
}
