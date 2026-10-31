//! Configuration options for [`Cache`](crate::Cache).

use serde::{Deserialize, Serialize};

/// Configuration controlling which entities to cache and memory bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Whether to cache guild objects. Defaults to `true`.
    pub cache_guilds: bool,
    /// Whether to cache channels and threads. Defaults to `true`.
    pub cache_channels: bool,
    /// Whether to cache guild members. Defaults to `true`.
    pub cache_members: bool,
    /// Whether to cache roles. Defaults to `true`.
    pub cache_roles: bool,
    /// Whether to cache standalone user objects. Defaults to `true`.
    pub cache_users: bool,
    /// Whether to cache user presences. Defaults to `true`.
    pub cache_presences: bool,
    /// Whether to cache voice states. Defaults to `true`.
    pub cache_voice_states: bool,
    /// Maximum number of messages to store per channel in a FIFO ring buffer.
    /// Defaults to `100`. Set to `0` to disable message caching.
    pub max_messages_per_channel: usize,
    /// Maximum number of members to store per guild. When exceeded, the oldest
    /// cached member is evicted (FIFO). Defaults to `None` (unbounded).
    pub max_members_per_guild: Option<usize>,
    /// Maximum number of users to store globally. When exceeded, the oldest
    /// cached user is evicted (FIFO). Defaults to `None` (unbounded).
    pub max_users: Option<usize>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_guilds: true,
            cache_channels: true,
            cache_members: true,
            cache_roles: true,
            cache_users: true,
            cache_presences: true,
            cache_voice_states: true,
            max_messages_per_channel: 100,
            max_members_per_guild: None,
            max_users: None,
        }
    }
}

impl CacheConfig {
    /// Creates a new `CacheConfig` with default settings (all entities enabled, 100 messages/channel).
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a `CacheConfig` with all caching toggles enabled and unbounded members/users.
    pub fn all() -> Self {
        Self::default()
    }

    /// Creates a `CacheConfig` with all caching toggles disabled.
    pub fn none() -> Self {
        Self {
            cache_guilds: false,
            cache_channels: false,
            cache_members: false,
            cache_roles: false,
            cache_users: false,
            cache_presences: false,
            cache_voice_states: false,
            max_messages_per_channel: 0,
            max_members_per_guild: None,
            max_users: None,
        }
    }

    /// Toggle guild caching.
    pub fn cache_guilds(mut self, enable: bool) -> Self {
        self.cache_guilds = enable;
        self
    }

    /// Toggle channel and thread caching.
    pub fn cache_channels(mut self, enable: bool) -> Self {
        self.cache_channels = enable;
        self
    }

    /// Toggle guild member caching.
    pub fn cache_members(mut self, enable: bool) -> Self {
        self.cache_members = enable;
        self
    }

    /// Toggle role caching.
    pub fn cache_roles(mut self, enable: bool) -> Self {
        self.cache_roles = enable;
        self
    }

    /// Toggle user caching.
    pub fn cache_users(mut self, enable: bool) -> Self {
        self.cache_users = enable;
        self
    }

    /// Toggle presence caching.
    pub fn cache_presences(mut self, enable: bool) -> Self {
        self.cache_presences = enable;
        self
    }

    /// Toggle voice state caching.
    pub fn cache_voice_states(mut self, enable: bool) -> Self {
        self.cache_voice_states = enable;
        self
    }

    /// Sets the maximum number of messages cached per channel (FIFO ring buffer).
    pub fn max_messages_per_channel(mut self, max: usize) -> Self {
        self.max_messages_per_channel = max;
        self
    }

    /// Sets the maximum number of members cached per guild (FIFO eviction).
    pub fn max_members_per_guild(mut self, max: Option<usize>) -> Self {
        self.max_members_per_guild = max;
        self
    }

    /// Sets the maximum number of users cached globally (FIFO eviction).
    pub fn max_users(mut self, max: Option<usize>) -> Self {
        self.max_users = max;
        self
    }
}
