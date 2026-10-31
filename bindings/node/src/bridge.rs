use crate::cache::NodeCache;
use crate::gateway::NodeGatewaySession;
use crate::limiter::{normalize_route, NodeRateLimiter, ResponseHeadersInput};
use crate::permissions::{compute_channel_permissions, has_permission, OverwriteInput};
use crate::snowflake::{deconstruct, from_parts, parse_snowflake};
use crate::verify::verify_interaction;
use serde_json::{json, Value};
use std::sync::Mutex;

static GLOBAL_CACHE: Mutex<Option<NodeCache>> = Mutex::new(None);
static GLOBAL_LIMITER: Mutex<Option<NodeRateLimiter>> = Mutex::new(None);
static GLOBAL_GATEWAY: Mutex<Option<NodeGatewaySession>> = Mutex::new(None);

/// Dispatches a method call with JSON string parameters and returns a JSON string result.
/// This acts as a universal bridge across N-API, worker threads, and subprocesses.
pub fn dispatch_call(method: &str, params_json: &str) -> Result<String, String> {
    let params: Value = serde_json::from_str(params_json)
        .map_err(|e| format!("Invalid JSON params: {e}"))?;

    match method {
        // Snowflake
        "snowflake.timestampMs" => {
            let id = extract_u64(&params["id"])?;
            let ts = crate::snowflake::timestamp_ms(id);
            Ok(json!({ "timestampMs": ts }).to_string())
        }
        "snowflake.workerId" => {
            let id = extract_u64(&params["id"])?;
            let wid = crate::snowflake::worker_id(id);
            Ok(json!({ "workerId": wid }).to_string())
        }
        "snowflake.processId" => {
            let id = extract_u64(&params["id"])?;
            let pid = crate::snowflake::process_id(id);
            Ok(json!({ "processId": pid }).to_string())
        }
        "snowflake.increment" => {
            let id = extract_u64(&params["id"])?;
            let inc = crate::snowflake::increment(id);
            Ok(json!({ "increment": inc }).to_string())
        }
        "snowflake.fromParts" => {
            let ts = params["timestampMs"].as_u64().ok_or("Missing timestampMs")?;
            let wid = params["workerId"].as_u64().ok_or("Missing workerId")? as u8;
            let pid = params["processId"].as_u64().ok_or("Missing processId")? as u8;
            let inc = params["increment"].as_u64().ok_or("Missing increment")? as u16;
            let sf = from_parts(ts, wid, pid, inc);
            Ok(json!({ "id": sf.to_string() }).to_string())
        }
        "snowflake.deconstruct" => {
            let id = extract_u64(&params["id"])?;
            let parts = deconstruct(id);
            Ok(serde_json::to_string(&parts).map_err(|e| e.to_string())?)
        }

        // Permissions
        "permissions.hasPermission" => {
            let p = extract_u64(&params["permissions"])?;
            let req = extract_u64(&params["required"])?;
            Ok(json!({ "hasPermission": has_permission(p, req) }).to_string())
        }
        "permissions.computeChannelPermissions" => {
            let base = extract_u64(&params["basePermissions"])?;
            let guild_id = extract_u64(&params["guildId"])?;
            let member_id = extract_u64(&params["memberId"])?;
            let member_roles = extract_u64_array(&params["memberRoles"])?;

            let mut overwrites = Vec::new();
            if let Some(arr) = params["overwrites"].as_array() {
                for item in arr {
                    overwrites.push(OverwriteInput {
                        id: extract_u64(&item["id"])?,
                        kind: item["type"].as_u64().unwrap_or(0) as u8,
                        allow: extract_u64(&item["allow"])?,
                        deny: extract_u64(&item["deny"])?,
                    });
                }
            }

            let result = compute_channel_permissions(base, &overwrites, guild_id, member_id, &member_roles);
            Ok(json!({ "permissions": result.to_string() }).to_string())
        }

        // Verify Interaction
        "verifyInteraction" => {
            let pk = params["publicKeyHex"].as_str().ok_or("Missing publicKeyHex")?;
            let sig = params["signatureHex"].as_str().ok_or("Missing signatureHex")?;
            let ts = params["timestamp"].as_str().ok_or("Missing timestamp")?;
            let body = if let Some(s) = params["body"].as_str() {
                s.as_bytes().to_vec()
            } else if let Some(arr) = params["body"].as_array() {
                arr.iter().filter_map(|v| v.as_u64().map(|b| b as u8)).collect()
            } else {
                return Err("Missing body".to_string());
            };

            let valid = verify_interaction(pk, sig, ts, &body);
            Ok(json!({ "valid": valid }).to_string())
        }

        // Route Normalization
        "normalizeRoute" => {
            let method = params["method"].as_str().ok_or("Missing method")?;
            let path = params["path"].as_str().ok_or("Missing path")?;
            let norm = normalize_route(method, path);
            Ok(serde_json::to_string(&norm).map_err(|e| e.to_string())?)
        }

        // Rate Limiter
        "rateLimiter.init" => {
            let mut lock = GLOBAL_LIMITER.lock().unwrap();
            let gps = params["globalPerSecond"].as_u64().map(|v| v as u32);
            let il = params["invalidLimit"].as_u64().map(|v| v as u32);
            let ita = params["invalidTripAt"].as_u64().map(|v| v as u32);
            *lock = Some(NodeRateLimiter::new(gps, il, ita));
            Ok(json!({ "initialized": true }).to_string())
        }
        "rateLimiter.acquire" => {
            let mut lock = GLOBAL_LIMITER.lock().unwrap();
            let limiter = lock.get_or_insert_with(|| NodeRateLimiter::new(None, None, None));
            let method = params["method"].as_str().ok_or("Missing method")?;
            let path = params["path"].as_str().ok_or("Missing path")?;
            let now_ms = params["nowMs"].as_u64().unwrap_or(0);
            let verdict = limiter.acquire(method, path, now_ms);
            Ok(serde_json::to_string(&verdict).map_err(|e| e.to_string())?)
        }
        "rateLimiter.onResponse" => {
            let mut lock = GLOBAL_LIMITER.lock().unwrap();
            let limiter = lock.get_or_insert_with(|| NodeRateLimiter::new(None, None, None));
            let method = params["method"].as_str().ok_or("Missing method")?;
            let path = params["path"].as_str().ok_or("Missing path")?;
            let status = params["status"].as_u64().ok_or("Missing status")? as u16;
            let now_ms = params["nowMs"].as_u64().unwrap_or(0);

            let headers = ResponseHeadersInput {
                limit: params["limit"].as_u64().map(|v| v as u32),
                remaining: params["remaining"].as_u64().map(|v| v as u32),
                reset_after_ms: params["resetAfterMs"].as_u64(),
                bucket: params["bucket"].as_str().map(str::to_string),
                global: params["global"].as_bool(),
                scope: params["scope"].as_str().map(str::to_string),
                retry_after_ms: params["retryAfterMs"].as_u64(),
            };

            limiter.on_response(method, path, status, headers, now_ms);
            Ok(json!({ "success": true }).to_string())
        }

        // Cache
        "cache.init" => {
            let mut lock = GLOBAL_CACHE.lock().unwrap();
            *lock = Some(NodeCache::default());
            Ok(json!({ "initialized": true }).to_string())
        }
        "cache.update" => {
            let mut lock = GLOBAL_CACHE.lock().unwrap();
            let cache = lock.get_or_insert_with(NodeCache::default);
            let evt = params["eventJson"].as_str().ok_or("Missing eventJson")?;
            cache.update(evt)?;
            Ok(json!({ "success": true }).to_string())
        }
        "cache.stats" => {
            let mut lock = GLOBAL_CACHE.lock().unwrap();
            let cache = lock.get_or_insert_with(NodeCache::default);
            Ok(serde_json::to_string(&cache.stats()).map_err(|e| e.to_string())?)
        }
        "cache.isChannelObfuscated" => {
            let mut lock = GLOBAL_CACHE.lock().unwrap();
            let cache = lock.get_or_insert_with(NodeCache::default);
            let cid = extract_u64(&params["channelId"])?;
            Ok(json!({ "obfuscated": cache.is_channel_obfuscated(cid) }).to_string())
        }
        "cache.computeMemberPermissions" => {
            let mut lock = GLOBAL_CACHE.lock().unwrap();
            let cache = lock.get_or_insert_with(NodeCache::default);
            let gid = extract_u64(&params["guildId"])?;
            let cid = extract_u64(&params["channelId"])?;
            let uid = extract_u64(&params["userId"])?;
            let perms = cache.compute_member_permissions(gid, cid, uid);
            Ok(json!({ "permissions": perms.to_string() }).to_string())
        }

        // Gateway Session
        "gateway.init" => {
            let mut lock = GLOBAL_GATEWAY.lock().unwrap();
            *lock = Some(NodeGatewaySession::new());
            Ok(json!({ "initialized": true }).to_string())
        }
        "gateway.onHello" => {
            let mut lock = GLOBAL_GATEWAY.lock().unwrap();
            let gw = lock.get_or_insert_with(NodeGatewaySession::new);
            let now_ms = params["nowMs"].as_u64().unwrap_or(0);
            let interval = params["intervalMs"].as_u64().ok_or("Missing intervalMs")?;
            let jitter = params["jitter"].as_f64().unwrap_or(0.5);
            gw.on_hello(now_ms, interval, jitter);
            Ok(json!({ "success": true }).to_string())
        }
        "gateway.onReady" => {
            let mut lock = GLOBAL_GATEWAY.lock().unwrap();
            let gw = lock.get_or_insert_with(NodeGatewaySession::new);
            let sess_id = params["sessionId"].as_str().ok_or("Missing sessionId")?;
            let url = params["resumeGatewayUrl"].as_str().ok_or("Missing resumeGatewayUrl")?;
            gw.on_ready(sess_id.to_string(), url.to_string());
            Ok(json!({ "success": true }).to_string())
        }
        "gateway.pollHeartbeat" => {
            let mut lock = GLOBAL_GATEWAY.lock().unwrap();
            let gw = lock.get_or_insert_with(NodeGatewaySession::new);
            let now_ms = params["nowMs"].as_u64().unwrap_or(0);
            let poll = gw.poll_heartbeat(now_ms);
            Ok(serde_json::to_string(&poll).map_err(|e| e.to_string())?)
        }
        "gateway.planReconnect" => {
            let mut lock = GLOBAL_GATEWAY.lock().unwrap();
            let gw = lock.get_or_insert_with(NodeGatewaySession::new);
            let why = params["why"].as_str().unwrap_or("close");
            let code = params["closeCode"].as_u64().map(|v| v as u16);
            let resumable = params["resumable"].as_bool();
            let plan = gw.plan_reconnect(why, code, resumable);
            Ok(serde_json::to_string(&plan).map_err(|e| e.to_string())?)
        }

        _ => Err(format!("Unknown method: {method}")),
    }
}

fn extract_u64(val: &Value) -> Result<u64, String> {
    if let Some(n) = val.as_u64() {
        Ok(n)
    } else if let Some(s) = val.as_str() {
        parse_snowflake(s)
    } else {
        Err(format!("Expected integer or numeric string, got: {val:?}"))
    }
}

fn extract_u64_array(val: &Value) -> Result<Vec<u64>, String> {
    let mut out = Vec::new();
    if let Some(arr) = val.as_array() {
        for v in arr {
            out.push(extract_u64(v)?);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatch_snowflake_and_permissions() {
        let res = dispatch_call(
            "snowflake.timestampMs",
            r#"{"id": "175928847299117063"}"#,
        )
        .unwrap();
        assert!(res.contains("1462015105796"));

        let res2 = dispatch_call(
            "permissions.hasPermission",
            r#"{"permissions": "8", "required": "2048"}"#,
        )
        .unwrap();
        assert!(res2.contains("true"));
    }

    #[test]
    fn test_dispatch_normalize_route() {
        let res = dispatch_call(
            "normalizeRoute",
            r#"{"method": "GET", "path": "/channels/123/messages"}"#,
        )
        .unwrap();
        assert!(res.contains("channels/:major/messages"));
    }
}
