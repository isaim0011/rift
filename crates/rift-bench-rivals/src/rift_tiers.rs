//! Rift head-to-head benchmark implementations.

use crate::metrics::TierRunResult;
use crate::server::{MockGateway, MockGatewayConfig};
use rift_gateway::IdentifyBudget;
use rift_model::GatewayEvent;
use rift_runtime::{event_channel, run_shard, Command, EventBufferPolicy, ShardConfig};
use std::hint::black_box;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Tier 1: Rift Raw Driver (End-to-End WebSocket + zstd-stream + framing, zero payload parse).
pub async fn run_rift_raw(event_count: usize) -> Result<TierRunResult, Box<dyn std::error::Error + Send + Sync>> {
    let gateway = MockGateway::start(MockGatewayConfig {
        event_count,
        guild_members: 100,
        zstd_level: 3,
        uncompressed: false,
        ..Default::default()
    })
    .await?;

    let intents = rift_core::Intents::from_bits(rift_core::Intents::ALL_KNOWN).unwrap_or_default();
    let mut shard_config = ShardConfig::new("rift-bench-token", intents);
    shard_config.gateway_url = gateway.url();
    shard_config.event_buffer_policy = EventBufferPolicy::Unbounded;

    let (events_tx, mut events_rx) = event_channel(shard_config.event_buffer_policy);
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(64);
    let budget = Arc::new(Mutex::new(IdentifyBudget::from_session_start_limit(
        1000, 1000, 86_400_000, 0, 0,
    )));

    let shard_task = tokio::spawn(async move {
        run_shard(shard_config, budget, events_tx, cmd_rx).await
    });

    // Wait for READY
    while let Some(event) = events_rx.recv().await {
        if event.name == "READY" {
            break;
        }
    }

    let burst_start = Instant::now();
    let mut latencies_us = Vec::with_capacity(event_count);
    let mut count = 0;

    while count < event_count {
        let t0 = Instant::now();
        let event = match events_rx.recv().await {
            Some(e) => e,
            None => break,
        };
        black_box(&event.name);
        black_box(event.seq);
        let elapsed_us = t0.elapsed().as_nanos() as f64 / 1_000.0;
        latencies_us.push(elapsed_us);
        count += 1;
    }

    let duration = burst_start.elapsed();
    let _ = cmd_tx.send(Command::Shutdown).await;
    let _ = shard_task.await;
    gateway.shutdown().await;

    Ok(TierRunResult::from_timings(
        "Rift Raw Driver".to_string(),
        "End-to-End (zstd-stream)",
        count,
        duration,
        latencies_us,
    ))
}

/// Tier 2: Rift Typed Model (End-to-End WebSocket + zstd-stream + framing + typed GatewayEvent).
pub async fn run_rift_typed(event_count: usize) -> Result<TierRunResult, Box<dyn std::error::Error + Send + Sync>> {
    let gateway = MockGateway::start(MockGatewayConfig {
        event_count,
        guild_members: 100,
        zstd_level: 3,
        uncompressed: false,
        ..Default::default()
    })
    .await?;

    let intents = rift_core::Intents::from_bits(rift_core::Intents::ALL_KNOWN).unwrap_or_default();
    let mut shard_config = ShardConfig::new("rift-bench-token", intents);
    shard_config.gateway_url = gateway.url();
    shard_config.event_buffer_policy = EventBufferPolicy::Unbounded;

    let (events_tx, mut events_rx) = event_channel(shard_config.event_buffer_policy);
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(64);
    let budget = Arc::new(Mutex::new(IdentifyBudget::from_session_start_limit(
        1000, 1000, 86_400_000, 0, 0,
    )));

    let shard_task = tokio::spawn(async move {
        run_shard(shard_config, budget, events_tx, cmd_rx).await
    });

    // Wait for READY
    while let Some(event) = events_rx.recv().await {
        if event.name == "READY" {
            let _ = GatewayEvent::parse(&event.name, &event.data);
            break;
        }
    }

    let burst_start = Instant::now();
    let mut latencies_us = Vec::with_capacity(event_count);
    let mut count = 0;

    while count < event_count {
        let t0 = Instant::now();
        let event = match events_rx.recv().await {
            Some(e) => e,
            None => break,
        };
        let typed = GatewayEvent::parse(&event.name, &event.data)
            .unwrap_or_else(|e| panic!("parse failed: {e}"));
        black_box(typed);
        let elapsed_us = t0.elapsed().as_nanos() as f64 / 1_000.0;
        latencies_us.push(elapsed_us);
        count += 1;
    }

    let duration = burst_start.elapsed();
    let _ = cmd_tx.send(Command::Shutdown).await;
    let _ = shard_task.await;
    gateway.shutdown().await;

    Ok(TierRunResult::from_timings(
        "Rift Typed Model".to_string(),
        "End-to-End (zstd-stream)",
        count,
        duration,
        latencies_us,
    ))
}

/// Parse-Only: Rift in-memory strongly-typed deserialization of identical frames.
pub fn run_rift_parse_only(raw_events: &[(String, Box<serde_json::value::RawValue>)]) -> TierRunResult {
    let mut latencies_us = Vec::with_capacity(raw_events.len());
    let burst_start = Instant::now();

    for (name, data) in raw_events {
        let t0 = Instant::now();
        let parsed = GatewayEvent::parse(name, data).expect("rift parse failed");
        black_box(parsed);
        latencies_us.push(t0.elapsed().as_nanos() as f64 / 1_000.0);
    }

    let duration = burst_start.elapsed();
    TierRunResult::from_timings(
        "Rift Model".to_string(),
        "Parse-Only (In-Memory)",
        raw_events.len(),
        duration,
        latencies_us,
    )
}
