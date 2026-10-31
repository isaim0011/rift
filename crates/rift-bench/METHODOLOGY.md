# Benchmark methodology

## What `cargo bench -p rift-bench` is (and isn't)

It times **individual components** of rift in isolation: zstd-stream decoding,
envelope parsing, interaction parsing, request serialization, shard routing,
permission math and the REST rate limiter. It is useful for **catching
regressions** and **finding hot spots**.

It is **not** evidence that rift is faster than discord.js, discord.py, Twilight
or Serenity. A component benchmark can't show that, because real bots are
dominated by network latency, Discord's own rate limits, and user code.

## What "faster than the others" would have to mean

Only measurements like these support a public claim:

| Metric | Why it matters |
|---|---|
| Events/second ingested (decompress + parse + dispatch + cache) | Throughput ceiling of one shard |
| CPU time per 1,000 events | Cost at scale; headroom on small VPSes |
| Resident memory with N guilds loaded | The usual real-world pain point |
| Time from process start to READY for N shards | Deploy and restart speed |
| p50 / p99 time from frame received to handler invoked | Latency under load |

## Local Mock Discord Gateway + REST Benchmark Harness (Built & Operational)

The benchmark harness described in the methodology is fully implemented in `crates/rift-bench`:

1. **Local Mock Discord Server (`127.0.0.1:0`)**:
   - **Mock Gateway WebSocket Server** (`MockGateway` via `tokio::net::TcpListener` + `tokio_tungstenite::accept_async`):
     - Speaks the real Discord v10 Gateway protocol over loopback WebSocket.
     - On connect: sends Opcode 10 `HELLO` (`{"op":10,"d":{"heartbeat_interval":41250}}`).
     - Validates client Opcode 2 `IDENTIFY` (and Opcode 6 `RESUME`).
     - Streams `READY` followed by `N` realistic `GUILD_CREATE` and `MESSAGE_CREATE` frames (~1 KB JSON each) compressed via a single long-lived `zstd-stream` context flushed after every frame.
     - Handles client Opcode 1 `HEARTBEAT` payloads with Opcode 11 `HEARTBEAT_ACK`.
     - Observes client close frames (Opcode 1000 `Normal`) for clean shutdown.
   - **Mock REST Server** (`MockRestServer`):
     - Serves `/api/v10/gateway/bot` for discovery and general API mock responses.

2. **Full End-to-End Pipeline Ingestion**:
   - Connects a real `rift_runtime::run_shard` driver over WebSocket + `zstd-stream`.
   - Ingests compressed frames, decompresses using `StreamDecoder`, parses the Gateway envelope, parses typed `rift_model::GatewayEvent` models, and updates `rift_cache::Cache`.
   - Measures:
     - `time_to_ready`: wall-clock duration from shard spawn to `READY` received.
     - `events_per_second`: end-to-end ingestion throughput.
     - `CPU/1000 events`: process CPU execution time per 1,000 ingested events.
     - `p50_latency_ns` & `p99_latency_ns`: per-event ingestion latency percentiles.

3. **Running the Benchmark Harness**:
   ```bash
   cargo run --release -p rift-bench --example harness
   ```
   Runs 1 warm-up run (discarded) + 5 measured iterations of 5,000 events each, then outputs a clean GitHub Flavored Markdown table of Min / Median / Max across all 5 runs.

4. **Running Harness Unit & Integration Tests**:
   ```bash
   cargo test -p rift-bench
   ```
   Verifies the end-to-end mock gateway + shard interaction: Hello -> Identify -> zstd-stream READY -> 100 events -> clean Shutdown.

5. **Cross-Library & Architecture Comparison Harness (`compare`)**:
   ```bash
   cargo run --release -p rift-bench --example compare
   ```
   Compares identical 10,000-event workloads across four rigorous architectural tiers and live runtimes:
   - **Tier 1: Rift Raw Driver (`rift-runtime` + `rift-gateway`)**: Equivalent to `twilight-gateway` (sans-I/O state machine + zstd-stream decompress + envelope scan, zero JSON payload deserialization).
   - **Tier 2: Rift Typed Model (`rift-runtime` + `rift-model`)**: Equivalent to `serenity` / `twilight-model` (strongly-typed zero-copy event deserialization).
   - **Tier 3: Rift Full Stack (`rift-runtime` + `rift-model` + `rift-cache` + permissions)**: Equivalent to `discord.js` / `discord.py` (state caching + channel obfuscation guard + effective permissions + REST route normalization).
   - **Baseline Reference Simulation (discord.js / discord.py Model)**: Algorithmic simulation of Node/Python runtimes executing eager JSON parsing into dynamic object trees (`serde_json::Value`), unbounded map growth, string BigInt parsing, dynamic permission loops, and regex route normalization on the identical frames.
   - **Live Interpreters (Node.js & Python 3)**: Auto-detects local `node` (`compare/bench_node.mjs`) and `python` (`compare/bench_python.py`) on `PATH` to include real V8 and CPython numbers directly in the printed comparison table.
   - **Mock Server Port Binding**: Pass `--external-port <PORT>` to keep `MockDiscordServer` running for external scripts to connect to.

## Rules for what gets claimed

- Quote a number only with its benchmark name, hardware, OS, and rift version.
- Never compare a rift microbenchmark with another library's end-to-end number.
- Use `cargo run --release -p rift-bench --example harness` on an identical workload when reporting comparative gateway throughput.
