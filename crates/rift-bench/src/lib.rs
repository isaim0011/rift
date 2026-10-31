//! # rift-bench
//!
//! Benchmark harness and loopback mock Discord v10 server for measuring
//! `rift`'s end-to-end performance.
//!
//! ## Modules
//! - [`data`]: Realistic deterministic gateway event generators (`READY`, `GUILD_CREATE`, `MESSAGE_CREATE`).
//! - [`server`]: Local loopback (`127.0.0.1:0`) Mock Discord Gateway (WebSocket + `zstd-stream`) and REST server.
//! - [`harness`]: End-to-end benchmark harness measuring ingestion throughput, latency percentiles, and CPU time.
//! - [`compare`]: Cross-library & architecture comparison harness (Rift vs. discord.js, discord.py, twilight, serenity).

#![warn(missing_docs)]

pub mod compare;
pub mod data;
pub mod harness;
pub mod server;

pub use compare::{CompareConfig, CompareHarness, ComparisonReport, MicroBenchmarkResult, TierResult};
pub use harness::{Harness, HarnessConfig, HarnessReport, MultiRunSummary};
pub use server::{
    MockDiscordServer, MockGateway, MockGatewayConfig, MockGatewayStats, MockRestServer,
};

#[cfg(test)]
mod tests {
    use super::*;
    use rift_cache::{Cache, CacheConfig};
    use rift_gateway::IdentifyBudget;
    use rift_model::GatewayEvent;
    use rift_runtime::{event_channel, run_shard, Command, EventBufferPolicy, ShardConfig};
    use std::sync::atomic::Ordering;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    #[tokio::test]
    async fn test_mock_gateway_shard_end_to_end() {
        const EVENT_COUNT: usize = 100;

        let gateway = MockGateway::start(MockGatewayConfig {
            heartbeat_interval_ms: 41_250,
            event_count: EVENT_COUNT,
            guild_members: 25,
            zstd_level: 3,
            custom_events: None,
            uncompressed: false,
        })
        .await
        .expect("start mock gateway");

        let intents = rift_core::Intents::from_bits(rift_core::Intents::ALL_KNOWN).unwrap_or_default();
        let mut config = ShardConfig::new("test-token", intents);
        config.gateway_url = gateway.url();
        config.event_buffer_policy = EventBufferPolicy::Unbounded;

        let (events_tx, mut events_rx) = event_channel(config.event_buffer_policy);
        let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(64);
        let budget = Arc::new(Mutex::new(IdentifyBudget::from_session_start_limit(
            1000, 1000, 86_400_000, 0, 0,
        )));
        let mut cache = Cache::new(CacheConfig::all());

        let shard_task = tokio::spawn(async move {
            run_shard(config, budget, events_tx, cmd_rx).await
        });

        // 1. Receive and verify READY
        let ready_event = tokio::time::timeout(Duration::from_secs(5), events_rx.recv())
            .await
            .expect("timeout waiting for READY")
            .expect("event stream closed");
        assert_eq!(ready_event.name, "READY");
        let typed_ready = GatewayEvent::parse(&ready_event.name, &ready_event.data)
            .expect("parse typed READY");
        cache.update(&typed_ready);

        // 2. Receive and ingest 100 dispatch events (GUILD_CREATE + MESSAGE_CREATEs)
        let mut received = 0;
        let mut last_seq = ready_event.seq;

        while received < EVENT_COUNT {
            let event = tokio::time::timeout(Duration::from_secs(5), events_rx.recv())
                .await
                .expect("timeout waiting for event")
                .expect("event stream closed");

            assert!(
                event.seq > last_seq,
                "sequence numbers must be strictly increasing: {} > {}",
                event.seq,
                last_seq
            );
            last_seq = event.seq;

            let typed = GatewayEvent::parse(&event.name, &event.data)
                .unwrap_or_else(|e| panic!("failed to parse {}: {e}", event.name));
            cache.update(&typed);
            received += 1;
        }

        assert_eq!(received, EVENT_COUNT);

        // 3. Clean Shutdown: send Command::Shutdown
        cmd_tx.send(Command::Shutdown).await.expect("send shutdown command");
        let shard_res = tokio::time::timeout(Duration::from_secs(5), shard_task)
            .await
            .expect("timeout waiting for shard shutdown")
            .expect("shard task panicked");
        assert!(shard_res.is_ok(), "shard must shut down cleanly: {:?}", shard_res);

        // 4. Assert Mock Gateway stats
        let stats = gateway.stats();
        for _ in 0..100 {
            if stats.clean_shutdown_seen.load(Ordering::SeqCst) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(stats.identifies_received.load(Ordering::SeqCst), 1);
        assert_eq!(stats.events_streamed.load(Ordering::SeqCst), EVENT_COUNT as u64);
        assert!(stats.clean_shutdown_seen.load(Ordering::SeqCst));

        // 5. Verify cache state
        assert_eq!(cache.stats().guilds, 1);
        assert!(cache.stats().messages > 0);

        gateway.shutdown().await;
    }

    #[tokio::test]
    async fn test_harness_run_and_metrics() {
        let report = Harness::run(HarnessConfig {
            event_count: 50,
            guild_members: 10,
            cache_config: CacheConfig::all(),
            zstd_level: 3,
            token: "test-token".to_string(),
        })
        .await
        .expect("harness run");

        assert_eq!(report.total_events, 50);
        assert!(report.time_to_ready > Duration::ZERO);
        assert!(report.total_duration > Duration::ZERO);
        assert!(report.events_per_second > 0.0);
        assert!(report.p50_latency_ns > 0);
        assert!(report.p99_latency_ns >= report.p50_latency_ns);

        let reports = vec![report.clone(), report.clone(), report];
        let summary = Harness::aggregate(&reports).expect("aggregate summary");
        let table = summary.render_markdown_table();
        assert!(table.contains("| **Time to READY** |"));
        assert!(table.contains("| **Events / sec** |"));
        assert!(table.contains("| **p50 Latency** |"));
    }

    #[tokio::test]
    async fn test_mock_discord_server_gateway_and_rest() {
        let server = MockDiscordServer::start(MockGatewayConfig {
            event_count: 10,
            ..Default::default()
        })
        .await
        .expect("start mock discord server");

        assert!(server.gateway_url().starts_with("ws://127.0.0.1:"));
        assert!(server.rest_url().starts_with("http://127.0.0.1:"));

        // Query mock REST server over loopback TCP
        let mut stream = TcpStream::connect(server.rest.addr()).await.expect("connect rest");
        stream
            .write_all(b"GET /api/v10/gateway/bot HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .expect("write GET");

        let mut buf = [0u8; 1024];
        let n = stream.read(&mut buf).await.expect("read response");
        let response = String::from_utf8_lossy(&buf[..n]);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains(&server.gateway_url()));

        server.shutdown().await;
    }
}
