use rift_cache::{Cache, CacheConfig};
use rift_core::{Permissions, Snowflake};
use rift_model::channel::{Channel, ChannelOverwrite, ChannelType};
use rift_model::events::{
    GuildDelete, GuildMemberAdd, GuildMemberRemove, GuildMemberUpdate, GuildMembersChunk,
    GuildRoleCreate, GuildRoleDelete, GuildRoleUpdate, MessageDelete, MessageDeleteBulk, Ready,
    ReadyApplication, ThreadDelete, ThreadListSync,
};
use rift_model::flags::ChannelFlags;
use rift_model::guild::{Guild, GuildMember, Role, UnavailableGuild};
use rift_model::message::Message;
use rift_model::presence::{ClientStatus, PresenceUpdate};
use rift_model::user::{PresenceUser, User};
use rift_model::voice::VoiceState;
use rift_model::GatewayEvent;

fn sf(id: u64) -> Snowflake {
    Snowflake::new(id)
}

fn make_user(id: u64, username: &str) -> User {
    User {
        id: sf(id),
        username: username.to_string(),
        discriminator: Some("0001".to_string()),
        global_name: Some(username.to_string()),
        avatar: None,
        bot: Some(false),
        system: None,
        mfa_enabled: None,
        banner: None,
        accent_color: None,
        locale: None,
        verified: None,
        email: None,
        flags: None,
        premium_type: None,
        public_flags: None,
        avatar_decoration_data: None,
    }
}

fn make_guild(id: u64, owner_id: u64, name: &str) -> Guild {
    Guild {
        id: sf(id),
        name: name.to_string(),
        icon: None,
        icon_hash: None,
        splash: None,
        discovery_splash: None,
        owner: None,
        owner_id: Some(sf(owner_id)),
        permissions: None,
        region: None,
        afk_channel_id: None,
        afk_timeout: None,
        widget_enabled: None,
        widget_channel_id: None,
        verification_level: None,
        default_message_notifications: None,
        explicit_content_filter: None,
        roles: Some(Vec::new()),
        emojis: None,
        features: None,
        mfa_level: None,
        application_id: None,
        system_channel_id: None,
        system_channel_flags: None,
        rules_channel_id: None,
        max_presences: None,
        max_members: None,
        vanity_url_code: None,
        description: None,
        banner: None,
        premium_tier: None,
        premium_subscription_count: None,
        preferred_locale: None,
        public_updates_channel_id: None,
        max_video_channel_users: None,
        max_stage_video_channel_users: None,
        approximate_member_count: None,
        approximate_presence_count: None,
        stickers: None,
        premium_progress_bar_enabled: None,
        safety_alerts_channel_id: None,
        joined_at: None,
        large: None,
        unavailable: None,
        member_count: None,
        voice_states: Some(Vec::new()),
        members: Some(Vec::new()),
        channels: Some(Vec::new()),
        threads: Some(Vec::new()),
        presences: Some(Vec::new()),
        stage_instances: None,
        guild_scheduled_events: None,
    }
}

fn make_channel(id: u64, guild_id: u64, name: &str, obfuscated: bool) -> Channel {
    let flags = if obfuscated {
        Some(ChannelFlags::CHANNEL_OBFUSCATED.bits())
    } else {
        Some(0)
    };

    Channel {
        id: sf(id),
        kind: ChannelType::GuildText,
        guild_id: Some(sf(guild_id)),
        position: Some(0),
        permission_overwrites: Some(Vec::new()),
        name: Some(name.to_string()),
        topic: Some("Test topic".to_string()),
        nsfw: Some(false),
        last_message_id: None,
        bitrate: None,
        user_limit: None,
        rate_limit_per_user: None,
        recipients: None,
        icon: None,
        owner_id: None,
        application_id: None,
        managed: None,
        parent_id: None,
        last_pin_timestamp: None,
        rtc_region: None,
        video_quality_mode: None,
        message_count: None,
        member_count: None,
        thread_metadata: None,
        member: None,
        default_auto_archive_duration: None,
        permissions: None,
        flags,
        total_message_sent: None,
        available_tags: None,
        applied_tags: None,
        default_reaction_emoji: None,
        default_thread_rate_limit_per_user: None,
        default_sort_order: None,
        default_forum_layout: None,
        newly_created: None,
    }
}

fn make_role(id: u64, name: &str, permissions: Permissions) -> Role {
    Role {
        id: sf(id),
        name: name.to_string(),
        color: 0,
        hoist: false,
        icon: None,
        unicode_emoji: None,
        position: 1,
        permissions,
        managed: false,
        mentionable: false,
        tags: None,
        flags: 0,
    }
}

fn make_member(user_id: u64, roles: Vec<u64>, timeout: Option<&str>) -> GuildMember {
    GuildMember {
        user: Some(make_user(user_id, &format!("user_{user_id}"))),
        nick: None,
        avatar: None,
        roles: roles.into_iter().map(sf).collect(),
        joined_at: "2026-01-01T00:00:00Z".to_string(),
        premium_since: None,
        deaf: Some(false),
        mute: Some(false),
        flags: None,
        pending: Some(false),
        permissions: None,
        communication_disabled_until: timeout.map(ToString::to_string),
        avatar_decoration_data: None,
    }
}

fn make_message(id: u64, channel_id: u64, author_id: u64, content: &str) -> Message {
    Message {
        id: sf(id),
        channel_id: sf(channel_id),
        author: Some(make_user(author_id, &format!("author_{author_id}"))),
        content: content.to_string(),
        timestamp: "2026-10-31T12:00:00Z".to_string(),
        edited_timestamp: None,
        tts: false,
        mention_everyone: false,
        mentions: Some(Vec::new()),
        mention_roles: Some(Vec::new()),
        mention_channels: None,
        attachments: Some(Vec::new()),
        embeds: Some(Vec::new()),
        reactions: None,
        nonce: None,
        pinned: false,
        webhook_id: None,
        kind: 0,
        activity: None,
        application: None,
        application_id: None,
        message_reference: None,
        flags: None,
        referenced_message: None,
        interaction: None,
        thread: None,
        components: None,
        sticker_items: None,
        position: None,
        role_subscription_data: None,
        poll: None,
        interaction_metadata: None,
        guild_id: None,
        member: None,
    }
}

#[test]
fn test_config_builders() {
    let cfg = CacheConfig::default();
    assert!(cfg.cache_guilds);
    assert!(cfg.cache_channels);
    assert!(cfg.cache_members);
    assert!(cfg.cache_roles);
    assert!(cfg.cache_users);
    assert!(cfg.cache_presences);
    assert!(cfg.cache_voice_states);
    assert_eq!(cfg.max_messages_per_channel, 100);
    assert_eq!(cfg.max_members_per_guild, None);
    assert_eq!(cfg.max_users, None);

    let none = CacheConfig::none();
    assert!(!none.cache_guilds);
    assert_eq!(none.max_messages_per_channel, 0);

    let custom = CacheConfig::new()
        .cache_guilds(true)
        .cache_channels(false)
        .max_messages_per_channel(50)
        .max_members_per_guild(Some(500))
        .max_users(Some(1000));

    assert!(custom.cache_guilds);
    assert!(!custom.cache_channels);
    assert_eq!(custom.max_messages_per_channel, 50);
    assert_eq!(custom.max_members_per_guild, Some(500));
    assert_eq!(custom.max_users, Some(1000));
}

#[test]
fn test_ready_event() {
    let mut cache = Cache::default();
    let ready = Ready {
        v: 10,
        user: make_user(1, "BotUser"),
        guilds: vec![
            UnavailableGuild {
                id: sf(100),
                unavailable: Some(true),
            },
            UnavailableGuild {
                id: sf(101),
                unavailable: Some(true),
            },
        ],
        session_id: "sess_123".to_string(),
        resume_gateway_url: "wss://gateway.discord.gg".to_string(),
        shard: Some([0, 1]),
        application: ReadyApplication {
            id: sf(1),
            flags: None,
        },
    };

    cache.update(&GatewayEvent::Ready(ready));

    assert_eq!(cache.current_user().map(|u| u.id), Some(sf(1)));
    assert_eq!(cache.user(sf(1)).map(|u| u.username.as_str()), Some("BotUser"));
    assert_eq!(cache.stats().guilds, 2);
    assert_eq!(cache.guild(sf(100)).map(|g| g.id), Some(sf(100)));
}

#[test]
fn test_guild_lifecycle() {
    let mut cache = Cache::default();
    let mut guild = make_guild(100, 42, "Rust Guild");
    guild.roles = Some(vec![
        make_role(100, "@everyone", Permissions::VIEW_CHANNEL),
        make_role(201, "Moderator", Permissions::BAN_MEMBERS),
    ]);
    guild.channels = Some(vec![
        make_channel(301, 100, "general", false),
        make_channel(302, 100, "announcements", false),
    ]);
    guild.members = Some(vec![
        make_member(42, vec![], None),
        make_member(43, vec![201], None),
    ]);

    // Guild Create
    cache.update(&GatewayEvent::GuildCreate(guild.clone()));

    assert_eq!(cache.stats().guilds, 1);
    assert_eq!(cache.stats().roles, 2);
    assert_eq!(cache.stats().visible_channels, 2);
    assert_eq!(cache.stats().members, 2);

    // Guild Update
    let mut updated_guild = guild.clone();
    updated_guild.name = "Rust Super Guild".to_string();
    cache.update(&GatewayEvent::GuildUpdate(updated_guild));
    assert_eq!(cache.guild(sf(100)).unwrap().name, "Rust Super Guild");

    // Guild Delete (temporary outage)
    let outage = GuildDelete {
        id: sf(100),
        unavailable: Some(true),
    };
    cache.update(&GatewayEvent::GuildDelete(outage));
    assert_eq!(cache.guild(sf(100)).unwrap().unavailable, Some(true));
    assert_eq!(cache.stats().visible_channels, 2); // channels kept during outage

    // Guild Delete (bot kicked or guild deleted)
    let kicked = GuildDelete {
        id: sf(100),
        unavailable: Some(false),
    };
    cache.update(&GatewayEvent::GuildDelete(kicked));
    assert_eq!(cache.stats().guilds, 0);
    assert_eq!(cache.stats().roles, 0);
    assert_eq!(cache.stats().visible_channels, 0);
    assert_eq!(cache.stats().members, 0);
}

#[test]
fn test_channel_lifecycle() {
    let mut cache = Cache::default();
    let ch = make_channel(301, 100, "general", false);

    // Channel Create
    cache.update(&GatewayEvent::ChannelCreate(ch.clone()));
    assert_eq!(cache.channel(sf(301)).unwrap().name.as_deref(), Some("general"));
    assert_eq!(cache.stats().visible_channels, 1);

    // Channel Update
    let mut ch_updated = ch.clone();
    ch_updated.name = Some("general-chat".to_string());
    cache.update(&GatewayEvent::ChannelUpdate(ch_updated));
    assert_eq!(cache.channel(sf(301)).unwrap().name.as_deref(), Some("general-chat"));

    // Add a message
    let msg = make_message(901, 301, 42, "hello world");
    cache.update(&GatewayEvent::MessageCreate(msg));
    assert_eq!(cache.stats().messages, 1);

    // Channel Delete (should clear channel and its cached messages)
    cache.update(&GatewayEvent::ChannelDelete(ch));
    assert_eq!(cache.channel(sf(301)), None);
    assert_eq!(cache.stats().visible_channels, 0);
    assert_eq!(cache.stats().messages, 0);
}

#[test]
fn test_nov16_channel_obfuscation_safety() {
    let mut cache = Cache::default();

    // Channel received with placeholder name "___hidden___" and obfuscated flag
    let mut obf_ch = make_channel(401, 100, "___hidden___", true);
    obf_ch.topic = Some("Secret Obfuscated Topic".to_string());
    obf_ch.permission_overwrites = Some(vec![ChannelOverwrite {
        id: sf(100),
        kind: 0,
        allow: Permissions::ALL,
        deny: Permissions::NONE,
    }]);

    cache.update(&GatewayEvent::ChannelCreate(obf_ch));

    // Nov 16 Compliance assertions:
    // 1. is_channel_obfuscated reports true
    assert!(cache.is_channel_obfuscated(sf(401)));

    // 2. visible_channel returns None
    assert_eq!(cache.visible_channel(sf(401)), None);

    // 3. channel accessor returns None to prevent leakage
    assert_eq!(cache.channel(sf(401)), None);

    // 4. raw_channel exists but has sanitized name/topic (placeholder stripped)
    let raw = cache.raw_channel(sf(401)).expect("raw channel exists");
    assert_eq!(raw.name, None);
    assert_eq!(raw.topic, None);
    assert_eq!(raw.permission_overwrites, None);

    // 5. stats count correctly distinguishes visible vs obfuscated
    assert_eq!(cache.stats().visible_channels, 0);
    assert_eq!(cache.stats().obfuscated_channels, 1);
    assert_eq!(cache.stats().total_channels(), 1);

    // Also verify visible channel works side-by-side
    let normal_ch = make_channel(402, 100, "visible-room", false);
    cache.update(&GatewayEvent::ChannelCreate(normal_ch));

    assert!(!cache.is_channel_obfuscated(sf(402)));
    assert_eq!(cache.visible_channel(sf(402)).unwrap().name.as_deref(), Some("visible-room"));
    assert_eq!(cache.channel(sf(402)).unwrap().name.as_deref(), Some("visible-room"));

    assert_eq!(cache.stats().visible_channels, 1);
    assert_eq!(cache.stats().obfuscated_channels, 1);
    assert_eq!(cache.stats().total_channels(), 2);
}

#[test]
fn test_channel_un_obfuscation() {
    let mut cache = Cache::default();
    let obf = make_channel(401, 100, "___hidden___", true);
    cache.update(&GatewayEvent::ChannelCreate(obf));

    assert!(cache.is_channel_obfuscated(sf(401)));
    assert_eq!(cache.visible_channel(sf(401)), None);

    // Channel becomes un-obfuscated via ChannelUpdate
    let un_obf = make_channel(401, 100, "unlocked-channel", false);
    cache.update(&GatewayEvent::ChannelUpdate(un_obf));

    assert!(!cache.is_channel_obfuscated(sf(401)));
    assert_eq!(cache.visible_channel(sf(401)).unwrap().name.as_deref(), Some("unlocked-channel"));
    assert_eq!(cache.stats().visible_channels, 1);
    assert_eq!(cache.stats().obfuscated_channels, 0);
}

#[test]
fn test_thread_lifecycle_and_list_sync() {
    let mut cache = Cache::default();
    let mut thread = make_channel(501, 100, "cool-thread", false);
    thread.kind = ChannelType::PublicThread;
    thread.parent_id = Some(sf(301));

    cache.update(&GatewayEvent::ThreadCreate(thread.clone()));
    assert_eq!(cache.channel(sf(501)).unwrap().name.as_deref(), Some("cool-thread"));

    let mut thread_up = thread.clone();
    thread_up.name = Some("renamed-thread".to_string());
    cache.update(&GatewayEvent::ThreadUpdate(thread_up));
    assert_eq!(cache.channel(sf(501)).unwrap().name.as_deref(), Some("renamed-thread"));

    // Thread list sync
    let mut synced_thread = make_channel(502, 100, "synced-thread", false);
    synced_thread.kind = ChannelType::PublicThread;
    let sync = ThreadListSync {
        guild_id: sf(100),
        channel_ids: None,
        threads: vec![synced_thread],
        members: vec![],
    };
    cache.update(&GatewayEvent::ThreadListSync(sync));
    assert_eq!(cache.channel(sf(502)).unwrap().name.as_deref(), Some("synced-thread"));

    // Thread delete
    let tdel = ThreadDelete {
        id: sf(501),
        guild_id: sf(100),
        parent_id: Some(sf(301)),
        kind: ChannelType::PublicThread.into(),
    };
    cache.update(&GatewayEvent::ThreadDelete(tdel));
    assert_eq!(cache.channel(sf(501)), None);
}

#[test]
fn test_member_lifecycle() {
    let mut cache = Cache::default();
    let gid = sf(100);

    // Member Add
    let m1 = make_member(10, vec![201], None);
    cache.update(&GatewayEvent::GuildMemberAdd(GuildMemberAdd {
        guild_id: gid,
        member: m1,
    }));
    assert_eq!(cache.member(gid, sf(10)).unwrap().roles, vec![sf(201)]);

    // Member Update (roles and nickname)
    let m1_update = GuildMemberUpdate {
        guild_id: gid,
        roles: vec![sf(201), sf(202)],
        user: make_user(10, "user_10"),
        nick: Some("Captain".to_string()),
        avatar: None,
        joined_at: None,
        premium_since: None,
        deaf: Some(false),
        mute: Some(false),
        pending: Some(false),
        communication_disabled_until: None,
        flags: None,
        avatar_decoration_data: None,
    };
    cache.update(&GatewayEvent::GuildMemberUpdate(m1_update));
    let cached_m1 = cache.member(gid, sf(10)).unwrap();
    assert_eq!(cached_m1.roles, vec![sf(201), sf(202)]);
    assert_eq!(cached_m1.nick.as_deref(), Some("Captain"));

    // Guild Members Chunk
    let m2 = make_member(11, vec![], None);
    let m3 = make_member(12, vec![], None);
    let chunk = GuildMembersChunk {
        guild_id: gid,
        members: vec![m2, m3],
        chunk_index: 0,
        chunk_count: 1,
        not_found: None,
        presences: None,
        nonce: None,
    };
    cache.update(&GatewayEvent::GuildMembersChunk(chunk));
    assert_eq!(cache.stats().members, 3);

    // Member Remove
    cache.update(&GatewayEvent::GuildMemberRemove(GuildMemberRemove {
        guild_id: gid,
        user: make_user(11, "user_11"),
    }));
    assert_eq!(cache.member(gid, sf(11)), None);
    assert_eq!(cache.stats().members, 2);
}

#[test]
fn test_member_fifo_eviction() {
    let config = CacheConfig::default().max_members_per_guild(Some(2));
    let mut cache = Cache::new(config);
    let gid = sf(100);

    cache.update(&GatewayEvent::GuildMemberAdd(GuildMemberAdd {
        guild_id: gid,
        member: make_member(1, vec![], None),
    }));
    cache.update(&GatewayEvent::GuildMemberAdd(GuildMemberAdd {
        guild_id: gid,
        member: make_member(2, vec![], None),
    }));
    assert_eq!(cache.stats().members, 2);
    assert!(cache.member(gid, sf(1)).is_some());
    assert!(cache.member(gid, sf(2)).is_some());

    // Adding 3rd member evicts oldest (1)
    cache.update(&GatewayEvent::GuildMemberAdd(GuildMemberAdd {
        guild_id: gid,
        member: make_member(3, vec![], None),
    }));
    assert_eq!(cache.stats().members, 2);
    assert_eq!(cache.member(gid, sf(1)), None);
    assert!(cache.member(gid, sf(2)).is_some());
    assert!(cache.member(gid, sf(3)).is_some());
}

#[test]
fn test_user_fifo_eviction() {
    let config = CacheConfig::default().max_users(Some(2));
    let mut cache = Cache::new(config);

    cache.update(&GatewayEvent::UserUpdate(make_user(1, "User1")));
    cache.update(&GatewayEvent::UserUpdate(make_user(2, "User2")));
    assert_eq!(cache.stats().users, 2);
    assert!(cache.user(sf(1)).is_some());
    assert!(cache.user(sf(2)).is_some());

    // 3rd user evicts user 1
    cache.update(&GatewayEvent::UserUpdate(make_user(3, "User3")));
    assert_eq!(cache.stats().users, 2);
    assert_eq!(cache.user(sf(1)), None);
    assert!(cache.user(sf(2)).is_some());
    assert!(cache.user(sf(3)).is_some());
}

#[test]
fn test_role_lifecycle() {
    let mut cache = Cache::default();
    let gid = sf(100);

    // Role Create
    let r1 = make_role(201, "Admin", Permissions::ADMINISTRATOR);
    cache.update(&GatewayEvent::GuildRoleCreate(GuildRoleCreate {
        guild_id: gid,
        role: r1,
    }));
    assert_eq!(cache.role(gid, sf(201)).unwrap().permissions, Permissions::ADMINISTRATOR);

    // Role Update
    let mut r1_mod = make_role(201, "Mod", Permissions::MANAGE_CHANNELS);
    r1_mod.name = "SuperMod".to_string();
    cache.update(&GatewayEvent::GuildRoleUpdate(GuildRoleUpdate {
        guild_id: gid,
        role: r1_mod,
    }));
    assert_eq!(cache.role(gid, sf(201)).unwrap().name, "SuperMod");
    assert_eq!(cache.role(gid, sf(201)).unwrap().permissions, Permissions::MANAGE_CHANNELS);

    // Role Delete
    cache.update(&GatewayEvent::GuildRoleDelete(GuildRoleDelete {
        guild_id: gid,
        role_id: sf(201),
    }));
    assert_eq!(cache.role(gid, sf(201)), None);
}

#[test]
fn test_message_ring_buffer_fifo_eviction() {
    let config = CacheConfig::default().max_messages_per_channel(3);
    let mut cache = Cache::new(config);
    let cid = sf(301);

    for i in 1..=4 {
        let msg = make_message(i, 301, 42, &format!("msg_{i}"));
        cache.update(&GatewayEvent::MessageCreate(msg));
    }

    // Ring buffer size should cap at 3, with msg 1 evicted
    let msgs = cache.channel_messages(cid).expect("channel messages exist");
    assert_eq!(msgs.len(), 3);
    assert_eq!(msgs.front().unwrap().id, sf(2));
    assert_eq!(msgs.back().unwrap().id, sf(4));
    assert_eq!(cache.message(cid, sf(1)), None);
    assert_eq!(cache.message(cid, sf(2)).map(|m| m.content.as_str()), Some("msg_2"));
}

#[test]
fn test_message_update_and_deletion() {
    let mut cache = Cache::default();
    let cid = sf(301);

    let m1 = make_message(101, 301, 42, "original");
    let m2 = make_message(102, 301, 42, "msg2");
    let m3 = make_message(103, 301, 42, "msg3");

    cache.update(&GatewayEvent::MessageCreate(m1));
    cache.update(&GatewayEvent::MessageCreate(m2));
    cache.update(&GatewayEvent::MessageCreate(m3));

    // Message Update
    let mut m1_edit = make_message(101, 301, 42, "edited content");
    m1_edit.edited_timestamp = Some("2026-10-31T12:05:00Z".to_string());
    cache.update(&GatewayEvent::MessageUpdate(m1_edit));
    assert_eq!(cache.message(cid, sf(101)).unwrap().content, "edited content");

    // Single Delete
    cache.update(&GatewayEvent::MessageDelete(MessageDelete {
        id: sf(102),
        channel_id: cid,
        guild_id: None,
    }));
    assert_eq!(cache.message(cid, sf(102)), None);
    assert_eq!(cache.channel_messages(cid).unwrap().len(), 2);

    // Bulk Delete
    cache.update(&GatewayEvent::MessageDeleteBulk(MessageDeleteBulk {
        ids: vec![sf(101), sf(103)],
        channel_id: cid,
        guild_id: None,
    }));
    assert_eq!(cache.channel_messages(cid).unwrap().len(), 0);
}

#[test]
fn test_voice_state_tracking() {
    let mut cache = Cache::default();
    let gid = sf(100);
    let uid = sf(42);

    // User joins voice channel
    let vs = VoiceState {
        guild_id: Some(gid),
        channel_id: Some(sf(501)),
        user_id: uid,
        member: None,
        session_id: "sess_voice".to_string(),
        deaf: false,
        mute: false,
        self_deaf: false,
        self_mute: false,
        self_stream: None,
        self_video: false,
        suppress: false,
        request_to_speak_timestamp: None,
    };
    cache.update(&GatewayEvent::VoiceStateUpdate(vs));
    assert_eq!(cache.voice_state(gid, uid).unwrap().channel_id, Some(sf(501)));

    // User moves to another voice channel
    let mut vs_moved = cache.voice_state(gid, uid).unwrap().clone();
    vs_moved.channel_id = Some(sf(502));
    cache.update(&GatewayEvent::VoiceStateUpdate(vs_moved));
    assert_eq!(cache.voice_state(gid, uid).unwrap().channel_id, Some(sf(502)));

    // User disconnects (channel_id: None)
    let mut vs_dc = cache.voice_state(gid, uid).unwrap().clone();
    vs_dc.channel_id = None;
    cache.update(&GatewayEvent::VoiceStateUpdate(vs_dc));
    assert_eq!(cache.voice_state(gid, uid), None);
}

#[test]
fn test_presence_tracking() {
    let mut cache = Cache::default();
    let gid = sf(100);
    let uid = sf(42);

    // Add user first
    cache.update(&GatewayEvent::UserUpdate(make_user(42, "OldName")));

    let pres = PresenceUpdate {
        user: PresenceUser {
            id: uid,
            username: Some("NewName".to_string()),
            discriminator: None,
            avatar: None,
            global_name: Some("New Global Name".to_string()),
        },
        guild_id: Some(gid),
        status: "online".to_string(),
        activities: None,
        client_status: Some(ClientStatus {
            desktop: Some("online".to_string()),
            mobile: None,
            web: None,
        }),
    };

    cache.update(&GatewayEvent::PresenceUpdate(pres));
    assert_eq!(cache.presence(gid, uid).unwrap().status, "online");
    assert_eq!(cache.user(uid).unwrap().username, "NewName");
    assert_eq!(cache.user(uid).unwrap().global_name.as_deref(), Some("New Global Name"));
}

#[test]
fn test_effective_permissions_owner_bypass() {
    let mut cache = Cache::default();
    let gid = sf(100);
    let owner_id = sf(42);
    let cid = sf(301);

    let guild = make_guild(100, 42, "Guild");
    let mut channel = make_channel(301, 100, "locked-channel", false);
    // Deny all for everyone
    channel.permission_overwrites = Some(vec![ChannelOverwrite {
        id: gid,
        kind: 0,
        allow: Permissions::NONE,
        deny: Permissions::ALL,
    }]);

    cache.update(&GatewayEvent::GuildCreate(guild));
    cache.update(&GatewayEvent::ChannelCreate(channel));

    let perms = cache
        .member_permissions_in_channel(gid, cid, owner_id)
        .expect("permissions computed");

    // Owner gets Permissions::ALL despite overwrites
    assert_eq!(perms, Permissions::ALL);
}

#[test]
fn test_effective_permissions_admin_bypass() {
    let mut cache = Cache::default();
    let gid = sf(100);
    let cid = sf(301);
    let admin_user = sf(50);
    let admin_role = sf(201);

    let mut guild = make_guild(100, 1, "Guild");
    guild.roles = Some(vec![
        make_role(100, "@everyone", Permissions::VIEW_CHANNEL),
        make_role(201, "AdminRole", Permissions::ADMINISTRATOR),
    ]);
    guild.members = Some(vec![make_member(50, vec![201], None)]);

    let mut channel = make_channel(301, 100, "deny-channel", false);
    channel.permission_overwrites = Some(vec![ChannelOverwrite {
        id: admin_role,
        kind: 0,
        allow: Permissions::NONE,
        deny: Permissions::ALL,
    }]);

    cache.update(&GatewayEvent::GuildCreate(guild));
    cache.update(&GatewayEvent::ChannelCreate(channel));

    let perms = cache
        .member_permissions_in_channel(gid, cid, admin_user)
        .expect("permissions computed");

    assert_eq!(perms, Permissions::ALL);
}

#[test]
fn test_effective_permissions_overwrites_hierarchy() {
    let mut cache = Cache::default();
    let gid = sf(100);
    let cid = sf(301);
    let uid = sf(50);
    let role_a = sf(201);
    let role_b = sf(202);

    let mut guild = make_guild(100, 1, "Guild");
    guild.roles = Some(vec![
        make_role(100, "@everyone", Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES),
        make_role(201, "RoleA", Permissions::NONE),
        make_role(202, "RoleB", Permissions::NONE),
    ]);
    guild.members = Some(vec![make_member(50, vec![201, 202], None)]);

    // Channel overwrites:
    // Role A denies SEND_MESSAGES, Role B allows ATTACH_FILES
    // Member overwrite explicitly allows SEND_MESSAGES
    let mut channel = make_channel(301, 100, "chat", false);
    channel.permission_overwrites = Some(vec![
        ChannelOverwrite {
            id: role_a,
            kind: 0,
            allow: Permissions::NONE,
            deny: Permissions::SEND_MESSAGES,
        },
        ChannelOverwrite {
            id: role_b,
            kind: 0,
            allow: Permissions::ATTACH_FILES,
            deny: Permissions::NONE,
        },
        ChannelOverwrite {
            id: uid,
            kind: 1,
            allow: Permissions::SEND_MESSAGES,
            deny: Permissions::NONE,
        },
    ]);

    cache.update(&GatewayEvent::GuildCreate(guild));
    cache.update(&GatewayEvent::ChannelCreate(channel));

    let perms = cache
        .member_permissions_in_channel(gid, cid, uid)
        .expect("permissions computed");

    // Member overwrite wins over Role A's deny, and Role B's allow attaches
    assert!(perms.contains(Permissions::VIEW_CHANNEL));
    assert!(perms.contains(Permissions::SEND_MESSAGES));
    assert!(perms.contains(Permissions::ATTACH_FILES));
}

#[test]
fn test_effective_permissions_implicit_rules() {
    let mut cache = Cache::default();
    let gid = sf(100);
    let cid = sf(301);
    let uid = sf(50);

    let mut guild = make_guild(100, 1, "Guild");
    guild.roles = Some(vec![make_role(
        100,
        "@everyone",
        Permissions::SEND_MESSAGES | Permissions::ATTACH_FILES, // Missing VIEW_CHANNEL!
    )]);
    guild.members = Some(vec![make_member(50, vec![], None)]);

    let channel = make_channel(301, 100, "chat", false);
    cache.update(&GatewayEvent::GuildCreate(guild));
    cache.update(&GatewayEvent::ChannelCreate(channel));

    let perms = cache
        .member_permissions_in_channel(gid, cid, uid)
        .expect("permissions computed");

    // Without VIEW_CHANNEL, implicit text permissions strip everything to NONE
    assert_eq!(perms, Permissions::NONE);
}

#[test]
fn test_effective_permissions_timed_out_member() {
    let mut cache = Cache::default();
    let gid = sf(100);
    let cid = sf(301);
    let uid = sf(50);

    let future_timeout = "2099-01-01T00:00:00Z";
    let mut guild = make_guild(100, 1, "Guild");
    guild.roles = Some(vec![make_role(
        100,
        "@everyone",
        Permissions::VIEW_CHANNEL
            | Permissions::SEND_MESSAGES
            | Permissions::READ_MESSAGE_HISTORY
            | Permissions::EMBED_LINKS,
    )]);
    guild.members = Some(vec![make_member(50, vec![], Some(future_timeout))]);

    let channel = make_channel(301, 100, "chat", false);
    cache.update(&GatewayEvent::GuildCreate(guild));
    cache.update(&GatewayEvent::ChannelCreate(channel));

    let perms = cache
        .member_permissions_in_channel(gid, cid, uid)
        .expect("permissions computed");

    // Timed out members keep only VIEW_CHANNEL and READ_MESSAGE_HISTORY
    assert_eq!(
        perms,
        Permissions::VIEW_CHANNEL | Permissions::READ_MESSAGE_HISTORY
    );
}

#[test]
fn test_effective_permissions_obfuscated_channel_returns_none() {
    let mut cache = Cache::default();
    let gid = sf(100);
    let cid = sf(401);
    let owner_id = sf(42);
    let uid = sf(50);

    let guild = make_guild(100, 42, "Guild");
    let obf_ch = make_channel(401, 100, "___hidden___", true);

    cache.update(&GatewayEvent::GuildCreate(guild));
    cache.update(&GatewayEvent::ChannelCreate(obf_ch));

    // Even guild owner immediately gets Permissions::NONE for obfuscated channels
    let owner_perms = cache
        .member_permissions_in_channel(gid, cid, owner_id)
        .expect("result returned");
    assert_eq!(owner_perms, Permissions::NONE);

    // Regular member also gets Permissions::NONE
    let member_perms = cache
        .member_permissions_in_channel(gid, cid, uid)
        .expect("result returned");
    assert_eq!(member_perms, Permissions::NONE);
}

#[test]
fn test_cache_stats_consistency() {
    let mut cache = Cache::default();
    let mut g = make_guild(100, 1, "G1");
    g.roles = Some(vec![make_role(100, "@everyone", Permissions::NONE)]);
    g.channels = Some(vec![
        make_channel(301, 100, "c1", false),
        make_channel(302, 100, "c2", true), // obfuscated
    ]);
    g.members = Some(vec![make_member(10, vec![], None)]);

    cache.update(&GatewayEvent::GuildCreate(g));

    cache.update(&GatewayEvent::MessageCreate(make_message(1, 301, 10, "hi")));
    cache.update(&GatewayEvent::MessageCreate(make_message(2, 301, 10, "there")));

    let stats = cache.stats();
    assert_eq!(stats.guilds, 1);
    assert_eq!(stats.visible_channels, 1);
    assert_eq!(stats.obfuscated_channels, 1);
    assert_eq!(stats.total_channels(), 2);
    assert_eq!(stats.roles, 1);
    assert_eq!(stats.members, 1);
    assert_eq!(stats.users, 1);
    assert_eq!(stats.messages, 2);
    assert_eq!(stats.cached_messages(), 2);

    cache.clear();
    let empty_stats = cache.stats();
    assert_eq!(empty_stats.guilds, 0);
    assert_eq!(empty_stats.visible_channels, 0);
    assert_eq!(empty_stats.obfuscated_channels, 0);
    assert_eq!(empty_stats.roles, 0);
    assert_eq!(empty_stats.members, 0);
    assert_eq!(empty_stats.users, 0);
    assert_eq!(empty_stats.messages, 0);
}
