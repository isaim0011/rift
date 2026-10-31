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

In synthetic benchmark runs mirroring a 100,000-guild bot experiencing a continuous flood of `MESSAGE_CREATE` and `TYPING_START` events:

| Metric | Rift | Serenity | Discord.js | Discord.py |
| :--- | :--- | :--- | :--- | :--- |
| **Gateway Throughput** | **820,000 evt/s** | 410,000 evt/s | 85,000 evt/s | 62,000 evt/s |
| **RAM (100k Guilds)** | **42 MB** | 86 MB | 490 MB | 380 MB |
| **REST P99 Latency** | **1.2 ms** | 1.8 ms | 8.4 ms | 11.1 ms |
| **Cold Boot Time** | **<15 ms** | <20 ms | 280 ms | 340 ms |

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
