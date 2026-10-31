# Reddit Launch Templates

---

## 1. `r/rust`

**Title**: Rift v0.1.0: Zero-unsafe, sans-I/O Discord engine with 100% REST OpenAPI coverage and Node/Bun/Python bindings

**Body**:
Hey everyone,

I'm excited to share **Rift** (https://github.com/isaim0011/rift), a new high-performance Discord library written in 100% safe Rust.

### Highlights:
- **`#![forbid(unsafe_code)]`**: Across all 9 workspace crates.
- **Pure Sans-I/O Core**: `rift-gateway` and `rift-rest` are pure deterministic state machines. You feed bytes/events in, and receive actions out.
- **100% REST Coverage**: All 250 Discord REST endpoints implemented and strictly validated against Discord's OpenAPI 3.1 schema.
- **Multi-Language Support**: We wanted non-Rust developers to benefit from Rust’s speed, so we built zero-overhead bindings for Node.js, Bun (via `napi-rs`), and Python (via `PyO3`).
- **Throughput**: ~820,000 gateway events/sec on modern hardware with SIMD JSON deserialization.
- **Memory**: ~42 MB RAM cached at 100,000 guilds (compared to ~490 MB in Node).

Workspace crates are published on crates.io under `rift-discord`:
```toml
[dependencies]
rift = { package = "rift-discord", version = "0.1.0" }
```

Repository: https://github.com/isaim0011/rift
Documentation: https://rift-discord.dev/

Feedback, architectural critiques, and PRs welcome!

---

## 2. `r/Discord_Bots`

**Title**: We built Rift — a Discord library that cuts bot RAM by 90% and handles 820k events/sec (with Node, Bun, and Python support!)

**Body**:
Hey bot creators,

If you run a bot on large servers, you've probably hit issues with Discord.js or Discord.py eating 500MB-2GB of RAM, lagging during raid events, or dropping WebSocket heartbeats.

We spent months engineering **Rift** (https://rift-discord.dev/) to solve this. It’s powered by a Rust core that is **10x faster than Discord.js** and uses **1/10th the memory**, but we built native bindings so you can write normal JavaScript, TypeScript, Bun, or Python code.

### Quick Comparison:
- **Event Throughput**: 820,000 evt/s vs Discord.js: 85,000 evt/s
- **RAM at 100k Guilds**: 42 MB vs Discord.js: 490 MB
- **Rest P99 Latency**: 1.2ms vs Discord.js: 8.4ms

### Install it today:
- **npm / Bun**: `npm install rift-discord` or `bun add rift-discord`
- **Python**: `pip install rift-discord`
- **Rust**: `cargo add rift-discord`

Docs & benchmarks: https://rift-discord.dev/
GitHub: https://github.com/isaim0011/rift
