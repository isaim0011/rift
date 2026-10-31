# rift-cache Plan

## Goal
Build an opt-in, high-performance, obfuscation-safe in-memory state cache for Discord bots in Rust.
Compliant with Discord API v10 and the Nov 16 Channel Obfuscation rules.

## Architecture
- `crates/rift-cache`:
  - `Cargo.toml`: Minimal dependencies on `rift-core`, `rift-model`, `serde`, `serde_json`.
  - `src/config.rs`: `CacheConfig` with entity toggles and memory bounds.
  - `src/stats.rs`: `CacheStats` reporting counts of entities and channels.
  - `src/permissions.rs`: Effective permission calculation implementing Discord permission hierarchy.
  - `src/cache.rs`: `Cache` maintaining state, event updates, evictions, obfuscation tracking.
  - `src/lib.rs`: Module exports, documentation, top-level re-exports.

## Memory Management
- Message ring buffer per channel using `VecDeque<Message>` with `max_messages_per_channel` capacity (default 100).
- Member cache per guild with configurable bounds `max_members_per_guild` using FIFO eviction.
- User cache with configurable bounds `max_users` using FIFO eviction.

## Obfuscation Compliance (Nov 16)
- Check `Channel::is_obfuscated()` (`ChannelFlags::CHANNEL_OBFUSCATED`, bit 17).
- Track obfuscated channel IDs in `obfuscated_channels: HashSet<Snowflake>`.
- `is_channel_obfuscated(id) -> bool` reports status.
- Obfuscated channels strip fake/placeholder names (`___hidden___`) and topics.
- `visible_channel(id) -> Option<&Channel>` returns `None` for obfuscated channels.
- `channel(id) -> Option<&Channel>` also returns `None` for obfuscated channels to guarantee zero leakage of placeholder metadata.
- `member_permissions_in_channel(guild, channel, user)` immediately returns `Some(Permissions::NONE)` if obfuscated.

## Permissions Resolution
- Full permission hierarchy:
  1. Obfuscated check -> `Permissions::NONE`.
  2. Owner check -> `Permissions::ALL`.
  3. Base permissions (`@everyone` + member roles) -> If `ADMINISTRATOR`, `Permissions::ALL`.
  4. Overwrites: `@everyone` overwrite, combined role overwrites (`allow` & `deny`), member overwrite.
  5. Implicit text permissions (`apply_implicit_text`).
  6. Timeout check (`apply_timeout`).

## Verification Strategy
- Zero compiler warnings with `#![warn(missing_docs)]`.
- At least 15 comprehensive unit tests:
  1. Config toggles and builders.
  2. Ready event populating users and unavailable guilds.
  3. Guild lifecycle: create, update, delete.
  4. Channel lifecycle: create, update, delete.
  5. Channel obfuscation detection and metadata masking.
  6. Channel visible vs obfuscated accessor behavior.
  7. Thread lifecycle and thread list sync.
  8. Member lifecycle: add, update, remove, chunk.
  9. Member limit eviction (FIFO).
  10. User limit eviction (FIFO).
  11. Role lifecycle: create, update, delete.
  12. Message ring buffer FIFO eviction when exceeding capacity.
  13. Message update and deletion (single and bulk).
  14. Voice state tracking (join, move, disconnect).
  15. Presence update tracking.
  16. End-to-end effective permissions: owner bypass.
  17. End-to-end effective permissions: administrator bypass.
  18. End-to-end effective permissions: role overwrites precedence.
  19. End-to-end effective permissions: member overwrite precedence.
  20. End-to-end effective permissions: timed-out member permissions.
  21. End-to-end effective permissions: obfuscated channel returns Permissions::NONE.
  22. Statistics consistency test across all entity additions and deletions.
