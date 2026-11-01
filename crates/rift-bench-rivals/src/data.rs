//! Deterministic synthetic Discord Gateway events for benchmarking and testing.
//! Identical workload to `crates/rift-bench/src/data.rs`.

use serde_json::json;

/// Generates a deterministic Snowflake-like string ID from an integer.
pub fn id(n: u64) -> String {
    (1_100_000_000_000_000_000u64 + n * 1_337_001).to_string()
}

/// A realistic `READY` payload matching Discord v10.
pub fn ready_payload(session_id: &str, resume_gateway_url: &str) -> Vec<u8> {
    let v = json!({
        "op": 0,
        "s": 1,
        "t": "READY",
        "d": {
            "v": 10,
            "user": {
                "id": id(1),
                "username": "rift-bot",
                "discriminator": "0000",
                "avatar": null,
                "bot": true,
                "mfa_enabled": false,
                "flags": 0
            },
            "guilds": [
                {
                    "id": id(3),
                    "unavailable": true
                }
            ],
            "session_id": session_id,
            "resume_gateway_url": resume_gateway_url,
            "shard": [0, 1],
            "application": {
                "id": id(1),
                "flags": 0
            }
        }
    });
    serde_json::to_vec(&v).expect("serialize READY")
}

/// A realistic `GUILD_CREATE` frame containing members, channels, and roles.
pub fn guild_create(seq: u64, guild_id: u64, members: usize) -> Vec<u8> {
    let list: Vec<_> = (0..members)
        .map(|i| {
            json!({
                "user": {
                    "id": id(i as u64),
                    "username": format!("user{i}"),
                    "discriminator": "0000",
                    "avatar": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
                    "bot": false
                },
                "roles": [id(11), id((i % 40) as u64)],
                "joined_at": "2024-03-01T12:00:00.000000+00:00",
                "nick": null,
                "flags": 0,
                "mute": false,
                "deaf": false
            })
        })
        .collect();

    let v = json!({
        "op": 0,
        "s": seq,
        "t": "GUILD_CREATE",
        "d": {
            "id": id(guild_id),
            "name": "bench-guild",
            "icon": null,
            "splash": null,
            "discovery_splash": null,
            "owner_id": id(1),
            "afk_channel_id": null,
            "afk_timeout": 300,
            "verification_level": 0,
            "default_message_notifications": 0,
            "explicit_content_filter": 0,
            "roles": [
                {
                    "id": id(11),
                    "name": "bench-role",
                    "color": 0,
                    "colors": {
                        "primary_color": 0
                    },
                    "hoist": false,
                    "position": 1,
                    "permissions": "0",
                    "managed": false,
                    "mentionable": false,
                    "flags": 0
                }
            ],
            "emojis": [],
            "features": [],
            "mfa_level": 0,
            "application_id": null,
            "system_channel_id": null,
            "system_channel_flags": 0,
            "rules_channel_id": null,
            "public_updates_channel_id": null,
            "safety_alerts_channel_id": null,
            "premium_progress_bar_enabled": false,
            "nsfw_level": 0,
            "joined_at": "2024-03-01T12:00:00.000000+00:00",
            "large": false,
            "unavailable": false,
            "member_count": members as u64,
            "members": list,
            "channels": [
                {
                    "id": id(7),
                    "type": 0,
                    "guild_id": id(guild_id),
                    "name": "general",
                    "position": 0,
                    "permission_overwrites": []
                }
            ],
            "threads": [],
            "presences": [],
            "stage_instances": [],
            "guild_scheduled_events": [],
            "premium_tier": 0,
            "preferred_locale": "en-US"
        }
    });
    serde_json::to_vec(&v).expect("serialize GUILD_CREATE")
}

/// A realistic `MESSAGE_CREATE` frame (~1 KB of JSON) with varying fields.
pub fn message_create(seq: u64, i: u64) -> Vec<u8> {
    let v = json!({
        "op": 0,
        "s": seq,
        "t": "MESSAGE_CREATE",
        "d": {
            "id": id(i),
            "channel_id": id(7),
            "guild_id": id(3),
            "type": 0,
            "content": format!(
                "message {i}: the quick brown fox jumps over the lazy dog {}",
                i.wrapping_mul(7919) % 10_007
            ),
            "author": {
                "id": id(i % 500),
                "username": format!("user{}", i % 500),
                "discriminator": "0",
                "avatar": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
                "global_name": format!("User {}", i % 500),
                "public_flags": 0
            },
            "member": {
                "roles": [id(11), id(12)],
                "joined_at": "2024-03-01T12:00:00.000000+00:00",
                "nick": null,
                "flags": 0,
                "mute": false,
                "deaf": false
            },
            "timestamp": "2026-10-30T06:44:00.123000+00:00",
            "edited_timestamp": null,
            "tts": false,
            "mention_everyone": false,
            "mentions": [],
            "mention_roles": [],
            "attachments": [],
            "embeds": [],
            "pinned": false,
            "components": [],
            "flags": 0
        }
    });
    serde_json::to_vec(&v).expect("serialize MESSAGE_CREATE")
}

/// Generates a realistic deterministic stream of `num_events` dispatch frames.
pub fn generate_event_stream(num_events: usize, guild_members: usize) -> Vec<Vec<u8>> {
    let mut events = Vec::with_capacity(num_events);
    if num_events == 0 {
        return events;
    }

    events.push(guild_create(2, 3, guild_members));

    for i in 1..num_events {
        let seq = (i + 2) as u64;
        events.push(message_create(seq, i as u64));
    }

    events
}
