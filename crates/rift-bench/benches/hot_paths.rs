//! Microbenchmarks of rift's hot paths, each measured in isolation.
//!
//! READ THIS BEFORE QUOTING ANY NUMBER: these measure single components on
//! synthetic-but-realistic data. They are NOT comparisons with other libraries
//! and say nothing about end-to-end throughput, memory or latency. See
//! METHODOLOGY.md for how credible cross-library numbers get produced.
//!
//! Run:  cargo bench -p rift-bench        (HTML report: target/criterion/report/index.html)

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use ed25519_dalek::Signer;
use rift_cache::Cache;
use rift_core::permissions::{apply_implicit_text, compute_base, compute_overwrites, Overwrite, Permissions};
use rift_core::shard::shard_for_guild;
use rift_core::Snowflake;
use rift_http::verify::verify_interaction as verify_interaction_request;
use rift_http::{RateLimitHeaders, RateLimiter, Route};
use rift_model::GatewayEvent;
use rift_rest::component::{
    validate_components_v2, ActionRow, Button, Component, Container, Section, Separator,
    TextDisplay,
};
use rift_rest::{AllowedMentions, CreateMessage, Interaction};
use rift_runtime::bench_support::{envelope_probe, StreamDecoder};
use serde_json::json;
use std::hint::black_box;
use std::io::Write;
use std::num::NonZeroU32;

// ---------- synthetic data ----------

fn id(n: u64) -> String {
    (1_100_000_000_000_000_000u64 + n * 1_337_001).to_string()
}

/// A realistic MESSAGE_CREATE frame (~1 KB of JSON). Fields vary per message so
/// the compressor can't cheat by seeing identical frames.
fn message_create(i: u64) -> Vec<u8> {
    let v = json!({
        "op": 0, "s": i + 1, "t": "MESSAGE_CREATE",
        "d": {
            "id": id(i), "channel_id": id(7), "guild_id": id(3), "type": 0,
            "content": format!("message {i}: the quick brown fox jumps over the lazy dog {}", i.wrapping_mul(7919) % 10_007),
            "author": {
                "id": id(i % 500), "username": format!("user{}", i % 500), "discriminator": "0",
                "avatar": "a1b2c3d4e5f60718293a4b5c6d7e8f90", "global_name": format!("User {}", i % 500),
                "public_flags": 0
            },
            "member": {
                "roles": [id(11), id(12)], "joined_at": "2024-03-01T12:00:00.000000+00:00",
                "nick": null, "flags": 0, "mute": false, "deaf": false
            },
            "timestamp": "2026-10-30T06:44:00.123000+00:00", "edited_timestamp": null, "tts": false,
            "mention_everyone": false, "mentions": [], "mention_roles": [], "attachments": [],
            "embeds": [], "pinned": false, "components": [], "flags": 0
        }
    });
    serde_json::to_vec(&v).unwrap()
}

/// A large GUILD_CREATE-like frame: `members` member objects.
fn guild_create(members: u64) -> Vec<u8> {
    let list: Vec<_> = (0..members)
        .map(|i| {
            json!({
                "user": { "id": id(i), "username": format!("user{i}"), "avatar": "a1b2c3d4e5f60718293a4b5c6d7e8f90", "bot": false },
                "roles": [id(11), id(i % 40)], "joined_at": "2024-03-01T12:00:00.000000+00:00",
                "nick": null, "flags": 0, "mute": false, "deaf": false
            })
        })
        .collect();
    serde_json::to_vec(&json!({
        "op": 0, "s": 2, "t": "GUILD_CREATE",
        "d": { "id": id(3), "name": "bench guild", "member_count": members, "members": list }
    }))
    .unwrap()
}

/// Compress like Discord's `zstd-stream`: one long-lived encoder, flushed after
/// every message. (Level 3 is an assumption; Discord's level isn't documented.)
fn encode_stream(msgs: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let mut enc = zstd::stream::write::Encoder::new(Vec::new(), 3).unwrap();
    msgs.iter()
        .map(|m| {
            enc.write_all(m).unwrap();
            enc.flush().unwrap();
            std::mem::take(enc.get_mut())
        })
        .collect()
}

// ---------- benchmarks ----------

fn gateway_decode(c: &mut Criterion) {
    const N: u64 = 1_000;
    let raw: Vec<Vec<u8>> = (0..N).map(message_create).collect();
    let raw_bytes: usize = raw.iter().map(Vec::len).sum();
    let frames = encode_stream(&raw);
    let wire_bytes: usize = frames.iter().map(Vec::len).sum();
    eprintln!(
        "[setup] {N} MESSAGE_CREATE frames: {raw_bytes} B of JSON -> {wire_bytes} B on the wire ({:.1}x)",
        raw_bytes as f64 / wire_bytes as f64
    );

    let mut g = c.benchmark_group("gateway_decode");
    g.throughput(Throughput::Elements(N));

    // One decoder per iteration, reused across all N frames (as on a real connection).
    g.bench_function("zstd_stream/message_create", |b| {
        b.iter(|| {
            let mut d = StreamDecoder::new().unwrap();
            for f in &frames {
                black_box(d.decode(f).unwrap());
            }
        })
    });

    // What the driver does per frame before your code sees it: decompress + parse the envelope.
    g.bench_function("zstd_stream+envelope/message_create", |b| {
        b.iter(|| {
            let mut d = StreamDecoder::new().unwrap();
            for f in &frames {
                let out = d.decode(f).unwrap();
                black_box(envelope_probe(out));
            }
        })
    });

    // Envelope parse alone (no decompression), for the share it contributes.
    g.bench_function("envelope_only/message_create", |b| {
        b.iter(|| {
            for m in &raw {
                black_box(envelope_probe(m));
            }
        })
    });
    g.finish();

    // One big frame: measures sustained decompression bandwidth.
    let big = guild_create(3_000);
    let big_wire = encode_stream(std::slice::from_ref(&big));
    eprintln!("[setup] GUILD_CREATE(3000 members): {} B of JSON -> {} B on the wire", big.len(), big_wire[0].len());
    let mut g = c.benchmark_group("gateway_decode_large");
    g.sample_size(30);
    g.throughput(Throughput::Bytes(big.len() as u64));
    g.bench_function("zstd_stream/guild_create_3000_members", |b| {
        b.iter(|| {
            let mut d = StreamDecoder::new().unwrap();
            black_box(d.decode(&big_wire[0]).unwrap());
        })
    });
    g.bench_function("zstd_stream+envelope/guild_create_3000_members", |b| {
        b.iter(|| {
            let mut d = StreamDecoder::new().unwrap();
            let out = d.decode(&big_wire[0]).unwrap();
            black_box(envelope_probe(out));
        })
    });
    g.bench_function("envelope_only(serde)/guild_create_3000_members", |b| {
        b.iter(|| black_box(envelope_probe(&big)))
    });
    g.finish();
}

fn interactions(c: &mut Criterion) {
    const SLASH: &str = r#"{
        "id": "1000", "application_id": "2000", "type": 2, "token": "tok123", "version": 1,
        "guild_id": "3000", "channel_id": "4000", "locale": "en-US", "app_permissions": "1",
        "member": { "user": { "id": "5000", "username": "gugu" }, "permissions": "2147483647", "nick": null },
        "data": { "id": "6000", "name": "echo", "type": 1, "options": [
            { "name": "text", "type": 3, "value": "hello world" },
            { "name": "shout", "type": 5, "value": true }
        ] }
    }"#;

    let mut g = c.benchmark_group("interactions");
    g.bench_function("parse_interaction", |b| {
        b.iter(|| black_box(serde_json::from_str::<Interaction>(black_box(SLASH)).unwrap()))
    });
    g.bench_function("parse_interaction+command_options", |b| {
        b.iter(|| {
            let i: Interaction = serde_json::from_str(black_box(SLASH)).unwrap();
            let cmd = i.command().unwrap();
            black_box((cmd.option("text").and_then(|o| o.as_str()).map(str::len), cmd.option("shout").and_then(|o| o.as_bool())));
        })
    });
    g.finish();
}

fn rest_serialize(c: &mut Criterion) {
    let mut g = c.benchmark_group("rest");
    g.bench_function("create_message/build+validate+serialize", |b| {
        b.iter(|| {
            let m = CreateMessage::new()
                .content(black_box("hello world"))
                .reply_to(Snowflake::new(5))
                .allowed_mentions(AllowedMentions::none());
            m.validate().unwrap();
            black_box(serde_json::to_vec(&m).unwrap());
        })
    });
    g.finish();
}

fn core_paths(c: &mut Criterion) {
    let mut g = c.benchmark_group("core");

    let ids: Vec<Snowflake> = (0..1_000).map(|i| Snowflake::new(1_100_000_000_000_000_000 + i * 7_777_777)).collect();
    let shards = NonZeroU32::new(16).unwrap();
    g.throughput(Throughput::Elements(ids.len() as u64));
    g.bench_function("shard_for_guild/1000_ids", |b| {
        b.iter(|| {
            let mut acc = 0u32;
            for &i in &ids {
                acc = acc.wrapping_add(shard_for_guild(black_box(i), shards));
            }
            black_box(acc)
        })
    });

    let roles: Vec<Permissions> = (0..20).map(|i| Permissions::from_bits_retain(1u64 << (i % 40))).collect();
    let overwrites: Vec<Overwrite> = (0..5)
        .map(|i| Overwrite {
            allow: Permissions::from_bits_retain(1u64 << (i + 3)),
            deny: Permissions::from_bits_retain(1u64 << (i + 10)),
        })
        .collect();
    g.throughput(Throughput::Elements(1));
    g.bench_function("permissions/20_roles_5_overwrites", |b| {
        b.iter(|| {
            let base = compute_base(false, Permissions::VIEW_CHANNEL, roles.iter().copied());
            let p = compute_overwrites(black_box(base), Some(overwrites[0]), overwrites.iter().copied(), Some(overwrites[1]));
            black_box(apply_implicit_text(p))
        })
    });
    g.finish();
}

fn rate_limiter(c: &mut Criterion) {
    let mut g = c.benchmark_group("http_limiter");

    g.bench_function("route_normalize", |b| {
        b.iter(|| {
            black_box(Route::new(
                black_box("POST"),
                black_box("/channels/123456789012345678/messages/987654321098765432"),
            ))
        })
    });

    // acquire + on_response across 100 channels, paced at 40 req/s so the
    // global limit never kicks in (we measure the bookkeeping, not waiting).
    let routes: Vec<Route> = (0..100).map(|i| Route::new("POST", &format!("/channels/{}/messages", id(i)))).collect();
    let headers = RateLimitHeaders {
        limit: Some(5),
        remaining: Some(4),
        reset_after_ms: Some(1_000),
        bucket: Some("bucket".into()),
        ..Default::default()
    };
    let mut limiter = RateLimiter::default();
    let mut now = 0u64;
    g.bench_function("acquire+on_response/100_channels", |b| {
        b.iter(|| {
            now += 25;
            let r = &routes[((now / 25) % 100) as usize];
            let verdict = limiter.acquire(r, now);
            limiter.on_response(r, 200, &headers, now);
            black_box(verdict)
        })
    });
    g.finish();
}

fn hex_encode(data: &[u8]) -> String {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(data.len() * 2);
    for &b in data {
        s.push(HEX_CHARS[(b >> 4) as usize] as char);
        s.push(HEX_CHARS[(b & 0x0f) as usize] as char);
    }
    s
}

fn model_and_cache(c: &mut Criterion) {
    let mut g = c.benchmark_group("model_and_cache");

    // 1. parse_message_create_typed
    let msg_json = json!({
        "id": id(1),
        "channel_id": id(7),
        "guild_id": id(3),
        "type": 0,
        "content": "the quick brown fox jumps over the lazy dog 42",
        "author": {
            "id": id(42),
            "username": "user42",
            "discriminator": "0",
            "avatar": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
            "global_name": "User 42",
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
    });
    let raw_msg_str = serde_json::to_string(&msg_json).unwrap();

    g.bench_function("parse_message_create_typed", |b| {
        b.iter(|| {
            black_box(GatewayEvent::parse_str("MESSAGE_CREATE", black_box(&raw_msg_str)).unwrap())
        })
    });

    // Setup for cache benchmarks
    const MEMBER_COUNT: u64 = 100;
    let guild_id = Snowflake::new(id(3).parse::<u64>().unwrap());
    let channel_id = Snowflake::new(id(7).parse::<u64>().unwrap());
    let obfuscated_channel_id = Snowflake::new(id(8).parse::<u64>().unwrap());

    let members_json: Vec<_> = (0..MEMBER_COUNT)
        .map(|i| {
            json!({
                "user": {
                    "id": id(i),
                    "username": format!("user{i}"),
                    "discriminator": "0",
                    "avatar": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
                    "bot": false
                },
                "roles": [id(11), id(12)],
                "joined_at": "2024-03-01T12:00:00.000000+00:00",
                "nick": null,
                "flags": 0,
                "mute": false,
                "deaf": false
            })
        })
        .collect();

    let guild_json = json!({
        "id": id(3),
        "name": "bench guild",
        "owner_id": id(999_999),
        "member_count": MEMBER_COUNT,
        "roles": [
            {
                "id": id(3),
                "name": "@everyone",
                "color": 0,
                "hoist": false,
                "position": 0,
                "permissions": "104320577",
                "managed": false,
                "mentionable": false,
                "flags": 0
            },
            {
                "id": id(11),
                "name": "moderator",
                "color": 0,
                "hoist": false,
                "position": 1,
                "permissions": "268435456",
                "managed": false,
                "mentionable": false,
                "flags": 0
            },
            {
                "id": id(12),
                "name": "member",
                "color": 0,
                "hoist": false,
                "position": 2,
                "permissions": "2048",
                "managed": false,
                "mentionable": false,
                "flags": 0
            }
        ],
        "channels": [
            {
                "id": id(7),
                "guild_id": id(3),
                "name": "general",
                "type": 0,
                "position": 0,
                "flags": 0,
                "permission_overwrites": [
                    {
                        "id": id(11),
                        "type": 0,
                        "allow": "2048",
                        "deny": "0"
                    }
                ]
            },
            {
                "id": id(8),
                "guild_id": id(3),
                "name": "hidden",
                "type": 0,
                "position": 1,
                "flags": 131072, // CHANNEL_OBFUSCATED (1 << 17)
                "permission_overwrites": []
            }
        ],
        "members": members_json,
        "threads": [],
        "emojis": [],
        "features": []
    });

    let guild_create_event = GatewayEvent::parse_str("GUILD_CREATE", &serde_json::to_string(&guild_json).unwrap()).unwrap();
    let message_create_event = GatewayEvent::parse_str("MESSAGE_CREATE", &raw_msg_str).unwrap();
    let member_ids: Vec<Snowflake> = (0..MEMBER_COUNT).map(|i| Snowflake::new(id(i).parse::<u64>().unwrap())).collect();

    // 2. cache_update_and_permissions
    g.throughput(Throughput::Elements(MEMBER_COUNT));
    g.bench_function("cache_update_and_permissions", |b| {
        b.iter(|| {
            let mut cache = Cache::default();
            cache.update(black_box(&guild_create_event));
            cache.update(black_box(&message_create_event));
            for &uid in &member_ids {
                black_box(cache.member_permissions_in_channel(
                    black_box(guild_id),
                    black_box(channel_id),
                    black_box(uid),
                ));
            }
        })
    });

    // 3. cache_obfuscated_channel_guard
    let mut preloaded_cache = Cache::default();
    preloaded_cache.update(&guild_create_event);
    let test_user = member_ids[0];

    g.throughput(Throughput::Elements(1));
    g.bench_function("cache_obfuscated_channel_guard", |b| {
        b.iter(|| {
            black_box(preloaded_cache.member_permissions_in_channel(
                black_box(guild_id),
                black_box(obfuscated_channel_id),
                black_box(test_user),
            ))
        })
    });

    g.finish();
}

fn serverless_and_components(c: &mut Criterion) {
    let mut g = c.benchmark_group("serverless_and_components");

    // 1. ed25519_verify_interaction
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);
    let verifying_key = signing_key.verifying_key();
    let pk_hex = hex_encode(verifying_key.as_bytes());
    let timestamp = "1700000000";
    let body = br#"{"id":"1234567890","type":1,"token":"test_token","version":1}"#;
    let mut signed_msg = Vec::with_capacity(timestamp.len() + body.len());
    signed_msg.extend_from_slice(timestamp.as_bytes());
    signed_msg.extend_from_slice(body);
    let sig = signing_key.sign(&signed_msg);
    let sig_hex = hex_encode(&sig.to_bytes());

    // Sanity check
    verify_interaction_request(&pk_hex, &sig_hex, timestamp, body).expect("verify interaction");

    g.bench_function("ed25519_verify_interaction", |b| {
        b.iter(|| {
            verify_interaction_request(
                black_box(&pk_hex),
                black_box(&sig_hex),
                black_box(timestamp),
                black_box(body),
            )
            .unwrap();
        })
    });

    // 2. components_v2_build_and_validate
    g.bench_function("components_v2_build_and_validate", |b| {
        b.iter(|| {
            let container = Container::new(vec![
                Section::new(vec![
                    TextDisplay::new(black_box("**Cluster Status:** All shards operational.")),
                    TextDisplay::new(black_box("Latency: 14ms | Memory: 42MB | Shards: 16/16")),
                ])
                .accessory(Button::primary(black_box("status_refresh"), black_box("Refresh Status")))
                .into(),
                Separator::new().divider(true).into(),
                Section::new(vec![
                    TextDisplay::new(black_box("Deployment v2.4.0 verified across all regions.")),
                ])
                .accessory(Button::secondary(black_box("rollback"), black_box("Rollback")))
                .into(),
                ActionRow::buttons(vec![
                    Button::success(black_box("approve_btn"), black_box("Approve")),
                    Button::danger(black_box("reject_btn"), black_box("Reject")),
                ])
                .into(),
            ])
            .accent_color(black_box(0x5865F2));

            container.validate().unwrap();
            let top: Component = container.into();
            validate_components_v2(std::slice::from_ref(&top)).unwrap();
            black_box(top)
        })
    });

    g.finish();
}

criterion_group!(
    benches,
    gateway_decode,
    interactions,
    rest_serialize,
    core_paths,
    rate_limiter,
    model_and_cache,
    serverless_and_components,
);
criterion_main!(benches);
