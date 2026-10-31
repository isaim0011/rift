//! # rift-cache
//!
//! An opt-in, high-performance, obfuscation-safe in-memory state cache for Discord bots.
//!
//! ## Features
//!
//! - **Fine-grained Entity Toggles**: Granular toggles via [`CacheConfig`] to cache only what
//!   your bot needs (guilds, channels, members, roles, users, presences, voice states).
//! - **Strict Memory Bounds**:
//!   - Channel message ring buffers with configurable FIFO capacities ([`CacheConfig::max_messages_per_channel`]).
//!   - Per-guild member limits with deterministic FIFO eviction ([`CacheConfig::max_members_per_guild`]).
//!   - Global standalone user limits with deterministic FIFO eviction ([`CacheConfig::max_users`]).
//! - **Nov 16 Channel Obfuscation Compliance**:
//!   - Detects obfuscated channels (`CHANNEL_OBFUSCATED`, bit 17).
//!   - Identifies whether a channel is obfuscated via [`Cache::is_channel_obfuscated`].
//!   - Never leaks fake or placeholder names (e.g. `___hidden___`) or topics.
//!   - [`Cache::visible_channel`] and [`Cache::channel`] return `None` for obfuscated channels.
//!   - Sanitizes raw channel storage so obfuscated channels carry `None` for name, topic, and overwrites.
//!   - [`Cache::member_permissions_in_channel`] immediately returns [`Permissions::NONE`] for obfuscated channels.
//! - **Full Discord Permission Math**:
//!   - End-to-end effective permissions computed via [`Cache::member_permissions_in_channel`].
//!   - Full support for guild owner bypass, administrator bypass, `@everyone` overwrites, role overwrites,
//!     member overwrites, implicit text permissions, and active member timeout restrictions.
//! - **Zero Compiler Warnings & No Unsafe**: `#![forbid(unsafe_code)]` and `#![warn(missing_docs)]`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod cache;
pub mod config;
pub mod permissions;
pub mod stats;
pub mod velocity;

pub use cache::Cache;
pub use config::CacheConfig;
pub use permissions::{
    apply_implicit_permissions, apply_timeout, compute_base_permissions, compute_overwrites,
    is_timed_out, parse_iso8601_to_unix,
};
pub use stats::CacheStats;
pub use velocity::{SlidingWindow, VelocityTracker};

// Re-export common primitives for convenience
pub use rift_core::{Permissions, Snowflake};
