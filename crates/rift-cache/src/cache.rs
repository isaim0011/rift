//! In-memory state cache for Discord bots.

use crate::config::CacheConfig;
use crate::permissions::{
    apply_implicit_permissions, apply_timeout, compute_base_permissions, compute_overwrites,
    is_timed_out,
};
use crate::stats::CacheStats;
use rift_core::permissions::Overwrite;
use rift_core::{Permissions, Snowflake};
use rift_model::channel::Channel;
use rift_model::events::{
    ChannelCreate, ChannelDelete, ChannelUpdate, GuildCreate, GuildDelete, GuildMemberAdd,
    GuildMemberRemove, GuildMemberUpdate, GuildMembersChunk, GuildRoleCreate, GuildRoleDelete,
    GuildRoleUpdate, GuildUpdate, MessageCreate, MessageDelete, MessageDeleteBulk, MessageUpdate,
    Ready, ThreadCreate, ThreadDelete, ThreadListSync, ThreadUpdate, UserUpdate,
    VoiceStateUpdate,
};
use rift_model::guild::{Guild, GuildMember, Role};
use rift_model::message::Message;
use rift_model::presence::PresenceUpdate;
use rift_model::user::User;
use rift_model::voice::VoiceState;
use rift_model::GatewayEvent;
use std::collections::{HashMap, HashSet, VecDeque};

/// In-memory state cache for Discord gateway entities.
#[derive(Clone, Debug)]
pub struct Cache {
    config: CacheConfig,
    guilds: HashMap<Snowflake, Guild>,
    channels: HashMap<Snowflake, Channel>,
    obfuscated_channels: HashSet<Snowflake>,
    roles: HashMap<Snowflake, HashMap<Snowflake, Role>>,
    members: HashMap<Snowflake, HashMap<Snowflake, GuildMember>>,
    member_order: HashMap<Snowflake, VecDeque<Snowflake>>,
    users: HashMap<Snowflake, User>,
    user_order: VecDeque<Snowflake>,
    messages: HashMap<Snowflake, VecDeque<Message>>,
    voice_states: HashMap<Snowflake, HashMap<Snowflake, VoiceState>>,
    presences: HashMap<Snowflake, HashMap<Snowflake, PresenceUpdate>>,
    current_user: Option<User>,
}

impl Default for Cache {
    fn default() -> Self {
        Self::new(CacheConfig::default())
    }
}

impl Cache {
    /// Creates a new cache instance with the provided configuration.
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            guilds: HashMap::new(),
            channels: HashMap::new(),
            obfuscated_channels: HashSet::new(),
            roles: HashMap::new(),
            members: HashMap::new(),
            member_order: HashMap::new(),
            users: HashMap::new(),
            user_order: VecDeque::new(),
            messages: HashMap::new(),
            voice_states: HashMap::new(),
            presences: HashMap::new(),
            current_user: None,
        }
    }

    /// Access the cache configuration.
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Returns current entity counts and cache statistics.
    pub fn stats(&self) -> CacheStats {
        let guilds = self.guilds.len();
        let obfuscated_channels = self.obfuscated_channels.len();
        let visible_channels = self.channels.len().saturating_sub(obfuscated_channels);
        let roles = self.roles.values().map(|m| m.len()).sum();
        let members = self.members.values().map(|m| m.len()).sum();
        let users = self.users.len();
        let messages = self.messages.values().map(|d| d.len()).sum();

        CacheStats {
            guilds,
            visible_channels,
            obfuscated_channels,
            roles,
            members,
            users,
            messages,
        }
    }

    /// Clears all cached state.
    pub fn clear(&mut self) {
        self.guilds.clear();
        self.channels.clear();
        self.obfuscated_channels.clear();
        self.roles.clear();
        self.members.clear();
        self.member_order.clear();
        self.users.clear();
        self.user_order.clear();
        self.messages.clear();
        self.voice_states.clear();
        self.presences.clear();
        self.current_user = None;
    }

    // --- Accessors ---

    /// Returns the cached guild with the given ID.
    pub fn guild(&self, id: Snowflake) -> Option<&Guild> {
        self.guilds.get(&id)
    }

    /// Returns an iterator over all cached guilds.
    pub fn guilds(&self) -> impl Iterator<Item = &Guild> {
        self.guilds.values()
    }

    /// Returns whether the channel with the specified ID exists in cache and is obfuscated.
    pub fn is_channel_obfuscated(&self, id: Snowflake) -> bool {
        self.obfuscated_channels.contains(&id)
    }

    /// Returns the cached channel if it exists and is visible (not obfuscated).
    pub fn visible_channel(&self, id: Snowflake) -> Option<&Channel> {
        if self.is_channel_obfuscated(id) {
            None
        } else {
            self.channels.get(&id)
        }
    }

    /// Returns the cached channel if visible. Obfuscated channels return `None` to prevent
    /// exposing fake or placeholder names and topics.
    pub fn channel(&self, id: Snowflake) -> Option<&Channel> {
        self.visible_channel(id)
    }

    /// Returns the raw cached channel object regardless of obfuscation.
    ///
    /// If the channel is obfuscated, its placeholder name and topic are guaranteed
    /// to have been sanitized to `None`.
    pub fn raw_channel(&self, id: Snowflake) -> Option<&Channel> {
        self.channels.get(&id)
    }

    /// Returns an iterator over all cached visible channels.
    pub fn channels(&self) -> impl Iterator<Item = &Channel> {
        self.channels
            .values()
            .filter(|ch| !self.is_channel_obfuscated(ch.id))
    }

    /// Returns the cached role within a guild.
    pub fn role(&self, guild_id: Snowflake, role_id: Snowflake) -> Option<&Role> {
        self.roles.get(&guild_id).and_then(|r| r.get(&role_id))
    }

    /// Returns an iterator over all cached roles for a guild.
    pub fn roles(&self, guild_id: Snowflake) -> Option<impl Iterator<Item = &Role>> {
        self.roles.get(&guild_id).map(|r| r.values())
    }

    /// Returns the cached guild member.
    pub fn member(&self, guild_id: Snowflake, user_id: Snowflake) -> Option<&GuildMember> {
        self.members.get(&guild_id).and_then(|m| m.get(&user_id))
    }

    /// Returns an iterator over all cached members in a guild.
    pub fn members(&self, guild_id: Snowflake) -> Option<impl Iterator<Item = &GuildMember>> {
        self.members.get(&guild_id).map(|m| m.values())
    }

    /// Returns the cached user.
    pub fn user(&self, id: Snowflake) -> Option<&User> {
        self.users.get(&id)
    }

    /// Returns an iterator over all cached standalone users.
    pub fn users(&self) -> impl Iterator<Item = &User> {
        self.users.values()
    }

    /// Returns the current bot user received from `READY` or `USER_UPDATE`.
    pub fn current_user(&self) -> Option<&User> {
        self.current_user.as_ref()
    }

    /// Returns the cached voice state for a user in a guild.
    pub fn voice_state(&self, guild_id: Snowflake, user_id: Snowflake) -> Option<&VoiceState> {
        self.voice_states
            .get(&guild_id)
            .and_then(|vs| vs.get(&user_id))
    }

    /// Returns an iterator over all cached voice states for a guild.
    pub fn voice_states(&self, guild_id: Snowflake) -> Option<impl Iterator<Item = &VoiceState>> {
        self.voice_states.get(&guild_id).map(|vs| vs.values())
    }

    /// Returns the cached presence for a user in a guild.
    pub fn presence(&self, guild_id: Snowflake, user_id: Snowflake) -> Option<&PresenceUpdate> {
        self.presences
            .get(&guild_id)
            .and_then(|p| p.get(&user_id))
    }

    /// Returns an iterator over all cached presences for a guild.
    pub fn presences(&self, guild_id: Snowflake) -> Option<impl Iterator<Item = &PresenceUpdate>> {
        self.presences.get(&guild_id).map(|p| p.values())
    }

    /// Returns a message from a channel's ring buffer by ID.
    pub fn message(&self, channel_id: Snowflake, message_id: Snowflake) -> Option<&Message> {
        self.messages
            .get(&channel_id)
            .and_then(|deque| deque.iter().find(|m| m.id == message_id))
    }

    /// Returns the channel's message ring buffer.
    pub fn channel_messages(&self, channel_id: Snowflake) -> Option<&VecDeque<Message>> {
        self.messages.get(&channel_id)
    }

    // --- Permissions ---

    /// Computes exact effective permissions for a member in a specific channel.
    ///
    /// Adheres strictly to Discord's permission resolution hierarchy:
    /// 1. Obfuscated channels immediately return `Permissions::NONE`.
    /// 2. Guild owner receives `Permissions::ALL`.
    /// 3. Administrator role receives `Permissions::ALL`.
    /// 4. Evaluates `@everyone` overwrite, then aggregated role overwrites, then member overwrite.
    /// 5. Applies implicit text channel permissions (e.g. missing `VIEW_CHANNEL` strips all).
    /// 6. Applies timeout restrictions (timed-out members retain only `VIEW_CHANNEL` & `READ_MESSAGE_HISTORY`).
    pub fn member_permissions_in_channel(
        &self,
        guild_id: Snowflake,
        channel_id: Snowflake,
        user_id: Snowflake,
    ) -> Option<Permissions> {
        let channel = self.channels.get(&channel_id)?;
        if let Some(ch_gid) = channel.guild_id {
            if ch_gid != guild_id {
                return None;
            }
        }

        // Nov 16 Channel Obfuscation Rule
        if self.is_channel_obfuscated(channel_id) || channel.is_obfuscated() {
            return Some(Permissions::NONE);
        }

        let guild = self.guild(guild_id)?;
        let is_owner = guild.owner_id == Some(user_id);
        if is_owner {
            return Some(Permissions::ALL);
        }

        let member = self.member(guild_id, user_id)?;

        // Base permissions
        let everyone_perms = self
            .role(guild_id, guild_id)
            .map(|r| r.permissions)
            .unwrap_or(Permissions::NONE);

        let member_role_perms = member
            .roles
            .iter()
            .filter_map(|&rid| self.role(guild_id, rid).map(|r| r.permissions));

        let base = compute_base_permissions(is_owner, everyone_perms, member_role_perms);
        if base.contains(Permissions::ADMINISTRATOR) {
            return Some(Permissions::ALL);
        }

        // Channel overwrites
        let everyone_ow = channel.permission_overwrites.as_ref().and_then(|ows| {
            ows.iter()
                .find(|o| o.kind == 0 && o.id == guild_id)
                .map(|o| Overwrite {
                    allow: o.allow,
                    deny: o.deny,
                })
        });

        let role_ows = channel
            .permission_overwrites
            .as_ref()
            .map(|ows| {
                ows.iter()
                    .filter(|o| o.kind == 0 && member.roles.contains(&o.id))
                    .map(|o| Overwrite {
                        allow: o.allow,
                        deny: o.deny,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let member_ow = channel.permission_overwrites.as_ref().and_then(|ows| {
            ows.iter()
                .find(|o| o.kind == 1 && o.id == user_id)
                .map(|o| Overwrite {
                    allow: o.allow,
                    deny: o.deny,
                })
        });

        let mut perms = compute_overwrites(base, everyone_ow, role_ows, member_ow);
        perms = apply_implicit_permissions(perms);

        if is_timed_out(member.communication_disabled_until.as_deref()) {
            perms = apply_timeout(perms, is_owner);
        }

        Some(perms)
    }

    // --- State Update ---

    /// Deterministically updates internal cache state from a Gateway event.
    pub fn update(&mut self, event: &GatewayEvent) {
        match event {
            GatewayEvent::Ready(ev) => self.handle_ready(ev),
            GatewayEvent::GuildCreate(ev) => self.handle_guild_create(ev),
            GatewayEvent::GuildUpdate(ev) => self.handle_guild_update(ev),
            GatewayEvent::GuildDelete(ev) => self.handle_guild_delete(ev),
            GatewayEvent::ChannelCreate(ev) => self.handle_channel_create(ev),
            GatewayEvent::ChannelUpdate(ev) => self.handle_channel_update(ev),
            GatewayEvent::ChannelDelete(ev) => self.handle_channel_delete(ev),
            GatewayEvent::ThreadCreate(ev) => self.handle_thread_create(ev),
            GatewayEvent::ThreadUpdate(ev) => self.handle_thread_update(ev),
            GatewayEvent::ThreadDelete(ev) => self.handle_thread_delete(ev),
            GatewayEvent::ThreadListSync(ev) => self.handle_thread_list_sync(ev),
            GatewayEvent::GuildMemberAdd(ev) => self.handle_guild_member_add(ev),
            GatewayEvent::GuildMemberUpdate(ev) => self.handle_guild_member_update(ev),
            GatewayEvent::GuildMemberRemove(ev) => self.handle_guild_member_remove(ev),
            GatewayEvent::GuildMembersChunk(ev) => self.handle_guild_members_chunk(ev),
            GatewayEvent::GuildRoleCreate(ev) => self.handle_guild_role_create(ev),
            GatewayEvent::GuildRoleUpdate(ev) => self.handle_guild_role_update(ev),
            GatewayEvent::GuildRoleDelete(ev) => self.handle_guild_role_delete(ev),
            GatewayEvent::MessageCreate(ev) => self.handle_message_create(ev),
            GatewayEvent::MessageUpdate(ev) => self.handle_message_update(ev),
            GatewayEvent::MessageDelete(ev) => self.handle_message_delete(ev),
            GatewayEvent::MessageDeleteBulk(ev) => self.handle_message_delete_bulk(ev),
            GatewayEvent::VoiceStateUpdate(ev) => self.handle_voice_state_update(ev),
            GatewayEvent::PresenceUpdate(ev) => self.handle_presence_update(ev),
            GatewayEvent::UserUpdate(ev) => self.handle_user_update(ev),
            _ => {}
        }
    }

    // --- Internal Event Handlers ---

    fn handle_ready(&mut self, ev: &Ready) {
        if self.config.cache_users {
            self.current_user = Some(ev.user.clone());
            self.insert_user_with_limit(ev.user.clone());
        }
        if self.config.cache_guilds {
            for ug in &ev.guilds {
                self.guilds.entry(ug.id).or_insert_with(|| Guild {
                    id: ug.id,
                    name: String::new(),
                    icon: None,
                    icon_hash: None,
                    splash: None,
                    discovery_splash: None,
                    owner: None,
                    owner_id: None,
                    permissions: None,
                    region: None,
                    afk_channel_id: None,
                    afk_timeout: None,
                    widget_enabled: None,
                    widget_channel_id: None,
                    verification_level: None,
                    default_message_notifications: None,
                    explicit_content_filter: None,
                    roles: None,
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
                    unavailable: ug.unavailable.or(Some(true)),
                    member_count: None,
                    voice_states: None,
                    members: None,
                    channels: None,
                    threads: None,
                    presences: None,
                    stage_instances: None,
                    guild_scheduled_events: None,
                });
            }
        }
    }

    fn handle_guild_create(&mut self, ev: &GuildCreate) {
        let guild_id = ev.id;

        if self.config.cache_guilds {
            self.guilds.insert(guild_id, ev.clone());
        }

        if self.config.cache_roles {
            if let Some(roles) = &ev.roles {
                let role_map = self.roles.entry(guild_id).or_default();
                for r in roles {
                    role_map.insert(r.id, r.clone());
                }
            }
        }

        if self.config.cache_channels {
            if let Some(channels) = &ev.channels {
                for ch in channels {
                    let mut ch = ch.clone();
                    if ch.guild_id.is_none() {
                        ch.guild_id = Some(guild_id);
                    }
                    self.store_channel(ch);
                }
            }
            if let Some(threads) = &ev.threads {
                for th in threads {
                    let mut th = th.clone();
                    if th.guild_id.is_none() {
                        th.guild_id = Some(guild_id);
                    }
                    self.store_channel(th);
                }
            }
        }

        if self.config.cache_members {
            if let Some(members) = &ev.members {
                for m in members {
                    self.insert_member_with_limit(guild_id, m.clone());
                }
            }
        }

        if self.config.cache_voice_states {
            if let Some(voice_states) = &ev.voice_states {
                let vs_map = self.voice_states.entry(guild_id).or_default();
                for vs in voice_states {
                    vs_map.insert(vs.user_id, vs.clone());
                }
            }
        }

        if self.config.cache_presences {
            if let Some(presences) = &ev.presences {
                let p_map = self.presences.entry(guild_id).or_default();
                for p in presences {
                    p_map.insert(p.user.id, p.clone());
                }
            }
        }
    }

    fn handle_guild_update(&mut self, ev: &GuildUpdate) {
        let guild_id = ev.id;
        if self.config.cache_guilds {
            self.guilds.insert(guild_id, ev.clone());
        }
        if self.config.cache_roles {
            if let Some(roles) = &ev.roles {
                let role_map = self.roles.entry(guild_id).or_default();
                for r in roles {
                    role_map.insert(r.id, r.clone());
                }
            }
        }
    }

    fn handle_guild_delete(&mut self, ev: &GuildDelete) {
        let guild_id = ev.id;
        if ev.unavailable == Some(true) {
            if let Some(g) = self.guilds.get_mut(&guild_id) {
                g.unavailable = Some(true);
            }
        } else {
            self.guilds.remove(&guild_id);
            self.roles.remove(&guild_id);
            self.members.remove(&guild_id);
            self.member_order.remove(&guild_id);
            self.voice_states.remove(&guild_id);
            self.presences.remove(&guild_id);

            // Remove all associated channels
            let channel_ids_to_remove: Vec<Snowflake> = self
                .channels
                .iter()
                .filter(|(_, ch)| ch.guild_id == Some(guild_id))
                .map(|(&id, _)| id)
                .collect();

            for id in channel_ids_to_remove {
                self.channels.remove(&id);
                self.obfuscated_channels.remove(&id);
                self.messages.remove(&id);
            }
        }
    }

    fn handle_channel_create(&mut self, ch: &ChannelCreate) {
        if self.config.cache_channels {
            self.store_channel(ch.clone());
        }
    }

    fn handle_channel_update(&mut self, ch: &ChannelUpdate) {
        if self.config.cache_channels {
            self.store_channel(ch.clone());
        }
    }

    fn handle_channel_delete(&mut self, ch: &ChannelDelete) {
        if self.config.cache_channels {
            self.channels.remove(&ch.id);
            self.obfuscated_channels.remove(&ch.id);
            self.messages.remove(&ch.id);
        }
    }

    fn handle_thread_create(&mut self, th: &ThreadCreate) {
        if self.config.cache_channels {
            self.store_channel(th.clone());
        }
    }

    fn handle_thread_update(&mut self, th: &ThreadUpdate) {
        if self.config.cache_channels {
            self.store_channel(th.clone());
        }
    }

    fn handle_thread_delete(&mut self, th: &ThreadDelete) {
        if self.config.cache_channels {
            self.channels.remove(&th.id);
            self.obfuscated_channels.remove(&th.id);
            self.messages.remove(&th.id);
        }
    }

    fn handle_thread_list_sync(&mut self, sync: &ThreadListSync) {
        if self.config.cache_channels {
            for th in &sync.threads {
                let mut th = th.clone();
                if th.guild_id.is_none() {
                    th.guild_id = Some(sync.guild_id);
                }
                self.store_channel(th);
            }
        }
    }

    fn handle_guild_member_add(&mut self, ev: &GuildMemberAdd) {
        if self.config.cache_members {
            self.insert_member_with_limit(ev.guild_id, ev.member.clone());
        }
    }

    fn handle_guild_member_update(&mut self, ev: &GuildMemberUpdate) {
        let user_id = ev.user.id;
        if self.config.cache_users {
            self.insert_user_with_limit(ev.user.clone());
        }

        if self.config.cache_members {
            let members = self.members.entry(ev.guild_id).or_default();
            if let Some(m) = members.get_mut(&user_id) {
                m.roles = ev.roles.clone();
                m.nick = ev.nick.clone();
                m.avatar = ev.avatar.clone();
                if let Some(joined) = &ev.joined_at {
                    m.joined_at = joined.clone();
                }
                m.premium_since = ev.premium_since.clone();
                m.deaf = ev.deaf;
                m.mute = ev.mute;
                m.pending = ev.pending;
                m.communication_disabled_until = ev.communication_disabled_until.clone();
                m.flags = ev.flags;
                m.user = Some(ev.user.clone());
                m.avatar_decoration_data = ev.avatar_decoration_data.clone();
            } else {
                let new_member = GuildMember {
                    user: Some(ev.user.clone()),
                    nick: ev.nick.clone(),
                    avatar: ev.avatar.clone(),
                    roles: ev.roles.clone(),
                    joined_at: ev.joined_at.clone().unwrap_or_default(),
                    premium_since: ev.premium_since.clone(),
                    deaf: ev.deaf,
                    mute: ev.mute,
                    flags: ev.flags,
                    pending: ev.pending,
                    permissions: None,
                    communication_disabled_until: ev.communication_disabled_until.clone(),
                    avatar_decoration_data: ev.avatar_decoration_data.clone(),
                };
                self.insert_member_with_limit(ev.guild_id, new_member);
            }
        }
    }

    fn handle_guild_member_remove(&mut self, ev: &GuildMemberRemove) {
        let user_id = ev.user.id;
        if self.config.cache_members {
            if let Some(members) = self.members.get_mut(&ev.guild_id) {
                members.remove(&user_id);
            }
            if let Some(order) = self.member_order.get_mut(&ev.guild_id) {
                order.retain(|&id| id != user_id);
            }
        }
        if let Some(vs) = self.voice_states.get_mut(&ev.guild_id) {
            vs.remove(&user_id);
        }
        if let Some(p) = self.presences.get_mut(&ev.guild_id) {
            p.remove(&user_id);
        }
    }

    fn handle_guild_members_chunk(&mut self, chunk: &GuildMembersChunk) {
        if self.config.cache_members {
            for m in &chunk.members {
                self.insert_member_with_limit(chunk.guild_id, m.clone());
            }
        }
        if self.config.cache_presences {
            if let Some(presences) = &chunk.presences {
                let p_map = self.presences.entry(chunk.guild_id).or_default();
                for p in presences {
                    p_map.insert(p.user.id, p.clone());
                }
            }
        }
    }

    fn handle_guild_role_create(&mut self, ev: &GuildRoleCreate) {
        if self.config.cache_roles {
            self.roles
                .entry(ev.guild_id)
                .or_default()
                .insert(ev.role.id, ev.role.clone());
        }
    }

    fn handle_guild_role_update(&mut self, ev: &GuildRoleUpdate) {
        if self.config.cache_roles {
            self.roles
                .entry(ev.guild_id)
                .or_default()
                .insert(ev.role.id, ev.role.clone());
        }
    }

    fn handle_guild_role_delete(&mut self, ev: &GuildRoleDelete) {
        if self.config.cache_roles {
            if let Some(roles) = self.roles.get_mut(&ev.guild_id) {
                roles.remove(&ev.role_id);
            }
        }
    }

    fn handle_message_create(&mut self, msg: &MessageCreate) {
        if self.config.cache_users {
            if let Some(author) = &msg.author {
                self.insert_user_with_limit(author.clone());
            }
        }

        if self.config.max_messages_per_channel > 0 {
            let deque = self.messages.entry(msg.channel_id).or_default();
            if deque.len() >= self.config.max_messages_per_channel {
                deque.pop_front();
            }
            deque.push_back(msg.clone());
        }
    }

    fn handle_message_update(&mut self, msg: &MessageUpdate) {
        if let Some(deque) = self.messages.get_mut(&msg.channel_id) {
            if let Some(existing) = deque.iter_mut().find(|m| m.id == msg.id) {
                *existing = msg.clone();
            }
        }
    }

    fn handle_message_delete(&mut self, ev: &MessageDelete) {
        if let Some(deque) = self.messages.get_mut(&ev.channel_id) {
            deque.retain(|m| m.id != ev.id);
        }
    }

    fn handle_message_delete_bulk(&mut self, ev: &MessageDeleteBulk) {
        if let Some(deque) = self.messages.get_mut(&ev.channel_id) {
            let ids: HashSet<Snowflake> = ev.ids.iter().copied().collect();
            deque.retain(|m| !ids.contains(&m.id));
        }
    }

    fn handle_voice_state_update(&mut self, vs: &VoiceStateUpdate) {
        if self.config.cache_voice_states {
            if let Some(gid) = vs.guild_id {
                let map = self.voice_states.entry(gid).or_default();
                if vs.channel_id.is_none() {
                    map.remove(&vs.user_id);
                } else {
                    map.insert(vs.user_id, vs.clone());
                }
            }
        }
    }

    fn handle_presence_update(&mut self, pres: &PresenceUpdate) {
        if self.config.cache_presences {
            if let Some(gid) = pres.guild_id {
                self.presences
                    .entry(gid)
                    .or_default()
                    .insert(pres.user.id, pres.clone());
            }
        }

        if self.config.cache_users {
            let uid = pres.user.id;
            if let Some(user) = self.users.get_mut(&uid) {
                if let Some(username) = &pres.user.username {
                    user.username = username.clone();
                }
                if let Some(discriminator) = &pres.user.discriminator {
                    user.discriminator = Some(discriminator.clone());
                }
                if let Some(avatar) = &pres.user.avatar {
                    user.avatar = Some(avatar.clone());
                }
                if let Some(global_name) = &pres.user.global_name {
                    user.global_name = Some(global_name.clone());
                }
            }
        }
    }

    fn handle_user_update(&mut self, user: &UserUpdate) {
        if self.config.cache_users {
            self.current_user = Some(user.clone());
            self.insert_user_with_limit(user.clone());
        }
    }

    // --- Helpers ---

    /// Stores a channel, sanitizing fake names and topics if obfuscated.
    fn store_channel(&mut self, mut ch: Channel) {
        if ch.is_obfuscated() {
            self.obfuscated_channels.insert(ch.id);
            ch.name = None;
            ch.topic = None;
            ch.permission_overwrites = None;
        } else {
            self.obfuscated_channels.remove(&ch.id);
        }
        self.channels.insert(ch.id, ch);
    }

    /// Inserts a guild member while enforcing `max_members_per_guild` FIFO eviction.
    fn insert_member_with_limit(&mut self, guild_id: Snowflake, member: GuildMember) {
        if let Some(user) = &member.user {
            let user_id = user.id;
            if self.config.cache_users {
                self.insert_user_with_limit(user.clone());
            }

            let members = self.members.entry(guild_id).or_default();
            let order = self.member_order.entry(guild_id).or_default();

            if !members.contains_key(&user_id) {
                if let Some(limit) = self.config.max_members_per_guild {
                    if limit == 0 {
                        return;
                    }
                    while members.len() >= limit {
                        if let Some(oldest) = order.pop_front() {
                            members.remove(&oldest);
                        } else {
                            break;
                        }
                    }
                }
                order.push_back(user_id);
            }
            members.insert(user_id, member);
        }
    }

    /// Inserts a user while enforcing `max_users` FIFO eviction.
    fn insert_user_with_limit(&mut self, user: User) {
        let user_id = user.id;
        if !self.users.contains_key(&user_id) {
            if let Some(limit) = self.config.max_users {
                if limit == 0 {
                    return;
                }
                while self.users.len() >= limit {
                    if let Some(oldest) = self.user_order.pop_front() {
                        self.users.remove(&oldest);
                    } else {
                        break;
                    }
                }
            }
            self.user_order.push_back(user_id);
        }
        self.users.insert(user_id, user);
    }
}
