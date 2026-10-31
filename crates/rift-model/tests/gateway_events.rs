//! Integration and deserialization tests for Gateway dispatch events.

use rift_core::{Permissions, Snowflake};
use rift_model::*;
use serde_json::value::RawValue;

#[test]
fn test_ready_event() {
    let payload = r#"{
        "v": 10,
        "user": {
            "id": "1088605110638227537",
            "username": "riftbot",
            "discriminator": "0001",
            "bot": true
        },
        "guilds": [
            {
                "id": "197038439483310080",
                "unavailable": true
            }
        ],
        "session_id": "74883f32483848b392934",
        "resume_gateway_url": "wss://gateway.discord.gg",
        "shard": [0, 1],
        "application": {
            "id": "1088605110638227537",
            "flags": 278528
        }
    }"#;

    let event = GatewayEvent::parse_str("READY", payload).expect("parse READY");
    if let GatewayEvent::Ready(ready) = event {
        assert_eq!(ready.v, 10);
        assert_eq!(ready.user.id, Snowflake::new(1088605110638227537));
        assert_eq!(ready.user.username, "riftbot");
        assert_eq!(ready.user.bot, Some(true));
        assert_eq!(ready.guilds.len(), 1);
        assert_eq!(ready.guilds[0].id, Snowflake::new(197038439483310080));
        assert_eq!(ready.guilds[0].unavailable, Some(true));
        assert_eq!(ready.session_id, "74883f32483848b392934");
        assert_eq!(ready.shard, Some([0, 1]));
        assert_eq!(ready.application.id, Snowflake::new(1088605110638227537));
        assert_eq!(ready.application.flags, Some(278528));
    } else {
        panic!("expected Ready variant");
    }
}

#[test]
fn test_resumed_event() {
    let payload = r#"{
        "_trace": ["[\"gateway-prd-main-abc\",{\"micros\":10.0}]"]
    }"#;

    let event = GatewayEvent::parse_str("RESUMED", payload).expect("parse RESUMED");
    if let GatewayEvent::Resumed(resumed) = event {
        assert!(resumed._trace.is_some());
    } else {
        panic!("expected Resumed variant");
    }
}

#[test]
fn test_guild_create_event() {
    let payload = r#"{
        "id": "197038439483310080",
        "name": "Rift Community",
        "icon": "an_icon_hash",
        "owner_id": "80351110224678912",
        "verification_level": 1,
        "default_message_notifications": 0,
        "explicit_content_filter": 0,
        "roles": [
            {
                "id": "197038439483310080",
                "name": "@everyone",
                "color": 0,
                "hoist": false,
                "position": 0,
                "permissions": "104320577",
                "managed": false,
                "mentionable": false,
                "flags": 0
            }
        ],
        "emojis": [],
        "features": ["COMMUNITY"],
        "mfa_level": 0,
        "system_channel_id": "197038439483310081",
        "system_channel_flags": 0,
        "vanity_url_code": null,
        "description": "A great guild",
        "banner": null,
        "premium_tier": 0,
        "preferred_locale": "en-US",
        "joined_at": "2024-01-01T00:00:00.000000+00:00",
        "large": false,
        "unavailable": false,
        "member_count": 42,
        "channels": [
            {
                "id": "197038439483310081",
                "type": 0,
                "guild_id": "197038439483310080",
                "name": "general",
                "position": 0,
                "flags": 0
            }
        ],
        "threads": [],
        "members": [],
        "presences": []
    }"#;

    let event = GatewayEvent::parse_str("GUILD_CREATE", payload).expect("parse GUILD_CREATE");
    if let GatewayEvent::GuildCreate(guild) = event {
        assert_eq!(guild.id, Snowflake::new(197038439483310080));
        assert_eq!(guild.name, "Rift Community");
        assert_eq!(guild.owner_id, Some(Snowflake::new(80351110224678912)));
        assert_eq!(guild.member_count, Some(42));
        assert_eq!(guild.channels.as_ref().map(|c| c.len()), Some(1));
        assert_eq!(guild.roles.as_ref().map(|r| r.len()), Some(1));
        let role = &guild.roles.unwrap()[0];
        assert_eq!(role.permissions, Permissions::from_bits_retain(104320577));
    } else {
        panic!("expected GuildCreate variant");
    }
}

#[test]
fn test_channel_create_normal_and_obfuscated() {
    // Normal channel
    let normal_payload = r#"{
        "id": "41771983423143937",
        "guild_id": "197038439483310080",
        "name": "general",
        "type": 0,
        "position": 0,
        "permission_overwrites": [
            {
                "id": "197038439483310080",
                "type": 0,
                "allow": "1024",
                "deny": "0"
            }
        ],
        "rate_limit_per_user": 0,
        "nsfw": false,
        "topic": "Welcome to general!",
        "last_message_id": "155117677105512449",
        "parent_id": "399942396007890945",
        "flags": 0
    }"#;

    let event = GatewayEvent::parse_str("CHANNEL_CREATE", normal_payload).expect("parse normal channel");
    if let GatewayEvent::ChannelCreate(channel) = event {
        assert_eq!(channel.id, Snowflake::new(41771983423143937));
        assert_eq!(channel.name.as_deref(), Some("general"));
        assert_eq!(channel.topic.as_deref(), Some("Welcome to general!"));
        assert!(!channel.is_obfuscated());
        assert_eq!(channel.channel_flags(), ChannelFlags::NONE);
    } else {
        panic!("expected ChannelCreate variant");
    }

    // Obfuscated channel (CHANNEL_OBFUSCATED = 1 << 17 = 131072)
    // Metadata like topic, bitrate, permissions, last_message_id are stripped.
    let obfuscated_payload = r#"{
        "id": "51771983423143938",
        "guild_id": "197038439483310080",
        "name": "___hidden___",
        "type": 0,
        "position": 5,
        "parent_id": "399942396007890945",
        "flags": 131072
    }"#;

    let event = GatewayEvent::parse_str("CHANNEL_CREATE", obfuscated_payload).expect("parse obfuscated channel");
    if let GatewayEvent::ChannelCreate(channel) = event {
        assert_eq!(channel.id, Snowflake::new(51771983423143938));
        assert_eq!(channel.name.as_deref(), Some("___hidden___"));
        assert!(channel.topic.is_none());
        assert!(channel.is_obfuscated());
        assert!(channel.channel_flags().contains(ChannelFlags::CHANNEL_OBFUSCATED));
    } else {
        panic!("expected ChannelCreate variant");
    }
}

#[test]
fn test_guild_member_add_event() {
    let payload = r#"{
        "guild_id": "197038439483310080",
        "roles": ["197038439483310080", "197038439483310099"],
        "user": {
            "id": "80351110224678912",
            "username": "bimo",
            "discriminator": "0001",
            "avatar": "834272909634892834"
        },
        "nick": "Bimo",
        "joined_at": "2024-01-01T12:00:00.000000+00:00",
        "deaf": false,
        "mute": false
    }"#;

    let event = GatewayEvent::parse_str("GUILD_MEMBER_ADD", payload).expect("parse GUILD_MEMBER_ADD");
    if let GatewayEvent::GuildMemberAdd(add) = event {
        assert_eq!(add.guild_id, Snowflake::new(197038439483310080));
        assert_eq!(add.member.roles.len(), 2);
        assert_eq!(add.member.roles[0], Snowflake::new(197038439483310080));
        assert_eq!(add.member.roles[1], Snowflake::new(197038439483310099));
        assert_eq!(add.member.nick.as_deref(), Some("Bimo"));
        assert_eq!(
            add.member.user.as_ref().map(|u| u.id),
            Some(Snowflake::new(80351110224678912))
        );
    } else {
        panic!("expected GuildMemberAdd variant");
    }
}

#[test]
fn test_guild_role_create_event() {
    let payload = r#"{
        "guild_id": "197038439483310080",
        "role": {
            "id": "417719834423143936",
            "name": "Moderator",
            "color": 3447003,
            "hoist": true,
            "position": 1,
            "permissions": "268435456",
            "managed": false,
            "mentionable": true,
            "flags": 0
        }
    }"#;

    let event = GatewayEvent::parse_str("GUILD_ROLE_CREATE", payload).expect("parse GUILD_ROLE_CREATE");
    if let GatewayEvent::GuildRoleCreate(role_create) = event {
        assert_eq!(role_create.guild_id, Snowflake::new(197038439483310080));
        assert_eq!(role_create.role.id, Snowflake::new(417719834423143936));
        assert_eq!(role_create.role.name, "Moderator");
        assert_eq!(role_create.role.permissions, Permissions::from_bits_retain(268435456));
        assert!(role_create.role.hoist);
    } else {
        panic!("expected GuildRoleCreate variant");
    }
}

#[test]
fn test_message_create_event() {
    let payload = r#"{
        "id": "155117677105512449",
        "channel_id": "41771983423143937",
        "guild_id": "197038439483310080",
        "author": {
            "id": "80351110224678912",
            "username": "bimo",
            "discriminator": "0001"
        },
        "content": "Hello, Discord Gateway!",
        "timestamp": "2024-01-01T12:05:00.000000+00:00",
        "edited_timestamp": null,
        "tts": false,
        "mention_everyone": false,
        "mentions": [],
        "mention_roles": [],
        "attachments": [],
        "embeds": [],
        "pinned": false,
        "type": 0
    }"#;

    let event = GatewayEvent::parse_str("MESSAGE_CREATE", payload).expect("parse MESSAGE_CREATE");
    if let GatewayEvent::MessageCreate(msg) = event {
        assert_eq!(msg.id, Snowflake::new(155117677105512449));
        assert_eq!(msg.channel_id, Snowflake::new(41771983423143937));
        assert_eq!(msg.guild_id, Some(Snowflake::new(197038439483310080)));
        assert_eq!(msg.content, "Hello, Discord Gateway!");
        assert_eq!(
            msg.author.as_ref().map(|a| a.id),
            Some(Snowflake::new(80351110224678912))
        );
    } else {
        panic!("expected MessageCreate variant");
    }
}

#[test]
fn test_message_reaction_add_event() {
    let payload = r#"{
        "user_id": "80351110224678912",
        "channel_id": "41771983423143937",
        "message_id": "155117677105512449",
        "guild_id": "197038439483310080",
        "emoji": {
            "id": null,
            "name": "👍"
        },
        "burst": false
    }"#;

    let event = GatewayEvent::parse_str("MESSAGE_REACTION_ADD", payload).expect("parse MESSAGE_REACTION_ADD");
    if let GatewayEvent::MessageReactionAdd(reaction) = event {
        assert_eq!(reaction.user_id, Snowflake::new(80351110224678912));
        assert_eq!(reaction.channel_id, Snowflake::new(41771983423143937));
        assert_eq!(reaction.message_id, Snowflake::new(155117677105512449));
        assert_eq!(reaction.guild_id, Some(Snowflake::new(197038439483310080)));
        assert_eq!(reaction.emoji.name.as_deref(), Some("👍"));
        assert_eq!(reaction.burst, Some(false));
    } else {
        panic!("expected MessageReactionAdd variant");
    }
}

#[test]
fn test_interaction_create_event() {
    let payload = r#"{
        "id": "1088605110638227538",
        "application_id": "1088605110638227537",
        "type": 2,
        "data": {
            "id": "1088605110638227539",
            "name": "ping",
            "type": 1
        },
        "guild_id": "197038439483310080",
        "channel_id": "41771983423143937",
        "member": {
            "user": {
                "id": "80351110224678912",
                "username": "bimo",
                "discriminator": "0001"
            },
            "roles": [],
            "joined_at": "2024-01-01T12:00:00.000000+00:00",
            "permissions": "104320577"
        },
        "token": "interaction_token_abc123",
        "version": 1,
        "app_permissions": "104320577",
        "locale": "en-US"
    }"#;

    let event = GatewayEvent::parse_str("INTERACTION_CREATE", payload).expect("parse INTERACTION_CREATE");
    if let GatewayEvent::InteractionCreate(interaction) = event {
        assert_eq!(interaction.id, Snowflake::new(1088605110638227538));
        assert_eq!(interaction.application_id, Snowflake::new(1088605110638227537));
        assert_eq!(interaction.kind, InteractionType::ApplicationCommand);
        assert_eq!(interaction.token, "interaction_token_abc123");
        assert_eq!(
            interaction.invoking_user().map(|u| u.id),
            Some(Snowflake::new(80351110224678912))
        );
        assert_eq!(
            interaction.member_permissions(),
            Some(Permissions::from_bits_retain(104320577))
        );
        assert_eq!(
            interaction.app_permissions(),
            Some(Permissions::from_bits_retain(104320577))
        );
        assert!(interaction.data.is_some());
    } else {
        panic!("expected InteractionCreate variant");
    }
}

#[test]
fn test_voice_state_update_event() {
    let payload = r#"{
        "guild_id": "197038439483310080",
        "channel_id": "197038439483310082",
        "user_id": "80351110224678912",
        "session_id": "4f9b88a9c39482f",
        "deaf": false,
        "mute": false,
        "self_deaf": false,
        "self_mute": false,
        "self_video": false,
        "suppress": false,
        "request_to_speak_timestamp": null
    }"#;

    let event = GatewayEvent::parse_str("VOICE_STATE_UPDATE", payload).expect("parse VOICE_STATE_UPDATE");
    if let GatewayEvent::VoiceStateUpdate(voice) = event {
        assert_eq!(voice.guild_id, Some(Snowflake::new(197038439483310080)));
        assert_eq!(voice.channel_id, Some(Snowflake::new(197038439483310082)));
        assert_eq!(voice.user_id, Snowflake::new(80351110224678912));
        assert_eq!(voice.session_id, "4f9b88a9c39482f");
        assert!(!voice.self_mute);
    } else {
        panic!("expected VoiceStateUpdate variant");
    }
}

#[test]
fn test_typing_start_event() {
    let payload = r#"{
        "channel_id": "41771983423143937",
        "guild_id": "197038439483310080",
        "user_id": "80351110224678912",
        "timestamp": 1704110700
    }"#;

    let event = GatewayEvent::parse_str("TYPING_START", payload).expect("parse TYPING_START");
    if let GatewayEvent::TypingStart(typing) = event {
        assert_eq!(typing.channel_id, Snowflake::new(41771983423143937));
        assert_eq!(typing.guild_id, Some(Snowflake::new(197038439483310080)));
        assert_eq!(typing.user_id, Snowflake::new(80351110224678912));
        assert_eq!(typing.timestamp, 1704110700);
    } else {
        panic!("expected TypingStart variant");
    }
}

#[test]
fn test_unknown_event_never_fails() {
    let unknown_json = r#"{"custom_key": "some_value", "number": 12345}"#;
    let raw = RawValue::from_string(unknown_json.to_string()).unwrap();

    let event = GatewayEvent::parse("SOME_BRAND_NEW_DISCORD_EVENT", &raw)
        .expect("unknown event must never fail to parse");

    if let GatewayEvent::Unknown { name, data } = event {
        assert_eq!(name, "SOME_BRAND_NEW_DISCORD_EVENT");
        assert_eq!(data.get(), unknown_json);
    } else {
        panic!("expected Unknown variant");
    }
}

#[test]
fn test_extra_and_unknown_fields_tolerated() {
    // Payloads with new unannounced fields from Discord must not fail.
    let payload = r#"{
        "id": "155117677105512449",
        "channel_id": "41771983423143937",
        "content": "Message with future fields",
        "timestamp": "2024-01-01T12:05:00.000000+00:00",
        "tts": false,
        "mention_everyone": false,
        "pinned": false,
        "type": 0,
        "future_discord_ai_summary": "Summary text",
        "unannounced_experiment_flags": [1, 2, 3],
        "nested_extra_data": {
            "key": "value"
        }
    }"#;

    let event = GatewayEvent::parse_str("MESSAGE_CREATE", payload)
        .expect("payload with extra fields must be tolerated cleanly");

    if let GatewayEvent::MessageCreate(msg) = event {
        assert_eq!(msg.id, Snowflake::new(155117677105512449));
        assert_eq!(msg.content, "Message with future fields");
    } else {
        panic!("expected MessageCreate variant");
    }
}
