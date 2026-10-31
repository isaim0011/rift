use rift_cache::{Cache, CacheConfig, CacheStats};
use rift_core::Snowflake;
use rift_model::GatewayEvent;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

/// Cache configuration tunables.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct CacheConfigInput {
    pub max_messages_per_channel: Option<usize>,
    pub max_members_per_guild: Option<usize>,
    pub max_users: Option<usize>,
}

/// Cache statistics snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheStatsOutput {
    pub guilds: usize,
    pub visible_channels: usize,
    pub obfuscated_channels: usize,
    pub roles: usize,
    pub members: usize,
    pub users: usize,
    pub messages: usize,
}

impl From<CacheStats> for CacheStatsOutput {
    fn from(s: CacheStats) -> Self {
        Self {
            guilds: s.guilds,
            visible_channels: s.visible_channels,
            obfuscated_channels: s.obfuscated_channels,
            roles: s.roles,
            members: s.members,
            users: s.users,
            messages: s.messages,
        }
    }
}

/// Helper envelope for parsing Gateway dispatches (`{"t": "EVENT_NAME", "d": {...}}`).
#[derive(Deserialize)]
struct GatewayEnvelope<'a> {
    t: Option<&'a str>,
    #[serde(borrow)]
    d: Option<&'a RawValue>,
}

/// In-memory Discord state cache with Nov 16 obfuscation safety and permission resolution.
pub struct NodeCache {
    inner: Cache,
}

impl Default for NodeCache {
    fn default() -> Self {
        Self::new(None)
    }
}

impl NodeCache {
    /// Creates a new cache instance with optional configuration.
    pub fn new(cfg: Option<CacheConfigInput>) -> Self {
        let mut config = CacheConfig::default();
        if let Some(c) = cfg {
            if let Some(m) = c.max_messages_per_channel {
                config.max_messages_per_channel = m;
            }
            if let Some(m) = c.max_members_per_guild {
                config.max_members_per_guild = Some(m);
            }
            if let Some(u) = c.max_users {
                config.max_users = Some(u);
            }
        }
        Self {
            inner: Cache::new(config),
        }
    }

    /// Updates cache with an incoming Gateway dispatch event.
    /// Supports both complete dispatch envelopes `{"t":"NAME","d":{...}}`
    /// or typed event payloads when event name is provided.
    pub fn update(&mut self, event_json: &str) -> Result<(), String> {
        if let Ok(env) = serde_json::from_str::<GatewayEnvelope>(event_json) {
            if let (Some(t), Some(d)) = (env.t, env.d) {
                let event = GatewayEvent::parse(t, d)
                    .map_err(|e| format!("Failed to parse gateway event {t}: {e}"))?;
                self.inner.update(&event);
                return Ok(());
            }
        }

        // Fallback: parse as RawValue if bare payload
        let raw = RawValue::from_string(event_json.to_string())
            .map_err(|e| format!("Invalid JSON: {e}"))?;
        let event = GatewayEvent::Unknown {
            name: "UNKNOWN".to_string(),
            data: raw,
        };
        self.inner.update(&event);
        Ok(())
    }

    /// Updates cache explicitly with event name and payload JSON.
    pub fn update_event(&mut self, name: &str, payload_json: &str) -> Result<(), String> {
        let raw = RawValue::from_string(payload_json.to_string())
            .map_err(|e| format!("Invalid JSON payload for {name}: {e}"))?;
        let event = GatewayEvent::parse(name, &raw)
            .map_err(|e| format!("Failed to parse {name}: {e}"))?;
        self.inner.update(&event);
        Ok(())
    }

    /// Returns current statistics and entity counts.
    pub fn stats(&self) -> CacheStatsOutput {
        self.inner.stats().into()
    }

    /// Checks if a channel is obfuscated under Nov 16 rules.
    pub fn is_channel_obfuscated(&self, channel_id: u64) -> bool {
        self.inner.is_channel_obfuscated(Snowflake::new(channel_id))
    }

    /// Retrieves a cached guild serialized as JSON, if present.
    pub fn get_guild(&self, guild_id: u64) -> Option<String> {
        let g = self.inner.guild(Snowflake::new(guild_id))?;
        serde_json::to_string(g).ok()
    }

    /// Retrieves a cached channel serialized as JSON.
    /// Returns `None` if the channel does not exist OR is obfuscated.
    pub fn get_channel(&self, channel_id: u64) -> Option<String> {
        let ch = self.inner.channel(Snowflake::new(channel_id))?;
        serde_json::to_string(ch).ok()
    }

    /// Retrieves a cached member serialized as JSON, if present.
    pub fn get_member(&self, guild_id: u64, user_id: u64) -> Option<String> {
        let m = self
            .inner
            .member(Snowflake::new(guild_id), Snowflake::new(user_id))?;
        serde_json::to_string(m).ok()
    }

    /// Retrieves a cached message serialized as JSON, if present.
    pub fn get_message(&self, channel_id: u64, message_id: u64) -> Option<String> {
        let msg = self
            .inner
            .message(Snowflake::new(channel_id), Snowflake::new(message_id))?;
        serde_json::to_string(msg).ok()
    }

    /// Computes effective permissions for a member in a channel.
    /// Returns 0 immediately if the channel is obfuscated or missing.
    pub fn compute_member_permissions(&self, guild_id: u64, channel_id: u64, user_id: u64) -> u64 {
        self.inner
            .member_permissions_in_channel(
                Snowflake::new(guild_id),
                Snowflake::new(channel_id),
                Snowflake::new(user_id),
            )
            .map(|p| p.bits())
            .unwrap_or(0)
    }

    /// Clears all cached state.
    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_lifecycle_and_stats() {
        let mut cache = NodeCache::default();
        let stats = cache.stats();
        assert_eq!(stats.guilds, 0);

        // Update with GUILD_CREATE
        let guild_json = r#"{
            "id": "1000",
            "name": "Test Guild",
            "owner_id": "2000",
            "roles": [],
            "emojis": [],
            "features": []
        }"#;
        assert!(cache.update_event("GUILD_CREATE", guild_json).is_ok());

        let stats2 = cache.stats();
        assert_eq!(stats2.guilds, 1);
        assert!(cache.get_guild(1000).is_some());

        cache.clear();
        assert_eq!(cache.stats().guilds, 0);
    }

    #[test]
    fn test_obfuscated_channel_safety() {
        let mut cache = NodeCache::default();

        // Channel with obfuscated flag (1 << 17 = 131072)
        let ch_json = r#"{
            "id": "5555",
            "type": 0,
            "flags": 131072,
            "name": "___hidden___"
        }"#;
        assert!(cache.update_event("CHANNEL_CREATE", ch_json).is_ok());

        assert!(cache.is_channel_obfuscated(5555));
        // Obfuscated channel must return None
        assert!(cache.get_channel(5555).is_none());
        // Permissions in obfuscated channel must be 0
        assert_eq!(cache.compute_member_permissions(1000, 5555, 2000), 0);
    }
}
