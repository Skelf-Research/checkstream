//! Metrics collection and reporting

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Metrics collector for CheckStream performance monitoring
#[derive(Clone)]
pub struct MetricsCollector {
    inner: Arc<MetricsInner>,
}

struct MetricsInner {
    total_requests: AtomicU64,
    total_tokens: AtomicU64,
    policy_triggers: AtomicU64,
    total_latency_us: AtomicU64,
    classifier_latency_us: AtomicU64,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            inner: Arc::new(MetricsInner {
                total_requests: AtomicU64::new(0),
                total_tokens: AtomicU64::new(0),
                policy_triggers: AtomicU64::new(0),
                total_latency_us: AtomicU64::new(0),
                classifier_latency_us: AtomicU64::new(0),
            }),
        }
    }

    /// Record a request
    pub fn record_request(&self) {
        self.inner.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Record tokens processed
    pub fn record_tokens(&self, count: u64) {
        self.inner.total_tokens.fetch_add(count, Ordering::Relaxed);
    }

    /// Record a policy trigger
    pub fn record_policy_trigger(&self) {
        self.inner.policy_triggers.fetch_add(1, Ordering::Relaxed);
    }

    /// Record latency
    pub fn record_latency(&self, latency_us: u64) {
        self.inner
            .total_latency_us
            .fetch_add(latency_us, Ordering::Relaxed);
    }

    /// Record classifier latency
    pub fn record_classifier_latency(&self, latency_us: u64) {
        self.inner
            .classifier_latency_us
            .fetch_add(latency_us, Ordering::Relaxed);
    }

    /// Get current metrics snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            total_requests: self.inner.total_requests.load(Ordering::Relaxed),
            total_tokens: self.inner.total_tokens.load(Ordering::Relaxed),
            policy_triggers: self.inner.policy_triggers.load(Ordering::Relaxed),
            total_latency_us: self.inner.total_latency_us.load(Ordering::Relaxed),
            classifier_latency_us: self.inner.classifier_latency_us.load(Ordering::Relaxed),
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of current metrics
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub total_requests: u64,
    pub total_tokens: u64,
    pub policy_triggers: u64,
    pub total_latency_us: u64,
    pub classifier_latency_us: u64,
}

impl MetricsSnapshot {
    /// Calculate average latency per request
    pub fn avg_latency_us(&self) -> u64 {
        if self.total_requests == 0 {
            0
        } else {
            self.total_latency_us / self.total_requests
        }
    }

    /// Calculate average classifier latency per request
    pub fn avg_classifier_latency_us(&self) -> u64 {
        if self.total_requests == 0 {
            0
        } else {
            self.classifier_latency_us / self.total_requests
        }
    }

    /// Calculate policy trigger rate
    pub fn trigger_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.policy_triggers as f64 / self.total_requests as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collection() {
        let metrics = MetricsCollector::new();

        metrics.record_request();
        metrics.record_tokens(100);
        metrics.record_policy_trigger();
        metrics.record_latency(5000);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.total_requests, 1);
        assert_eq!(snapshot.total_tokens, 100);
        assert_eq!(snapshot.policy_triggers, 1);
        assert_eq!(snapshot.avg_latency_us(), 5000);
    }
}
