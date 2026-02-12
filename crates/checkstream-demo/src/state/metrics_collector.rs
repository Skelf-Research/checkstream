use crate::models::{
    AggregatedStats, HeatmapData, MetricsSnapshot, RequestAction, RequestResult, TimelineData,
    TimelinePoint,
};
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};

const MAX_HISTORY_SIZE: usize = 10000;
const TIMELINE_WINDOW_MINUTES: i64 = 60;
const TIMELINE_BUCKET_SECONDS: i64 = 60;

/// Collects and aggregates metrics in real-time
pub struct MetricsCollector {
    inner: RwLock<MetricsInner>,
}

struct MetricsInner {
    /// Recent request results for calculations
    results: VecDeque<RequestResult>,
    /// Latency samples for percentile calculations
    latencies: VecDeque<f64>,
    /// Issue type counts
    issues_detected: HashMap<String, u64>,
    /// Rule trigger counts
    rules_triggered: HashMap<String, u64>,
    /// Timeline data points
    timeline: VecDeque<TimelinePoint>,
    /// Heatmap buckets: rule_name -> time_bucket -> count
    heatmap: HashMap<String, HashMap<i64, u64>>,
    /// Total counts
    total_requests: u64,
    blocked_count: u64,
    redacted_count: u64,
    passed_count: u64,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(MetricsInner {
                results: VecDeque::with_capacity(MAX_HISTORY_SIZE),
                latencies: VecDeque::with_capacity(MAX_HISTORY_SIZE),
                issues_detected: HashMap::new(),
                rules_triggered: HashMap::new(),
                timeline: VecDeque::with_capacity(TIMELINE_WINDOW_MINUTES as usize),
                heatmap: HashMap::new(),
                total_requests: 0,
                blocked_count: 0,
                redacted_count: 0,
                passed_count: 0,
            }),
        }
    }

    /// Record a request result
    pub fn record(&self, result: &RequestResult) {
        let mut inner = self.inner.write();

        // Update totals
        inner.total_requests += 1;
        match result.action {
            RequestAction::Block => inner.blocked_count += 1,
            RequestAction::Redact => inner.redacted_count += 1,
            RequestAction::Pass => inner.passed_count += 1,
        }

        // Record latency
        inner.latencies.push_back(result.latency_ms);
        if inner.latencies.len() > MAX_HISTORY_SIZE {
            inner.latencies.pop_front();
        }

        // Record issues
        for issue in &result.issues_detected {
            *inner
                .issues_detected
                .entry(issue.issue_type.clone())
                .or_insert(0) += 1;
        }

        // Record rules
        for rule in &result.triggered_rules {
            *inner.rules_triggered.entry(rule.clone()).or_insert(0) += 1;

            // Update heatmap
            let bucket = result.timestamp.timestamp() / TIMELINE_BUCKET_SECONDS;
            let rule_buckets = inner.heatmap.entry(rule.clone()).or_default();
            *rule_buckets.entry(bucket).or_insert(0) += 1;
        }

        // Store result
        inner.results.push_back(result.clone());
        if inner.results.len() > MAX_HISTORY_SIZE {
            inner.results.pop_front();
        }
    }

    /// Get current metrics snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        let inner = self.inner.read();
        let now = Utc::now();

        // Calculate requests per minute (last 60 seconds)
        let one_minute_ago = now - Duration::seconds(60);
        let recent_count = inner
            .results
            .iter()
            .filter(|r| r.timestamp > one_minute_ago)
            .count() as f64;

        let total = inner.total_requests as f64;
        let block_rate = if total > 0.0 {
            inner.blocked_count as f64 / total
        } else {
            0.0
        };
        let redact_rate = if total > 0.0 {
            inner.redacted_count as f64 / total
        } else {
            0.0
        };

        let avg_latency = if !inner.latencies.is_empty() {
            inner.latencies.iter().sum::<f64>() / inner.latencies.len() as f64
        } else {
            0.0
        };

        let p99_latency = self.calculate_percentile(&inner.latencies, 99.0);

        MetricsSnapshot {
            timestamp: now,
            requests_total: inner.total_requests,
            requests_per_minute: recent_count,
            block_rate,
            redact_rate,
            avg_latency_ms: avg_latency,
            p99_latency_ms: p99_latency,
            active_connections: 0, // Updated by server
        }
    }

    /// Get aggregated statistics
    pub fn stats(&self) -> AggregatedStats {
        let inner = self.inner.read();

        let total = inner.total_requests as f64;
        let block_rate = if total > 0.0 {
            inner.blocked_count as f64 / total
        } else {
            0.0
        };
        let redact_rate = if total > 0.0 {
            inner.redacted_count as f64 / total
        } else {
            0.0
        };

        let avg_latency = if !inner.latencies.is_empty() {
            inner.latencies.iter().sum::<f64>() / inner.latencies.len() as f64
        } else {
            0.0
        };

        // Calculate requests per minute
        let now = Utc::now();
        let one_minute_ago = now - Duration::seconds(60);
        let rpm = inner
            .results
            .iter()
            .filter(|r| r.timestamp > one_minute_ago)
            .count() as f64;

        AggregatedStats {
            requests_total: inner.total_requests,
            requests_per_minute: rpm,
            blocked_count: inner.blocked_count,
            redacted_count: inner.redacted_count,
            passed_count: inner.passed_count,
            block_rate,
            redact_rate,
            avg_latency_ms: avg_latency,
            p50_latency_ms: self.calculate_percentile(&inner.latencies, 50.0),
            p95_latency_ms: self.calculate_percentile(&inner.latencies, 95.0),
            p99_latency_ms: self.calculate_percentile(&inner.latencies, 99.0),
            issues_detected: inner.issues_detected.clone(),
            rules_triggered: inner.rules_triggered.clone(),
        }
    }

    /// Get heatmap data for visualization
    pub fn heatmap(&self, window_minutes: i64) -> HeatmapData {
        let inner = self.inner.read();
        let now = Utc::now();
        let start_bucket =
            (now - Duration::minutes(window_minutes)).timestamp() / TIMELINE_BUCKET_SECONDS;
        let end_bucket = now.timestamp() / TIMELINE_BUCKET_SECONDS;

        let rules: Vec<String> = inner.heatmap.keys().cloned().collect();
        let mut time_buckets = Vec::new();
        let mut values = Vec::new();

        for bucket in start_bucket..=end_bucket {
            time_buckets
                .push(DateTime::from_timestamp(bucket * TIMELINE_BUCKET_SECONDS, 0).unwrap_or(now));
        }

        for rule in &rules {
            let mut row = Vec::new();
            if let Some(buckets) = inner.heatmap.get(rule) {
                for bucket in start_bucket..=end_bucket {
                    row.push(*buckets.get(&bucket).unwrap_or(&0));
                }
            } else {
                row = vec![0; (end_bucket - start_bucket + 1) as usize];
            }
            values.push(row);
        }

        HeatmapData {
            rules,
            time_buckets,
            values,
        }
    }

    /// Get timeline data for charts
    pub fn timeline(&self, window_minutes: u32) -> TimelineData {
        let inner = self.inner.read();
        let now = Utc::now();
        let window = Duration::minutes(window_minutes as i64);
        let bucket_duration = Duration::seconds(TIMELINE_BUCKET_SECONDS);

        let mut points = Vec::new();
        let mut current = now - window;

        while current <= now {
            let bucket_end = current + bucket_duration;
            let bucket_results: Vec<_> = inner
                .results
                .iter()
                .filter(|r| r.timestamp >= current && r.timestamp < bucket_end)
                .collect();

            let count = bucket_results.len() as f64;
            let blocked = bucket_results
                .iter()
                .filter(|r| r.action == RequestAction::Block)
                .count() as f64;
            let block_rate = if count > 0.0 { blocked / count } else { 0.0 };
            let avg_latency = if count > 0.0 {
                bucket_results.iter().map(|r| r.latency_ms).sum::<f64>() / count
            } else {
                0.0
            };

            points.push(TimelinePoint {
                timestamp: current,
                requests_per_minute: count * (60.0 / TIMELINE_BUCKET_SECONDS as f64),
                block_rate,
                avg_latency_ms: avg_latency,
            });

            current = bucket_end;
        }

        TimelineData {
            points,
            window_minutes,
        }
    }

    fn calculate_percentile(&self, data: &VecDeque<f64>, percentile: f64) -> f64 {
        if data.is_empty() {
            return 0.0;
        }

        let mut sorted: Vec<f64> = data.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let index = (percentile / 100.0 * (sorted.len() - 1) as f64).round() as usize;
        sorted[index.min(sorted.len() - 1)]
    }

    /// Reset all metrics
    pub fn reset(&self) {
        let mut inner = self.inner.write();
        inner.results.clear();
        inner.latencies.clear();
        inner.issues_detected.clear();
        inner.rules_triggered.clear();
        inner.timeline.clear();
        inner.heatmap.clear();
        inner.total_requests = 0;
        inner.blocked_count = 0;
        inner.redacted_count = 0;
        inner.passed_count = 0;
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
