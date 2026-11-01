//! Twilight head-to-head benchmark implementations.

use crate::metrics::TierRunResult;
use crate::server::{MockGateway, MockGatewayConfig};
use futures_util::StreamExt as FuturesStreamExt;
use std::hint::black_box;
use std::time::Instant;
use twilight_gateway::{parse, ConfigBuilder, Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt};

/// Twilight Raw Driver: End-to-End WebSocket + zstd-stream decompression + message framing.
pub async fn run_twilight_raw(event_count: usize) -> Result<TierRunResult, Box<dyn std::error::Error + Send + Sync>> {
    let gateway = MockGateway::start(MockGatewayConfig {
        event_count,
        guild_members: 100,
        zstd_level: 3,
        uncompressed: false,
        ..Default::default()
    })
    .await?;

    let config = ConfigBuilder::new("twilight-bench-token".to_string(), Intents::all())
        .proxy_url(gateway.url())
        .build();

    let mut shard = Shard::with_config(ShardId::ONE, config);

    // 1. Wait for HELLO (op 10) and READY (op 0)
    let mut ready_seen = false;
    while let Some(msg_res) = FuturesStreamExt::next(&mut shard).await {
        let msg = msg_res?;
        black_box(&msg);
        if ready_seen {
            break;
        }
        ready_seen = true;
    }

    // 2. Measure burst of N dispatch messages
    let burst_start = Instant::now();
    let mut latencies_us = Vec::with_capacity(event_count);
    let mut count = 0;

    while count < event_count {
        let t0 = Instant::now();
        let msg = match FuturesStreamExt::next(&mut shard).await {
            Some(Ok(m)) => m,
            _ => break,
        };
        black_box(msg);
        let elapsed_us = t0.elapsed().as_nanos() as f64 / 1_000.0;
        latencies_us.push(elapsed_us);
        count += 1;
    }

    let duration = burst_start.elapsed();
    gateway.shutdown().await;

    Ok(TierRunResult::from_timings(
        "Twilight Raw Driver (twilight-gateway)".to_string(),
        "End-to-End (zstd-stream)",
        count,
        duration,
        latencies_us,
    ))
}

/// Twilight Typed Model: End-to-End WebSocket + zstd-stream + strongly-typed twilight Event parsing.
pub async fn run_twilight_typed(event_count: usize) -> Result<TierRunResult, Box<dyn std::error::Error + Send + Sync>> {
    let gateway = MockGateway::start(MockGatewayConfig {
        event_count,
        guild_members: 100,
        zstd_level: 3,
        uncompressed: false,
        ..Default::default()
    })
    .await?;

    let config = ConfigBuilder::new("twilight-bench-token".to_string(), Intents::all())
        .proxy_url(gateway.url())
        .build();

    let mut shard = Shard::with_config(ShardId::ONE, config);

    // Wait for READY
    while let Some(res) = shard.next_event(EventTypeFlags::all()).await {
        let event = res?;
        if matches!(event, Event::Ready(_)) {
            break;
        }
    }

    // Measure burst of N dispatch events
    let burst_start = Instant::now();
    let mut latencies_us = Vec::with_capacity(event_count);
    let mut count = 0;

    while count < event_count {
        let t0 = Instant::now();
        let event = match shard.next_event(EventTypeFlags::all()).await {
            Some(Ok(e)) => e,
            _ => break,
        };
        black_box(event);
        let elapsed_us = t0.elapsed().as_nanos() as f64 / 1_000.0;
        latencies_us.push(elapsed_us);
        count += 1;
    }

    let duration = burst_start.elapsed();
    gateway.shutdown().await;

    Ok(TierRunResult::from_timings(
        "Twilight Typed (twilight-gateway + model)".to_string(),
        "End-to-End (zstd-stream)",
        count,
        duration,
        latencies_us,
    ))
}

/// Parse-Only: Twilight in-memory strongly-typed deserialization using `twilight_gateway::parse`.
pub fn run_twilight_parse_only(raw_json_strings: &[String]) -> TierRunResult {
    let mut latencies_us = Vec::with_capacity(raw_json_strings.len());
    let burst_start = Instant::now();

    for frame in raw_json_strings {
        let t0 = Instant::now();
        let parsed = parse(frame.clone(), EventTypeFlags::all())
            .expect("twilight parse failed")
            .expect("event not filtered");
        black_box(parsed);
        latencies_us.push(t0.elapsed().as_nanos() as f64 / 1_000.0);
    }

    let duration = burst_start.elapsed();
    TierRunResult::from_timings(
        "Twilight Model (twilight-model via parse)".to_string(),
        "Parse-Only (In-Memory)",
        raw_json_strings.len(),
        duration,
        latencies_us,
    )
}
