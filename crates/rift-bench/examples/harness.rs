//! End-to-end benchmark harness runner.
//!
//! Runs 5 iterations of a 5,000-event compressed stream over loopback WebSocket,
//! discards warm-up, and prints a clean markdown table of median/min/max `time_to_ready`,
//! `events/sec`, `CPU/1000 events`, and `p50`/`p99` latency.
//!
//! Run with: `cargo run --release -p rift-bench --example harness`

use rift_bench::{Harness, HarnessConfig, HarnessReport};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("================================================================================");
    println!("  rift End-to-End Discord Gateway + Cache Ingestion Benchmark Harness");
    println!("  Protocol: Discord v10 Gateway (WebSocket + zstd-stream) + Cache Ingestion");
    println!("  Configuration: 5,000 events per iteration, release mode, 127.0.0.1:0 loopback");
    println!("================================================================================\n");

    let config = HarnessConfig {
        event_count: 5_000,
        guild_members: 100,
        cache_config: rift_cache::CacheConfig::all(),
        zstd_level: 3,
        token: "rift-benchmark-bot-token".to_string(),
    };

    println!("[1/6] Running warm-up iteration (5,000 events)...");
    let warmup_start = std::time::Instant::now();
    let warmup_report = Harness::run(config.clone()).await?;
    println!(
        "      Warm-up completed in {:.2?} ({:.0} ev/s). Discarding warm-up run.\n",
        warmup_start.elapsed(),
        warmup_report.events_per_second
    );

    let mut reports: Vec<HarnessReport> = Vec::with_capacity(5);

    for i in 1..=5 {
        println!("[{}/6] Running measured iteration {i} (5,000 events)...", i + 1);
        let report = Harness::run(config.clone()).await?;
        println!(
            "      Run {i}: time_to_ready={:.2?}, events/sec={:.0}, p50={:.2} \u{03bc}s, p99={:.2} \u{03bc}s, CPU/1k={:.2?}",
            report.time_to_ready,
            report.events_per_second,
            report.p50_latency_ns as f64 / 1_000.0,
            report.p99_latency_ns as f64 / 1_000.0,
            report.cpu_time_per_1k,
        );
        reports.push(report);
    }

    println!("\nAggregating results across 5 measured iterations...\n");

    let summary = Harness::aggregate(&reports).expect("aggregated summary");

    println!("### Benchmark Results (5 iterations x 5,000 events)\n");
    println!("{}", summary.render_markdown_table());
    println!("\n*All metrics measured end-to-end on loopback: WebSocket frame -> zstd-stream decompress -> envelope parse -> typed GatewayEvent parse -> Cache update.*");

    Ok(())
}
