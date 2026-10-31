//! Statistics snapshot for [`Cache`](crate::Cache).

use serde::{Deserialize, Serialize};

/// Current entity counts in the cache.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// Number of cached guilds.
    pub guilds: usize,
    /// Number of cached visible (non-obfuscated) channels.
    pub visible_channels: usize,
    /// Number of cached obfuscated channels.
    pub obfuscated_channels: usize,
    /// Total number of cached roles across all guilds.
    pub roles: usize,
    /// Total number of cached guild members across all guilds.
    pub members: usize,
    /// Total number of cached users globally.
    pub users: usize,
    /// Total number of cached messages across all channels.
    pub messages: usize,
}

impl CacheStats {
    /// Total number of channels cached (both visible and obfuscated).
    pub fn total_channels(&self) -> usize {
        self.visible_channels + self.obfuscated_channels
    }

    /// Alias for [`messages`](Self::messages).
    pub fn cached_messages(&self) -> usize {
        self.messages
    }
}
