# Rift

[![CI](https://github.com/isaim0011/rift/actions/workflows/ci.yml/badge.svg)](https://github.com/isaim0011/rift/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](Cargo.toml)
[![OpenAPI Coverage](https://img.shields.io/badge/OpenAPI%203.1-250%2F250%20(100%25)-brightgreen.svg)](COVERAGE_REST.md)
[![Rust Safety](https://img.shields.io/badge/unsafe-forbidden-success.svg)](crates/)
[![npm](https://img.shields.io/npm/v/rift-discord.svg?color=cb3837&logo=npm)](https://www.npmjs.com/package/rift-discord)
[![PyPI](https://img.shields.io/pypi/v/rift-discord.svg?color=3775a9&logo=pypi)](https://pypi.org/project/rift-discord/)

**Rift** is a high-performance, modular, sans-I/O Discord engine written in pure Rust. Built from the ground up for extreme throughput, predictable sub-microsecond latency, and zero memory waste, Rift implements **250 out of 250 Discord REST operations (100.0% of Discord's OpenAPI 3.1 specification)** alongside a complete sharded gateway runtime, Voice Gateway v8 engine, obfuscation-safe state cache, and zero-overhead native bindings for Node.js, Bun, and Python.

---

## Architecture (10 Workspace Crates)

Rift is architected as 10 focused crates with strict layering, clean separation of concerns, and `#![forbid(unsafe_code)]` across all library crates:

```
                          ┌──────────────────────────┐
                          │           rift           │  Facade & Framework
                          │ (prelude, router, client)│
                          └─────────────┬────────────┘
                                        │
           ┌────────────────────────────┼───────────────────────────┐
           ▼                            ▼                           ▼
┌────────────────────┐       ┌────────────────────┐      ┌────────────────────┐
│    rift-runtime    │       │     rift-rest      │      │     rift-cache     │
│ (events, dispatch) │       │ (250/250 REST ops) │      │ (obfuscation-safe) │
└──────────┬─────────┘       └──────────┬─────────┘      └──────────┬─────────┘
           │                            │                           │
           ▼                            ▼                           ▼
┌────────────────────┐       ┌────────────────────┐      ┌────────────────────┐
│    rift-gateway    │       │     rift-http      │      │     rift-model     │
│ (shards, zstd/zlib)│       │(sans-IO ratelimit, │      │(events, entities)  │
└──────────┬─────────┘       │ Ed25519 verifier)  │      └──────────┬─────────┘
           │                 └──────────┬─────────┘                 │
           └────────────────────────────┼───────────────────────────┘
                                        ▼
                             ┌────────────────────┐
                             │     rift-core      │  Primitives (Snowflake,
                             │ (types, perms, ids)│  Permissions, Intents)
                             └────────────────────┘

Auxiliary & Tooling Crates:
• rift-voice:    Sans-I/O Voice Gateway v8 state machine, UDP IP discovery, and RTP framing.
• rift-coverage: Automated CI test & coverage analyzer verifying parity against openapi.json.
• rift-bench:    Criterion microbenchmarks, comparative cross-runtime benchmarks, and loopback mock harness.
• bindings:      Native zero-copy CPython and Node.js/Bun bindings (`rift-discord` on npm and PyPI).
```

1. **`rift`**: Umbrella facade exporting `rift::prelude::*`, `Framework`, `CommandRouter`, and unified `ClientBuilder`.
2. **`rift-core`**: Core primitives—strongly-typed `Snowflake`, zero-allocation `Permissions` bitfield math, `Intents`, and Discord opcodes.
3. **`rift-http`**: Sans-I/O rate limiting engine with per-route bucket tracking, 429 exponential backoff, and serverless Ed25519 signature verification.
4. **`rift-gateway`**: Resilient sharded WebSocket connection manager with heartbeat watchdogs, automatic resumes, and stream decompression (`zlib-stream` & `zstd`).
5. **`rift-rest`**: Production-grade asynchronous REST client covering 100% of Discord's OpenAPI 3.1 specification (all 250 operations) with local validation and multipart support.
6. **`rift-model`**: Strongly-typed Rust representations of Discord payloads, gateway event dispatches, and rich interactive components.
7. **`rift-cache`**: Lock-free, FIFO-bounded, obfuscation-safe in-memory cache with full Discord permission calculation engine.
8. **`rift-runtime`**: Event routing loop, command dispatcher, shard supervisor, and backpressure-aware event channels.
9. **`rift-voice`**: Sans-I/O Voice Gateway v8 protocol framing, UDP IP discovery, and RTP transport utilities.
10. **`rift-coverage` & `rift-bench`**: Spec coverage analyzer, Criterion performance suite, cross-library comparison tests, and end-to-end mock gateway runner.

---

## Feature Flags

Rift is modularized so minimal bots, serverless webhook handlers, and CLI utilities do not pay for features they do not use.

| Feature | Default | Description | Included Dependencies |
| :--- | :---: | :--- | :--- |
| **`default`** | Yes | Standard battery-included profile for modern bots | Enables `cache` and `framework` |
| **`cache`** | Yes | Obfuscation-safe in-memory state cache with permission calculator | `rift-cache` (`Cache`, `CacheConfig`, `Member`, `Guild`) |
| **`framework`** | Yes | Ergonomic slash command router, modal/button handlers, and auto-registration | `CommandRouter`, `Framework`, `prelude::*` |
| **`voice`** | No | Sans-I/O Voice Gateway v8 protocol, UDP IP discovery, and RTP framing | `rift-voice` (`VoiceSession`, RTP encryption) |
| **`full`** | No | Complete battery-included engine with all capabilities enabled | `cache`, `framework`, `voice` |

### Customizing Features in `Cargo.toml`

#### 1. Ultra-Lean Minimal Bot / Serverless Webhook (`default-features = false`)
Disables both the in-memory cache and the framework layer. Ideal for AWS Lambda, Cloudflare Workers, or microservices that only need REST requests, typed Snowflake math, or Ed25519 interaction verification:
```toml
[dependencies]
rift = { package = "rift-discord", version = "0.2", default-features = false }
```

#### 2. Gateway Bot without In-Memory Cache
Runs a full gateway event loop or command framework without keeping guild or channel state in RAM:
```toml
[dependencies]
rift = { package = "rift-discord", version = "0.2", default-features = false, features = ["framework"] }
```

#### 3. All Features Enabled (Full Engine with Voice)
```toml
[dependencies]
rift = { package = "rift-discord", version = "0.2", features = ["full"] }
```

---

## Structured Logging & Observability with `tracing`

Rift instruments all subsystems using zero-allocation structured diagnostics via the [`tracing`](https://crates.io/crates/tracing) ecosystem. Spans and events are emitted across:
- **Gateway Shards**: Shard spawn, heartbeat intervals, sequence updates, frame latency, decompression sizes, and automatic reconnect/resume triggers.
- **Runtime Supervisor**: Task lifecycle, worker thread health, shard status transitions, backpressure drops, and graceful shutdown events.
- **REST Rate-Limiter**: Route bucket keys, remaining quota, reset clocks, and 429 `Retry-After` backoff timers.

Users can attach any tracing subscriber (e.g. `tracing-subscriber`, OpenTelemetry, Datadog) with standard initialization:

```rust
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize structured logging with environment filter
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,rift=debug")),
        )
        .with_target(true)
        .with_thread_ids(true)
        .init();

    // 2. Build and run Rift client - all internal spans & events are automatically logged
    let client = rift::Client::builder("DISCORD_BOT_TOKEN", rift::Intents::GUILDS)
        .build()
        .await?;

    println!("Bot client initialized successfully with structured tracing.");
    Ok(())
}
```

---

## Quickstart (Rust)

### High-Level Framework with `rift::prelude::*`

The high-level `Framework` couples low-latency command routing with ergonomic handlers for slash commands, modals, and interactive components:

```rust
use rift::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Build router with type-safe slash commands and component handlers
    let router = CommandRouter::new()
        .slash("ping", "Check bot latency", |ctx| async move {
            let row = ActionRow::buttons(vec![
                Button::primary("btn:refresh", "Refresh Ping"),
            ]);
            let reply = CreateMessage::new()
                .content("Pong! Bot is operating with nominal latency.")
                .components_v2(vec![row.into()]);
            ctx.reply_message(reply).await?;
            Ok(())
        })
        .component_prefix("btn:", |ctx| async move {
            ctx.reply_ephemeral("Action confirmed via custom button component!").await?;
            Ok(())
        });

    // 2. Configure client with intents and caching
    let client = Client::builder("DISCORD_BOT_TOKEN", Intents::GUILDS | Intents::GUILD_MESSAGES)
        .with_cache(CacheConfig::default())
        .build()
        .await?;

    // 3. Register global slash commands and start event loop
    let mut framework = Framework::new(client, router);
    framework.register_global_commands(Snowflake::new(123456789012345678)).await?;
    framework.run().await?;

    Ok(())
}
```

### Low-Level Sharded Event Loop

When full control over raw gateway events is preferred:

```rust
use rift::{Client, Intents};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Client::builder("DISCORD_BOT_TOKEN", Intents::default())
        .build()
        .await?;

    while let Some(event) = client.next_event().await {
        println!("[Shard {}] Received event: {}", event.shard_id, event.name);
    }

    Ok(())
}
```

---

## Multi-Language Bindings (Node.js, Bun & Python)

Rift exposes its high-performance, sans-I/O Rust core directly to JavaScript/TypeScript (Node.js, Bun) and Python runtimes without writing a single line of Rust.

### Node.js & Bun (`rift-discord`)

Install via npm, pnpm, yarn, or bun:
```bash
npm install rift-discord
# or
bun add rift-discord
```

Use Rift's Rust primitives, Ed25519 verifier, cache, and client in TypeScript or JavaScript:

```typescript
import {
  Snowflake,
  Permissions,
  verifyInteraction,
  RateLimiter,
  Cache,
  RiftClient,
} from "rift-discord";

// 1. High-precision 64-bit Snowflake parsing & math
const id = "123456789012345678";
console.log(`Created: ${new Date(Snowflake.timestampMs(id)).toISOString()}, Worker: ${Snowflake.workerId(id)}`);

// 2. Zero-allocation Bitfield Permissions
const perms = Permissions.VIEW_CHANNEL | Permissions.SEND_MESSAGES | Permissions.EMBED_LINKS;
console.log("Can send messages:", Permissions.hasPermission(perms, Permissions.SEND_MESSAGES));

// 3. Constant-Time Serverless Ed25519 Interaction Verification
// Perfect for Express, Fastify, Hono, or Cloudflare Workers:
const isValid = verifyInteraction(
  process.env.DISCORD_PUBLIC_KEY!,
  req.headers["x-signature-ed25519"],
  req.headers["x-signature-timestamp"],
  rawBodyBuffer
);

// 4. Sans-I/O Rate Limiter & Obfuscation-Safe In-Memory Cache
const limiter = new RateLimiter();
const cache = new Cache();
cache.update(rawGuildJsonPayload);
```

### Python (`rift-discord`)

Install via pip or uv:
```bash
pip install rift-discord
# or
uv add rift-discord
```

Idiomatic Python bindings with type annotations and native C-extension speed:

```python
import os
from rift import (
    Snowflake,
    Permissions,
    verify_interaction,
    RateLimiter,
    Cache,
    RiftClient,
)

# 1. 64-bit Snowflake inspection
sf_id = 123456789012345678
print(f"Timestamp: {Snowflake.timestamp_ms(sf_id)}, Worker ID: {Snowflake.worker_id(sf_id)}, Process ID: {Snowflake.process_id(sf_id)}")

# 2. Permissions bitfield operations
perms = Permissions.VIEW_CHANNEL | Permissions.SEND_MESSAGES | Permissions.MANAGE_ROLES
assert Permissions.has_permission(perms, Permissions.SEND_MESSAGES)
print(f"Effective permissions bitfield: {perms:#x}")

# 3. Serverless Ed25519 Webhook Verification (FastAPI, Quart, Flask)
is_valid = verify_interaction(
    public_key_hex=os.environ["DISCORD_PUBLIC_KEY"],
    signature_hex=request.headers["x-signature-ed25519"],
    timestamp=request.headers["x-signature-timestamp"],
    body=raw_body_bytes,
)

# 4. Sans-I/O Rate Limiter and Obfuscation-safe Cache
limiter = RateLimiter()
cache = Cache()
cache.update(guild_data_dict)
```

---

## Key Features & Safety Guarantees

### 1. Nov 16 `CHANNEL_OBFUSCATION` Cache Safety
Discord's November 16 channel metadata obfuscation masks channels inaccessible to bots over Gateway dispatches. Rift provides first-class defense against data corruption:
- Detects the `CHANNEL_OBFUSCATED` flag (`1 << 17`) immediately on ingest.
- Automatically strips spoofed names, topics, and permission overwrites so placeholder strings (such as `___hidden___`) never contaminate bot state.
- `Cache::visible_channel()` and `Cache::channel()` safely return `None` for obfuscated channels.
- `Cache::member_permissions_in_channel()` returns `Permissions::NONE` for obfuscated channels to prevent privilege leakage.

### 2. Serverless Ed25519 Interaction Verification
For Cloudflare Workers, AWS Lambda, or HTTP interaction endpoints:
```rust
use rift::http::{verify_interaction, HEADER_SIGNATURE, HEADER_TIMESTAMP};

// Validates timestamp drift, hex public keys, and Ed25519 signatures in constant time:
let is_valid = verify_interaction(public_key, signature_header, timestamp_header, body_bytes);
```

### 3. Sans-I/O Voice Gateway v8 & RTP Framing
`rift-voice` implements Discord's Voice Gateway v8 protocol without coupling to an async runtime:
- Deterministic voice protocol state machine (`VoiceSession`).
- UDP IP Discovery packet serialization (`build_ip_discovery_request`, `parse_ip_discovery_response`).
- Real-time Transport Protocol (RTP) packet construction and nonce appendix layout for authenticated voice encryption modes (`aead_xchacha20_poly1305_rtpsize`, `aead_aes256_gcm_rtpsize`).

### 4. 100% REST OpenAPI Coverage (250 / 250 Operations)
All 250 operations documented in Discord's OpenAPI 3.1 specification are implemented, typed, locally validated, and tested:
- 33 `applications` operations
- 50 `channels` operations
- 2 `gateway` operations
- 93 `guilds` operations
- 1 `interactions` operation
- 9 `invites` operations
- 15 `lobbies` operations
- 4 `oauth2` operations
- 5 `partner-sdk` operations
- 2 `skus` operations
- 1 `soundboard-default-sounds` operation
- 4 `stage-instances` operations
- 2 `sticker-packs` operations
- 1 `stickers` operation
- 12 `users` operations
- 1 `voice` operation
- 15 `webhooks` operations

---

## Verification & Benchmarks

### 1. Running Tests
Execute the entire test suite across all workspace crates:
```bash
cargo test --workspace --all-targets
```

Test feature combinations:
```bash
# Minimal profile (no default features: cache & framework disabled)
cargo test -p rift --no-default-features

# Full battery-included profile
cargo test -p rift --all-features

# Verify documentation examples
cargo test --workspace --doc
```

### 2. Checking OpenAPI Spec Coverage
Run the automated coverage analyzer in strict mode:
```bash
cargo run -p rift-coverage -- openapi.json --strict
```

### 3. Cross-Library Comparison Benchmark (`compare`)
Rift includes a reproducible 3-tier comparative benchmark comparing Rift's Rust engine against live Node.js V8 (`discord.js` / `@discordjs/rest`) and Python CPython (`discord.py`) workloads:

- **Tier 1 (Primitives & Cryptography)**: Snowflake string formatting/parsing, Permissions bitfield checks, and constant-time Ed25519 signature verification against standard JS `tweetnacl`/WebCrypto and Python `PyNaCl`.
- **Tier 2 (State Cache & Channel Obfuscation)**: Cache ingestion of 10,000 members and permission calculation engine vs Discord.js `GuildMemberManager` and Discord.py cache dictionaries.
- **Tier 3 (Gateway Ingestion & Rate-Limiting)**: zstd-stream decompression, JSON envelope deserialization, and route bucket reservation under heavy concurrent load vs `@discordjs/rest` and `aiohttp`.

Run the comparison benchmark:
```bash
cargo run --release -p rift-bench --example compare
```

### 4. End-to-End Mock Gateway Ingestion Harness (`harness`)
Rift features an end-to-end loopback Discord Gateway + REST mock server (`127.0.0.1:0`) running WebSocket + `zstd-stream` compression. It executes a warm-up pass followed by 5 measured iterations of 5,000 events each, recording median, min, max `time_to_ready`, `events/sec`, `CPU/1000 events`, and `p50`/`p99` latency:

```bash
cargo run --release -p rift-bench --example harness
```

### 5. Microbenchmarks (Criterion)
Measure rate limiter performance, permission computation, and JSON serialization:
```bash
cargo bench -p rift-bench
```

---

## Continuous Integration & Quality Gates

Every commit and pull request is rigorously validated by GitHub Actions (`.github/workflows/ci.yml`):

1. **Multi-OS Test Matrix**: Runs all target tests, feature matrix permutations (`--no-default-features`, `--all-features`), and documentation tests across `ubuntu-latest`, `windows-latest`, and `macos-latest`.
2. **Lint & Formatting**: Enforces zero compiler warnings with `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo fmt --all -- --check`.
3. **Spec Parity Gate**: Automatically verifies 100% coverage (250/250 endpoints) with `cargo run -p rift-coverage -- openapi.json --strict`.
4. **Multi-Language Bindings**: Compiles and tests the native Node.js and Python binding crates.
5. **Zero-Warning Documentation**: Validates that all documentation and intra-doc links compile cleanly under `RUSTDOCFLAGS="-D warnings" cargo doc`.

---

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](Cargo.toml) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](Cargo.toml) or http://opensource.org/licenses/MIT)

at your option.
