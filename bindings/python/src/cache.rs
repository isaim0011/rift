use rift_cache::{Cache, CacheConfig};
use rift_core::Snowflake;
use rift_model::GatewayEvent;
use serde::Deserialize;
use serde_json::value::RawValue;

#[derive(Deserialize)]
struct GatewayEnvelope<'a> {
    t: Option<&'a str>,
    #[serde(borrow)]
    d: Option<&'a RawValue>,
}

/// In-memory Discord state cache for Python with Nov 16 obfuscation compliance.
pub struct PyCache {
    inner: Cache,
}

impl Default for PyCache {
    fn default() -> Self {
        Self::new(None, None, None)
    }
}

impl PyCache {
    pub fn new(
        max_messages_per_channel: Option<usize>,
        max_members_per_guild: Option<usize>,
        max_users: Option<usize>,
    ) -> Self {
        let mut config = CacheConfig::default();
        if let Some(m) = max_messages_per_channel {
            config.max_messages_per_channel = m;
        }
        if let Some(m) = max_members_per_guild {
            config.max_members_per_guild = Some(m);
        }
        if let Some(u) = max_users {
            config.max_users = Some(u);
        }
        Self {
            inner: Cache::new(config),
        }
    }

    pub fn update(&mut self, event_json: &str) -> Result<(), String> {
        if let Ok(env) = serde_json::from_str::<GatewayEnvelope>(event_json) {
            if let (Some(t), Some(d)) = (env.t, env.d) {
                let event = GatewayEvent::parse(t, d)
                    .map_err(|e| format!("Failed to parse event {t}: {e}"))?;
                self.inner.update(&event);
                return Ok(());
            }
        }

        let raw = RawValue::from_string(event_json.to_string())
            .map_err(|e| format!("Invalid JSON: {e}"))?;
        let event = GatewayEvent::Unknown {
            name: "UNKNOWN".to_string(),
            data: raw,
        };
        self.inner.update(&event);
        Ok(())
    }

    pub fn update_event(&mut self, name: &str, payload_json: &str) -> Result<(), String> {
        let raw = RawValue::from_string(payload_json.to_string())
            .map_err(|e| format!("Invalid JSON: {e}"))?;
        let event = GatewayEvent::parse(name, &raw)
            .map_err(|e| format!("Failed to parse {name}: {e}"))?;
        self.inner.update(&event);
        Ok(())
    }

    /// Returns (guilds, visible_channels, obfuscated_channels, roles, members, users, messages)
    pub fn stats(&self) -> (usize, usize, usize, usize, usize, usize, usize) {
        let s = self.inner.stats();
        (
            s.guilds,
            s.visible_channels,
            s.obfuscated_channels,
            s.roles,
            s.members,
            s.users,
            s.messages,
        )
    }

    pub fn is_channel_obfuscated(&self, channel_id: u64) -> bool {
        self.inner.is_channel_obfuscated(Snowflake::new(channel_id))
    }

    pub fn get_guild(&self, guild_id: u64) -> Option<String> {
        let g = self.inner.guild(Snowflake::new(guild_id))?;
        serde_json::to_string(g).ok()
    }

    pub fn get_channel(&self, channel_id: u64) -> Option<String> {
        let ch = self.inner.channel(Snowflake::new(channel_id))?;
        serde_json::to_string(ch).ok()
    }

    pub fn get_member(&self, guild_id: u64, user_id: u64) -> Option<String> {
        let m = self
            .inner
            .member(Snowflake::new(guild_id), Snowflake::new(user_id))?;
        serde_json::to_string(m).ok()
    }

    pub fn get_message(&self, channel_id: u64, message_id: u64) -> Option<String> {
        let msg = self
            .inner
            .message(Snowflake::new(channel_id), Snowflake::new(message_id))?;
        serde_json::to_string(msg).ok()
    }

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

    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

#[cfg(feature = "pyo3-binding")]
pub mod pyo3_impl {
    use super::*;
    use pyo3::prelude::*;

    #[pyclass(name = "Cache")]
    pub struct PyCacheClass {
        inner: PyCache,
    }

    #[pymethods]
    impl PyCacheClass {
        #[new]
        #[pyo3(signature = (max_messages_per_channel=None, max_members_per_guild=None, max_users=None))]
        pub fn new(
            max_messages_per_channel: Option<usize>,
            max_members_per_guild: Option<usize>,
            max_users: Option<usize>,
        ) -> Self {
            Self {
                inner: PyCache::new(max_messages_per_channel, max_members_per_guild, max_users),
            }
        }

        pub fn update(&mut self, event_json: &str) -> PyResult<()> {
            self.inner
                .update(event_json)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))
        }

        pub fn update_event(&mut self, name: &str, payload_json: &str) -> PyResult<()> {
            self.inner
                .update_event(name, payload_json)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))
        }

        pub fn stats(&self) -> (usize, usize, usize, usize, usize, usize, usize) {
            self.inner.stats()
        }

        pub fn is_channel_obfuscated(&self, channel_id: u64) -> bool {
            self.inner.is_channel_obfuscated(channel_id)
        }

        pub fn get_guild(&self, guild_id: u64) -> Option<String> {
            self.inner.get_guild(guild_id)
        }

        pub fn get_channel(&self, channel_id: u64) -> Option<String> {
            self.inner.get_channel(channel_id)
        }

        pub fn get_member(&self, guild_id: u64, user_id: u64) -> Option<String> {
            self.inner.get_member(guild_id, user_id)
        }

        pub fn get_message(&self, channel_id: u64, message_id: u64) -> Option<String> {
            self.inner.get_message(channel_id, message_id)
        }

        pub fn compute_member_permissions(&self, guild_id: u64, channel_id: u64, user_id: u64) -> u64 {
            self.inner
                .compute_member_permissions(guild_id, channel_id, user_id)
        }

        pub fn clear(&mut self) {
            self.inner.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_py_cache_obfuscation() {
        let mut cache = PyCache::default();
        let ch_json = r#"{
            "id": "777",
            "type": 0,
            "flags": 131072,
            "name": "hidden"
        }"#;
        assert!(cache.update_event("CHANNEL_CREATE", ch_json).is_ok());
        assert!(cache.is_channel_obfuscated(777));
        assert!(cache.get_channel(777).is_none());
    }
}
