//! Node/Bun bindings for Rift: high-performance Discord engine in Rust.

pub mod bridge;
pub mod cache;
pub mod client;
pub mod gateway;
pub mod intents;
pub mod limiter;
pub mod permissions;
pub mod snowflake;
pub mod verify;

pub use bridge::dispatch_call;
pub use cache::{CacheConfigInput, CacheStatsOutput, NodeCache};
pub use client::{ClientEventOutput, NodeRiftClient, ShardCommand};
pub use gateway::{HeartbeatPollResult, NodeGatewaySession, ReconnectPlanResult};
pub use limiter::{AcquireVerdict, NodeRateLimiter, NormalizedRoute, ResponseHeadersInput};
pub use permissions::OverwriteInput;

#[cfg(feature = "napi-binding")]
mod napi_exports {
    use super::*;
    use napi::bindgen_prelude::*;
    use napi_derive::napi;

    #[napi]
    pub fn snowflake_timestamp_ms(id_str: String) -> Result<i64> {
        let id = snowflake::parse_snowflake(&id_str)
            .map_err(|e| Error::new(Status::InvalidArg, e))?;
        Ok(snowflake::timestamp_ms(id) as i64)
    }

    #[napi]
    pub fn snowflake_worker_id(id_str: String) -> Result<u32> {
        let id = snowflake::parse_snowflake(&id_str)
            .map_err(|e| Error::new(Status::InvalidArg, e))?;
        Ok(snowflake::worker_id(id) as u32)
    }

    #[napi]
    pub fn snowflake_process_id(id_str: String) -> Result<u32> {
        let id = snowflake::parse_snowflake(&id_str)
            .map_err(|e| Error::new(Status::InvalidArg, e))?;
        Ok(snowflake::process_id(id) as u32)
    }

    #[napi]
    pub fn snowflake_increment(id_str: String) -> Result<u32> {
        let id = snowflake::parse_snowflake(&id_str)
            .map_err(|e| Error::new(Status::InvalidArg, e))?;
        Ok(snowflake::increment(id) as u32)
    }

    #[napi]
    pub fn snowflake_from_parts(
        timestamp_ms: i64,
        worker_id: u32,
        process_id: u32,
        increment: u32,
    ) -> String {
        let sf = snowflake::from_parts(
            timestamp_ms as u64,
            worker_id as u8,
            process_id as u8,
            increment as u16,
        );
        sf.to_string()
    }

    #[napi]
    pub fn verify_interaction(
        public_key_hex: String,
        signature_hex: String,
        timestamp: String,
        body: Buffer,
    ) -> bool {
        verify::verify_interaction(&public_key_hex, &signature_hex, &timestamp, body.as_ref())
    }

    #[napi(object)]
    pub struct JsNormalizedRoute {
        pub id: String,
        pub template: String,
        pub major: String,
    }

    #[napi]
    pub fn normalize_route(method: String, path: String) -> JsNormalizedRoute {
        let r = limiter::normalize_route(&method, &path);
        JsNormalizedRoute {
            id: r.id,
            template: r.template,
            major: r.major,
        }
    }

    #[napi]
    pub fn has_permission(permissions: String, required: String) -> Result<bool> {
        let p = snowflake::parse_snowflake(&permissions)
            .map_err(|e| Error::new(Status::InvalidArg, e))?;
        let r = snowflake::parse_snowflake(&required)
            .map_err(|e| Error::new(Status::InvalidArg, e))?;
        Ok(permissions::has_permission(p, r))
    }

    #[napi]
    pub fn dispatch_bridge(method: String, params_json: String) -> Result<String> {
        dispatch_call(&method, &params_json).map_err(|e| Error::new(Status::GenericFailure, e))
    }
}

#[cfg(feature = "napi-binding")]
pub use napi_exports::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_modules_compile_and_link() {
        let sf = snowflake::from_parts(1_462_015_105_796, 1, 0, 7);
        assert_eq!(sf, 175_928_847_299_117_063);

        assert!(permissions::has_permission(permissions::ADMINISTRATOR, permissions::VIEW_CHANNEL));

        assert_eq!(intents::CAPABILITY_CHANNEL_OBFUSCATION, 1 << 15);

        assert!(!verify::verify_interaction("bad", "sig", "123", b"test"));

        let route = limiter::normalize_route("GET", "/guilds/100/channels");
        assert_eq!(route.major, "guilds/100");

        let cache = NodeCache::default();
        assert_eq!(cache.stats().guilds, 0);

        let mut gw = NodeGatewaySession::new();
        assert_eq!(gw.poll_heartbeat(0).action, "wait");

        let client = NodeRiftClient::new("token".to_string(), None, None);
        assert_eq!(client.capabilities(), 0);
    }
}
