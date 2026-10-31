//! Cross-Library Comparison Benchmark Binary.
//!
//! Executes apples-to-apples comparative benchmarks on the current machine,
//! evaluating `rift` against `discord.js` and `discord.py` models across identical
//! workloads (10,000 events, loopback WebSocket + zstd-stream, permissions, route normalization).
//!
//! # Usage
//! ```bash
//! cargo run --release -p rift-bench --example compare
//! ```
//!
//! Options:
//! - `--events <N>`: Number of events to stream per tier (default: 10,000)
//! - `--external-port <PORT>`: Bind MockDiscordServer to a fixed port for external scripts
//! - `--no-external`: Skip auto-invoking Node.js / Python interpreters
//! - `--json`: Output raw JSON report instead of Markdown

use rift_bench::{CompareConfig, CompareHarness, MockDiscordServer, MockGatewayConfig};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = env::args().collect();

    let mut event_count = 10_000;
    let mut external_port: Option<u16> = None;
    let mut run_external = true;
    let mut json_output = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--events" => {
                if let Some(val) = args.get(i + 1) {
                    if let Ok(n) = val.parse::<usize>() {
                        event_count = n;
                    }
                    i += 1;
                }
            }
            "--external-port" => {
                if let Some(val) = args.get(i + 1) {
                    if let Ok(p) = val.parse::<u16>() {
                        external_port = Some(p);
                    }
                    i += 1;
                }
            }
            "--no-external" => {
                run_external = false;
            }
            "--json" => {
                json_output = true;
            }
            "--help" | "-h" => {
                println!("Usage: cargo run --release -p rift-bench --example compare [OPTIONS]");
                println!();
                println!("Options:");
                println!("  --events <N>           Number of events per run (default: 10,000)");
                println!("  --external-port <PORT> Bind MockDiscordServer to fixed port for external runners");
                println!("  --no-external          Skip invoking local node/python interpreters");
                println!("  --json                 Print JSON report instead of Markdown table");
                println!("  --help, -h             Print this help message");
                return Ok(());
            }
            _ => {}
        }
        i += 1;
    }

    // If external-port is specified and no other test requested, start server and wait
    if let Some(port) = external_port {
        println!("================================================================================");
        println!("  MockDiscordServer Starting on Fixed Loopback Port: {port}");
        println!("  Speak Discord v10 Gateway Protocol (HELLO -> IDENTIFY -> READY -> {event_count} Events)");
        println!("================================================================================\n");

        let server = MockDiscordServer::start_on_port(
            port,
            port + 1,
            MockGatewayConfig {
                event_count,
                guild_members: 100,
                uncompressed: true, // Allow external vanilla ws/aiohttp to connect without zstd C++ dependency
                ..Default::default()
            },
        )
        .await?;

        println!("Gateway WebSocket URL: {}", server.gateway_url());
        println!("REST API Base URL:     {}", server.rest_url());
        println!("\nServer is running. Press Ctrl+C to terminate or connect external clients.\n");

        tokio::signal::ctrl_c().await?;
        println!("\nShutting down MockDiscordServer...");
        server.shutdown().await;
        return Ok(());
    }

    eprintln!("================================================================================");
    eprintln!("  rift Cross-Library & Architecture Comparison Benchmark Harness");
    eprintln!("  Workload: {event_count} Gateway v10 Frames + Permissions + Routes + State Cache");
    eprintln!("================================================================================\n");

    eprintln!("[1/5] Executing Tier 1: Rift Raw Driver (twilight-gateway equivalent)...");
    eprintln!("[2/5] Executing Tier 2: Rift Typed Model (serenity / twilight-model equivalent)...");
    eprintln!("[3/5] Executing Tier 3: Rift Full Stack (discord.js / discord.py equivalent)...");
    eprintln!("[4/5] Executing Baseline Reference Simulation (Node/V8 dynamic model)...");
    if run_external {
        eprintln!("[5/5] Detecting and invoking live Node.js and Python interpreters on PATH...");
    }

    let config = CompareConfig {
        event_count,
        guild_members: 100,
        zstd_level: 3,
        token: "rift-compare-benchmark-token".to_string(),
        run_live_external: run_external,
        external_port: None,
    };

    let report = CompareHarness::run(config).await?;

    if json_output {
        println!("{}", report.to_json());
    } else {
        println!("{}", report.render_markdown());
    }

    Ok(())
}
