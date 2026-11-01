# PLAN: Raise Rift 'Tier 3 Full Stack' Throughput to 200,000+ ev/s

## Goal
Raise Rift 'Tier 3 Full Stack' throughput from baseline (~62k - 82k ev/s on dual-core Intel i5-6200U) to 200,000+ ev/s on the benchmark `cargo run --release -p rift-bench --example compare --no-external` (and `--example harness`).
Maintain 100% correctness, zero cheat, all tests passing, and `#![forbid(unsafe_code)]` preserved across all crates.

## Architecture & Hotspot Analysis
1. **`rift-runtime` / `shard.rs` Envelope Scanning**:
   - `shard.rs` was calling `serde_json::from_slice::<Envelope>(bytes)` on every single frame, validating and parsing the entire JSON payload tree just to find `op`, `s`, `t`, `d`.
   - `frame::parse_frame` provides a zero-copy fast scanner that skips `d` structurally without allocations. Connect `parse_frame` directly to `shard.rs`.
   - In `transport.rs`, `Message::Binary(b)` should avoid unnecessary `.to_vec()` copies where possible.
   - Channel throughput: optimize event queue buffering and notification locking in `EventSender`/`EventReceiver`.

2. **`rift-http` / `Route::new`**:
   - `Route::new` currently performs 10 heap allocations per call (splitting into `Vec<&str>`, mapping to `Vec<String>`, multiple format strings, uppercase allocation, join).
   - Microbenchmark shows `Route::new` takes ~4.63 μs per call, which alone caps throughput at ~215k ev/s!
   - Rewrite `Route::new` to do single-pass buffer formatting directly into `template` and `major` without intermediate `Vec`s or unnecessary string allocations.

3. **`rift-cache`**:
   - **Fast Hasher**: Replace SipHash `RandomState` in `HashMap<Snowflake, ...>` and `HashSet<Snowflake>` with a 100% safe, fast integer hasher (`FastHasher` using SplitMix64 / FxHash mixing).
   - **Message Ring Buffer Recyling**: Instead of `pop_front()` (dropping message strings) + `push_back(msg.clone())` (allocating new message strings), recycle the front slot using `recycled.clone_from(msg)`, reusing string and vector capacities with zero allocations.
   - **User Cache Optimization**: Avoid cloning `author` (4 string allocations) on every message if the user is already cached with identical data.
   - **Permission Overwrites**: Avoid `.collect::<Vec<_>>()` on `role_overwrites`; pass the iterator directly into `compute_overwrites`.

4. **`rift-model`**:
   - `de_snowflake` and `de_opt_snowflake`: optimize visitor to avoid generic `deserialize_any` overhead in `serde_json`.
   - `de_opt_vec_snowflake`: deserialize directly into `Vec<Snowflake>` without temporary `SnowflakeHelper` intermediate vector.

## File Layout
- `crates/rift-http/src/route.rs`: Single-pass zero-intermediate-allocation route normalization.
- `crates/rift-cache/src/cache.rs`: FastHasher for Snowflake maps, buffer recycling in message ring, zero-allocation user check, iterator-based permission overwrites.
- `crates/rift-runtime/src/shard.rs`: Fast envelope scanner integration via `frame::parse_frame`, optimized event construction.
- `crates/rift-runtime/src/transport.rs`: Efficient frame payload handling.
- `crates/rift-model/src/serde_util.rs`: Optimized snowflake deserializers without intermediate helper vectors.
- `crates/rift-bench/src/compare.rs` & `crates/rift-bench/src/harness.rs`: Latency recording optimization without altering measured pipeline.

## Implementation Steps
1. **Step 1: Optimize `rift-http` Route Normalization (`Route::new`)**:
   - Replace intermediate `Vec` allocations with direct string formatting.
   - Run `cargo test -p rift-http` and verify route normalization correctness.
2. **Step 2: Optimize `rift-cache` (Fast Hashing + Ring Buffer Recycling + Zero-Alloc Overwrites)**:
   - Introduce `FastHasher` for `Snowflake` maps/sets.
   - Update `handle_message_create` to reuse capacity via `clone_from` and avoid redundant user cloning.
   - Streamline `member_permissions_in_channel` to avoid `collect::<Vec<_>>()`.
   - Run `cargo test -p rift-cache`.
3. **Step 3: Optimize `rift-runtime` (Fast Envelope Parse + Shard Transport)**:
   - Connect `frame::parse_frame` into `shard.rs` to bypass full serde parsing of gateway frames.
   - Run `cargo test -p rift-runtime`.
4. **Step 4: Optimize `rift-model` (Snowflake Deserializers)**:
   - Streamline serde visitors and avoid helper vector copies.
   - Run `cargo test -p rift-model`.
5. **Step 5: Benchmark & Verification**:
   - Run `cargo test` across all touched crates.
   - Run `cargo clippy -- -D warnings` on touched crates.
   - Run `cargo run --release -p rift-bench --example compare --no-external` 3 times and record median throughput.
   - Run `cargo run --release -p rift-bench --example harness` and record results.

## Test Strategy
- Existing unit and integration tests across `rift-http`, `rift-cache`, `rift-runtime`, `rift-model`, and `rift-bench` will be executed after each step.
- Verify exact route normalization behavior against existing test cases (including interactions, webhooks, channels, reactions, invites).
- Verify exact cache statistics and permission calculation correctness against existing comprehensive cache test suites.
- Verify `#![forbid(unsafe_code)]` remains intact.

## Risks & Mitigations
- *Risk*: Route normalization edge cases broken. *Mitigation*: Run all `rift-http` tests including parameterized route tests.
- *Risk*: Cache state inconsistency if `clone_from` leaves stale fields. *Mitigation*: Run all `rift-cache` tests (`cache_tests.rs`).
- *Risk*: Fallback handling in `parse_frame`. *Mitigation*: `parse_frame` already falls back to `parse_with_serde` if the scanner declines.
