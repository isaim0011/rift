# Plan: rift-model Implementation

## Goals
1. Provide a non-exhaustive `GatewayEvent` enum covering all Discord Gateway dispatch events:
   - Lifecycle: `READY`, `RESUMED`
   - Guilds: `GUILD_CREATE`, `GUILD_UPDATE`, `GUILD_DELETE`, `GUILD_AUDIT_LOG_ENTRY_CREATE`, `GUILD_BAN_ADD`, `GUILD_BAN_REMOVE`, `GUILD_EMOJIS_UPDATE`, `GUILD_STICKERS_UPDATE`, `GUILD_INTEGRATIONS_UPDATE`
   - Channels: `CHANNEL_CREATE`, `CHANNEL_UPDATE`, `CHANNEL_DELETE`, `CHANNEL_PINS_UPDATE`
   - Threads: `THREAD_CREATE`, `THREAD_UPDATE`, `THREAD_DELETE`, `THREAD_LIST_SYNC`, `THREAD_MEMBER_UPDATE`, `THREAD_MEMBERS_UPDATE`
   - Members: `GUILD_MEMBER_ADD`, `GUILD_MEMBER_UPDATE`, `GUILD_MEMBER_REMOVE`, `GUILD_MEMBERS_CHUNK`
   - Roles: `GUILD_ROLE_CREATE`, `GUILD_ROLE_UPDATE`, `GUILD_ROLE_DELETE`
   - Messages: `MESSAGE_CREATE`, `MESSAGE_UPDATE`, `MESSAGE_DELETE`, `MESSAGE_DELETE_BULK`, `MESSAGE_POLL_VOTE_ADD`, `MESSAGE_POLL_VOTE_REMOVE`
   - Reactions: `MESSAGE_REACTION_ADD`, `MESSAGE_REACTION_REMOVE`, `MESSAGE_REACTION_REMOVE_ALL`, `MESSAGE_REACTION_REMOVE_EMOJI`
   - Interactions: `INTERACTION_CREATE`, `APPLICATION_COMMAND_PERMISSIONS_UPDATE`
   - Voice: `VOICE_STATE_UPDATE`, `VOICE_SERVER_UPDATE`, `VOICE_CHANNEL_EFFECT_SEND`
   - Presence: `PRESENCE_UPDATE`, `TYPING_START`, `USER_UPDATE`
   - AutoMod: `AUTO_MODERATION_RULE_CREATE`, `AUTO_MODERATION_RULE_UPDATE`, `AUTO_MODERATION_RULE_DELETE`, `AUTO_MODERATION_ACTION_EXECUTION`
   - Monetization: `ENTITLEMENT_CREATE`, `ENTITLEMENT_UPDATE`, `ENTITLEMENT_DELETE`, `SUBSCRIPTION_CREATE`, `SUBSCRIPTION_UPDATE`, `SUBSCRIPTION_DELETE`
   - Scheduled Events & Soundboard: `GUILD_SCHEDULED_EVENT_*`, `GUILD_SOUNDBOARD_*`, `STAGE_INSTANCE_*`, `INTEGRATION_*`, `INVITE_*`, `WEBHOOKS_UPDATE`
   - Fallback: `Unknown { name: String, data: Box<RawValue> }` (guarantees new events never fail).
2. `GatewayEvent::parse(name: &str, raw: &RawValue) -> Result<Self, serde_json::Error>`
3. Typed payload structs with:
   - IDs as `rift_core::Snowflake` (accepting string decimal format as well as integers in JSON).
   - Permissions as `rift_core::Permissions` (accepting string decimal format as well as integers in JSON).
   - Obfuscated channels: `CHANNEL_OBFUSCATED = 1 << 17` flag representable in `ChannelFlags`, and `is_obfuscated()` method on `Channel`. Channel metadata fields are optional and never assumed present.
   - Unknown/optional fields tolerated.
4. `#![warn(missing_docs)]` and doc comments on all public items.
5. Unit tests with realistic payloads for the 10 most common events, an unknown event, and extra/unknown fields.
