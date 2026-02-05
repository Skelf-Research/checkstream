use crate::models::TrafficState;
use parking_lot::RwLock;
use tokio::sync::oneshot;

/// Controls traffic generation
pub struct TrafficController {
    inner: RwLock<ControllerInner>,
}

struct ControllerInner {
    state: TrafficState,
    stop_sender: Option<oneshot::Sender<()>>,
}

impl TrafficController {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(ControllerInner {
                state: TrafficState::Stopped,
                stop_sender: None,
            }),
        }
    }

    /// Get current state
    pub fn state(&self) -> TrafficState {
        self.inner.read().state
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.inner.read().state == TrafficState::Running
    }

    /// Start traffic generation, returns a receiver that signals when to stop
    pub fn start(&self) -> Option<oneshot::Receiver<()>> {
        let mut inner = self.inner.write();
        if inner.state == TrafficState::Running {
            return None;
        }

        let (tx, rx) = oneshot::channel();
        inner.stop_sender = Some(tx);
        inner.state = TrafficState::Running;
        Some(rx)
    }

    /// Stop traffic generation
    pub fn stop(&self) {
        let mut inner = self.inner.write();
        if let Some(sender) = inner.stop_sender.take() {
            let _ = sender.send(());
        }
        inner.state = TrafficState::Stopped;
    }

    /// Pause traffic generation
    pub fn pause(&self) {
        let mut inner = self.inner.write();
        if inner.state == TrafficState::Running {
            inner.state = TrafficState::Paused;
        }
    }

    /// Resume traffic generation
    pub fn resume(&self) {
        let mut inner = self.inner.write();
        if inner.state == TrafficState::Paused {
            inner.state = TrafficState::Running;
        }
    }
}

impl Default for TrafficController {
    fn default() -> Self {
        Self::new()
    }
}
