use crate::models::DemoEvent;
use tokio::sync::broadcast;

/// Event bus for broadcasting events to WebSocket clients
pub struct EventBus {
    sender: broadcast::Sender<DemoEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<DemoEvent> {
        self.sender.subscribe()
    }

    /// Publish an event to all subscribers
    pub fn publish(&self, event: DemoEvent) {
        // Ignore send errors (no subscribers)
        let _ = self.sender.send(event);
    }

    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1024)
    }
}
