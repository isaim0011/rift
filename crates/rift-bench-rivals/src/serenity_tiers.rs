//! Serenity head-to-head benchmark implementations.

use crate::metrics::TierRunResult;
use serenity::model::event::Event;
use std::hint::black_box;
use std::time::Instant;

/// Parse-Only: Serenity in-memory strongly-typed deserialization of identical frames into `serenity::model::event::Event`.
pub fn run_serenity_parse_only(raw_json_frames: &[Vec<u8>]) -> TierRunResult {
    let mut latencies_us = Vec::with_capacity(raw_json_frames.len());
    let burst_start = Instant::now();

    for frame in raw_json_frames {
        let t0 = Instant::now();
        let parsed: Event = serde_json::from_slice(frame).expect("serenity Event deserialization failed");
        black_box(parsed);
        latencies_us.push(t0.elapsed().as_nanos() as f64 / 1_000.0);
    }

    let duration = burst_start.elapsed();
    TierRunResult::from_timings(
        "Serenity Model (serenity::model)".to_string(),
        "Parse-Only (In-Memory)",
        raw_json_frames.len(),
        duration,
        latencies_us,
    )
}
