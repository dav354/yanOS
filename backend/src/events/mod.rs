use std::path::PathBuf;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum ExternalEvent {
    ConfigChanged(PathBuf),
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
