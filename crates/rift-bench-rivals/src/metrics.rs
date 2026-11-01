//! Benchmark result structures, latency percentile math, and multi-run aggregation.

use std::time::Duration;

/// Result of a single execution of a benchmark tier.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct TierRunResult {
    pub name: String,
    pub category: &'static str,
    pub events_processed: usize,
    pub duration: Duration,
    pub events_per_sec: f64,
    pub p50_us: f64,
    pub p95_us: f64,
    pub p99_us: f64,
}

impl TierRunResult {
    /// Computes summary metrics from raw per-event latency timings.
    pub fn from_timings(
        name: String,
        category: &'static str,
        count: usize,
        duration: Duration,
        mut latencies_us: Vec<f64>,
    ) -> Self {
        latencies_us.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p50_us = if !latencies_us.is_empty() {
            latencies_us[latencies_us.len() * 50 / 100]
        } else {
            0.0
        };
        let p95_us = if !latencies_us.is_empty() {
            latencies_us[latencies_us.len() * 95 / 100]
        } else {
            0.0
        };
        let p99_us = if !latencies_us.is_empty() {
            latencies_us[latencies_us.len() * 99 / 100]
        } else {
            0.0
        };
        let events_per_sec = if duration.as_secs_f64() > 0.0 {
            count as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        Self {
            name,
            category,
            events_processed: count,
            duration,
            events_per_sec,
            p50_us,
            p95_us,
            p99_us,
        }
    }
}

/// Aggregates multiple runs (e.g. 5 runs) and picks the median run by throughput.
pub fn select_median_run(mut runs: Vec<TierRunResult>) -> TierRunResult {
    assert!(!runs.is_empty(), "runs must not be empty");
    runs.sort_by(|a, b| {
        a.events_per_sec
            .partial_cmp(&b.events_per_sec)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let median_idx = runs.len() / 2;
    runs.swap_remove(median_idx)
}
