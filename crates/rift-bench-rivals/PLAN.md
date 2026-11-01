# Head-to-Head Rivals Benchmark Plan (`rift-bench-rivals`)

## 1. Objective
Benchmark `rift` head-to-head against the major Rust Discord library ecosystems:
- **Twilight** (`twilight-gateway` v0.17 + `twilight-model` v0.17)
- **Serenity** (`serenity` v0.12 gateway + model)
- **Rift** (`rift-runtime` + `rift-gateway` + `rift-model` + `rift-cache`)

against the exact same deterministic 10,000-event workload used in `rift-bench`'s `compare` harness:
- 1 `READY` frame (Opcode 0)
- 1 `GUILD_CREATE` frame (Opcode 0, with 100 members, roles, channels)
- 9,999 `MESSAGE_CREATE` frames (Opcode 0, ~1 KB JSON with author, member, roles, channel, timestamp)

## 2. Architectural Tiers & Equivalence
1. **Raw Gateway Driver Tier (Decompression + Framing)**:
   - **Rift Raw Driver**: `rift-runtime` + `rift-gateway` (sans-I/O zstd-stream decompression, framing, envelope extraction, zero JSON deserialization).
   - **Twilight Raw Driver**: `twilight-gateway` `Shard` polling raw messages (`next()`) with `zstd` transport decompression.
2. **End-to-End Typed Model Ingestion Tier**:
   - **Rift Typed**: `rift-runtime` + `rift-model` (`GatewayEvent::parse` on every dispatch).
   - **Twilight Typed**: `twilight-gateway` + `twilight-model` (`next_event(EventTypeFlags::all())` decoding into `twilight_model::gateway::event::Event`).
   - **Serenity E2E** (if drivable against loopback mock): `serenity::gateway::Shard` / `WsClient` reading and decoding into `serenity::model::event::GatewayEvent`.
3. **Parse-Only Typed Model Deserialization Tier**:
   - Ingesting identical raw JSON payload buffers across all 3 libraries:
     - `rift-model`: `GatewayEvent::parse(name, raw_data)`
     - `twilight-model`: `twilight_model::gateway::event::Event` deserialization
     - `serenity-model`: `serenity::model::event::GatewayEvent` or `Event` deserialization
   - Labeled explicitly as "Parse-Only" to isolate CPU deserialization speed from socket I/O.
4. **Full Stack Tier**:
   - **Rift Full Stack**: Driver + Typed Parsing + Ring-buffer Cache + Permission Resolution + Route Normalization.

## 3. Methodology & Rigor
- Compilation: `--release` mode (`opt-level = 3`, `lto = "fat"`).
- Runs: 1 warm-up run + median of 5 real runs.
- Metrics:
  - Throughput (events/sec)
  - Latency: p50 (median) and p99 (tail) in microseconds
  - Total duration and data processed
- Isolation: `crates/rift-bench-rivals` excluded from default workspace build so normal workspace builds (`cargo test --workspace`) remain fast without dragging in twilight/serenity dependencies.
