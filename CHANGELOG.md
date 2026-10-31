# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-31

### Added

- **`rift-core`**:
  - Zero-dependency Discord fundamental primitives including 64-bit `Snowflake` arithmetic, epoch offsets, and timestamp extraction.
  - Bitflag abstractions for `Intents`, `Permissions`, and `Capabilities` conforming to the Discord Developer Documentation.
  - Shard calculation algorithms (`shard_for_guild`, `start_waves`) with deterministic identify concurrency spacing.
  - Standard gateway close codes and opcode enumerations.

- **`rift-gateway`**:
  - Sans-I/O state machine managing Discord Gateway protocol sessions without runtime coupling.
  - Heartbeat timer state machine with jitter calculations, sequence tracking, and zombie connection detection.
  - Identify budget manager enforcing the 1,000 identifies/day rolling reserve and rate limits.
  - Token-bucket send rate limiter (120 payloads per 60 seconds) with priority-lane bypass for heartbeats and resumes.
  - Reconnect and resume planner cleanly differentiating between fatal and resumable disconnect codes.

- **`rift-http`**:
  - Sans-I/O REST rate limiter implementing Discord per-route bucket hashes, reset timestamps, and global concurrency caps.
  - Cloudflare invalid-request circuit breaker tracking 401, 403, and 429 response codes to prevent bot IP bans (10,000 errors / 10-minute threshold).
  - Webhook verification utilities powered by `ed25519-dalek` for verifying Discord interaction request signatures with timestamp validation.

- **`rift-rest`**:
  - Complete, asynchronous Discord REST client achieving 100% endpoint coverage (250/250 routes mapped from the Discord OpenAPI 3.1 specification).
  - Typed builder interfaces for messages, embeds, action rows, buttons, select menus, text displays, media galleries, containers, and modals.
  - First-class support for Interaction responses (pong, deferred messages, modal launches, autocomplete suggestions).
  - Swappable transport layer with a high-performance default implementation on `reqwest` and native TLS.

- **`rift-runtime`**:
  - Production-grade Tokio async driver managing WebSocket connections to Discord's Gateway v10.
  - Real-time `zstd-stream` decompression pipeline maintaining an active decompression context across binary WebSocket frames.
  - Backpressure-aware event channel architecture supporting both `Unbounded` and `Bounded` buffer policies with configurable `drop_oldest` shedding.
  - Structured, non-blocking telemetry and diagnostics replacing stdout/stderr logging with `tracing` spans and events.

- **`rift-model`**:
  - Lazy-parsed gateway dispatch type system handling ~70 distinct Discord Gateway events.
  - Raw JSON value preservation via `serde_json::value::RawValue` allowing bots to deserialize only required fields.
  - Strongly-typed representations for Guilds, Channels, Members, Roles, Emojis, Stickers, Invites, and Interactions.

- **`rift-cache`**:
  - High-performance, memory-bounded in-memory state cache governed by FIFO ring buffers for messages and member events.
  - Obfuscation-safe channel access and visibility tracking compliant with Discord's November 16 `CHANNEL_OBFUSCATION` capability.
  - Lock-free, real-time effective permission calculation resolving administrator overrides, ownership rights, member/role overwrites, and channel obfuscation restrictions.

- **`rift-voice`**:
  - Sans-I/O Discord Voice Gateway v8 protocol engine and UDP framing implementation.
  - Support for modern Discord Audio & Video End-to-End Encryption (DAVE) MLS opcodes.
  - Pure-Rust, zero-dependency implementations of XChaCha20-Poly1305 and ChaCha20-Poly1305 AEAD ciphers with constant-time equality validation.
  - UDP IP discovery packet construction, parsing, and RTP header manipulation.

- **`rift` (Top-Level Library & Framework)**:
  - Multi-shard supervisor orchestrating automated wave launches adhering to Gateway `max_concurrency` rules.
  - High-level `CommandRouter` and `Framework` supporting slash commands, user context menus, message context menus, button/component interactions, modal submissions, and autocomplete.
  - Feature flags for modular compilation: `cache` (`dep:rift-cache`), `framework`, and `voice` (`dep:rift-voice`), with `default = ["cache", "framework"]`.
  - Comprehensive `prelude` module for ergonomic one-line imports across the entire Rift ecosystem.
  - Re-export of `tracing` for out-of-the-box structured logging integration.

- **Ecosystem Language Bindings**:
  - Node.js and Bun native bindings via `napi-rs` (`bindings/node`).
  - Python 3 native extension module via `PyO3` (`bindings/python`).

### Performance

- Zero allocations on unhandled gateway dispatch events via early opcode probing and client-side event filtering.
- Continuous streaming decompression with `zstd` avoiding per-message allocation spikes and buffer fragmentation.
- Zero-copy deserialization using borrowed JSON raw value slices for large guild create dispatches.
- Atomic token-bucket send and REST bucket limiters minimizing synchronization overhead under high concurrency.
- Release profiles configured with fat Link-Time Optimization (`lto = "fat"`), single codegen units, and `panic = "abort"` for maximum runtime throughput.

### Security

- Enforced `#![forbid(unsafe_code)]` across all Rust library crates in the workspace.
- Pure-Rust cryptographic implementations in `rift-voice` and `rift-http` utilizing constant-time comparison routines to prevent timing side channels.
- Hardened identify reserve mechanism reserving identify tokens to prevent crash-looping bot processes from exhausting daily login quotas and invalidating bot tokens.
- Circuit breaker monitoring invalid REST responses to proactively stop outgoing traffic before triggering Cloudflare firewall mitigations.
- Obfuscated channel boundaries preventing accidental exposure or leakage of restricted guild channel metadata.
