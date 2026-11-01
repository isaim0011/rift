# Reddit Launch Templates

---

## 1. `r/rust`

**Title**: Rift v0.2.0: Zero-unsafe, sans-I/O Discord engine with 100% REST OpenAPI coverage and Node/Bun/Python bindings

**Body**:
Hey everyone,

I'm excited to share **Rift** (https://github.com/isaim0011/rift), a new high-performance Discord library written in 100% safe Rust.

### Highlights:
- **`#![forbid(unsafe_code)]`**: Across all 9 workspace crates.
- **Pure Sans-I/O Core**: `rift-gateway` and `rift-rest` are pure deterministic state machines. You feed bytes/events in, and receive actions out.
- **100% REST Coverage**: All 250 Discord REST endpoints implemented and strictly validated against Discord's OpenAPI 3.1 schema.
- **Multi-Language Support**: We wanted non-Rust developers to benefit from Rust’s speed, so we built zero-overhead bindings for Node.js, Bun (via `napi-rs`), and Python (via `PyO3`).
- **Throughput**: 447,035 raw gateway events/sec and 112,381 events/sec full-stack ingestion measured on local loopback (Intel i5-6200U).
- **Compression**: 19.5x wire bandwidth savings via streaming `zstd-stream` (7.6 MB raw -> 0.4 MB compressed).

Workspace crates are published on crates.io under `rift-discord`:
```toml
[dependencies]
rift = { package = "rift-discord", version = "0.2.0" }
```

Repository: https://github.com/isaim0011/rift
Documentation: https://rift-discord.dev/

Feedback, architectural critiques, and PRs welcome!

---

## 2. `r/Discord_Bots`

**Title**: We built Rift — an ultra-fast, zero-unsafe Discord engine with Node, Bun, and Python support!

**Body**:
Hey bot creators,

If you run a bot on large servers, you've probably hit issues with Discord.js or Discord.py eating memory, lagging during raid events, or dropping WebSocket heartbeats.

We spent months engineering **Rift** (https://rift-discord.dev/) to solve this. It’s powered by a Rust core with zero unsafe code and native bindings so you can write normal JavaScript, TypeScript, Bun, or Python code.

### Quick Measured Comparison (Loopback with zstd-stream on Intel i5-6200U):
- **Raw Event Throughput**: 447,035 ev/s (p50: 3.40 µs)
- **Full Stack Throughput**: 112,381 ev/s (vs live Node.js: 39,911 ev/s, live Python: 4,061 ev/s)
- **Wire Bandwidth Savings**: 19.5x compression via zstd-stream (7.6 MB raw -> 0.4 MB compressed)
- **State Memory**: Bounded FIFO O(50) (~120 KB per shard)

### Install it today:
- **npm / Bun**: `npm install rift-discord` or `bun add rift-discord`
- **Python**: `pip install rift-discord`
- **Rust**: `cargo add rift-discord`

Docs & benchmarks: https://rift-discord.dev/
GitHub: https://github.com/isaim0011/rift
