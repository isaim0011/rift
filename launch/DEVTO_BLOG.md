# Building Rift: A Zero-Unsafe, Sans-I/O Discord Engine in Rust with Node & Python Bindings

Discord bots power millions of servers, but the frameworks that run them have stagnated in performance. If you've ever operated a bot serving 50,000+ guilds using Discord.js or Discord.py, you know the symptoms:
- High memory usage (400MB to 1.5GB+ just for basic guild caching)
- Event loop delays during message bursts
- Complex mock setups needed just to unit test event handlers

Today, we're introducing **Rift** — a fresh, ground-up reimagining of what a Discord v10 library should look like in 2026.

---

## The Three Architecture Pillars of Rift

### 1. Pure Sans-I/O Protocol Engine
Most network libraries tightly couple the network transport (`tokio::net::TcpStream`, `websocket`, `reqwest`) with the protocol logic.

In Rift, `rift-gateway` and `rift-rest` are **sans-I/O**. They are pure deterministic state machines:
- **Input**: Raw network frames, sequence updates, heartbeats.
- **Output**: Typed actions, emitted events, outgoing command buffers.

Because the protocol engine doesn't open sockets or manage timers internally, you can unit-test every edge case — dropped connections, resume sequences, invalid sessions, zlib-stream corruptions — in milliseconds without mocking networks.

### 2. Strict Zero-Unsafe Guarantee
Discord bots handle untrusted input from millions of users across the internet. Rift compiles under `#![forbid(unsafe_code)]` across all 9 crates in the workspace. Memory safety and concurrency safety are guaranteed at compile time.

### 3. Polyglot by Design (Rust, TypeScript, Bun, Python)
Writing in Rust shouldn't be mandatory to get systems-level speed. We built native bridges:
- **Node.js & Bun**: Powered by `napi-rs` with zero-copy buffer handoffs.
- **Python**: Powered by `PyO3` with full asyncio integration.

---

## Performance Benchmarks

Measured via `rift-bench` loopback harness with stateful `zstd-stream` compression (Intel Core i5-6200U @ 2.30GHz, Windows 10, 10,000 Gateway v10 events):

| Metric | Rift Raw Driver | Rift Full Stack | Live Node.js v24 | Live Python 3.14 |
| :--- | :--- | :--- | :--- | :--- |
| **Gateway Throughput** | **447,035 ev/s** | **112,381 ev/s** | 39,911 ev/s | 4,061 ev/s |
| **p50 Ingestion Latency** | **3.20 µs** | **10.00 µs** | 9.70 µs | 95.60 µs |
| **p99 Ingestion Latency** | **20.20 µs** | **28.20 µs** | 54.50 µs | 570.10 µs |
| **Wire Compression** | **19.5x (zstd-stream)** | **19.5x (zstd-stream)** | 1.0x (raw) | 1.0x (raw) |
| **State Memory Model** | **Stateless Driver** | **Bounded FIFO O(50)** | Heap GC (~22.8 MB) | Dict Graphs (~34.3 MB) |

---

## 100% Discord REST Coverage

Rift implements every single one of Discord's 250 REST routes, continuously verified against Discord's official OpenAPI 3.1 schema. Zero missing endpoints, zero handwritten schema discrepancies.

---

## AI Agent Integration: Model Context Protocol (MCP)

To bring Discord bot creation into the era of AI engineering, Rift ships with an official **MCP Server** ready to deploy on Cloudflare Workers. Any AI agent (Claude Desktop, Cursor, Antigravity) can connect to inspect routes, calculate bitwise permissions, and generate verified Rift bot code in seconds.

---

## Getting Started

Install Rift using your package manager of choice:

```bash
# Rust
cargo add rift-discord

# Node.js
npm install rift-discord

# Bun
bun add rift-discord

# Python
pip install rift-discord
```

- **GitHub**: https://github.com/isaim0011/rift
- **Website**: https://rift-discord.dev/
- **Documentation**: https://docs.rs/rift-discord
