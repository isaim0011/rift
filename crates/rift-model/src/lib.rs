//! # rift-model
//!
//! Strongly-typed Discord Gateway dispatch events and models.
//!
//! Designed for zero-overhead lazy parsing from raw gateway events
//! (`rift_runtime::Event.data: Box<RawValue>`).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod auto_moderation;
pub mod channel;
pub mod events;
pub mod flags;
pub mod guild;
pub mod interaction;
pub mod message;
pub mod monetization;
pub mod presence;
pub mod serde_util;
pub mod user;
pub mod voice;

pub use auto_moderation::{
    AutoModerationAction, AutoModerationActionExecution, AutoModerationActionMetadata,
    AutoModerationRule, AutoModerationTriggerMetadata,
};
pub use channel::{
    Channel, ChannelOverwrite, ChannelType, DefaultReaction, ForumTag, ThreadMember,
    ThreadMetadata,
};
pub use events::*;
pub use flags::ChannelFlags;
pub use guild::{
    AuditLogEntry, Emoji, Guild, GuildMember, Integration, IntegrationAccount,
    IntegrationApplication, Role, RoleTags, SoundboardSound, StageInstance, Sticker,
    UnavailableGuild,
};
pub use interaction::{ApplicationCommandPermission, Interaction, InteractionType};
pub use message::{
    Attachment, ChannelMention, Embed, EmbedAuthor, EmbedField, EmbedFooter, EmbedImage,
    EmbedProvider, EmbedThumbnail, EmbedVideo, Message, MessageActivity, MessageInteraction,
    MessageReference, Poll, PollAnswer, PollAnswerCount, PollMedia, PollResults, Reaction,
    ReactionCountDetails, ReactionEmoji, StickerItem,
};
pub use monetization::{Entitlement, Subscription};
pub use presence::{
    Activity, ActivityAssets, ActivityButton, ActivityEmoji, ActivityParty, ActivitySecrets,
    ActivityTimestamps, ClientStatus, PresenceUpdate,
};
pub use rift_core::{Permissions, Snowflake};
pub use user::{PresenceUser, User};
pub use voice::{VoiceChannelEffect, VoiceServerUpdate, VoiceState};

use serde_json::value::RawValue;

/// A typed Discord Gateway dispatch event.
///
/// Dispatched with Gateway opcode `0`. New or unhandled events fall back to
/// [`GatewayEvent::Unknown`] so updates to the Discord API never cause deserialization failure.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum GatewayEvent {
    /// Handshake completed; initial state provided.
    Ready(Ready),
    /// Reconnected and replayed missed events.
    Resumed(Resumed),

    /// Guild joined, became available, or lazy-loaded at startup.
    GuildCreate(GuildCreate),
    /// Guild settings updated.
    GuildUpdate(GuildUpdate),
    /// Guild became unavailable or user left/was removed.
    GuildDelete(GuildDelete),

    /// New channel created.
    ChannelCreate(ChannelCreate),
    /// Channel settings updated.
    ChannelUpdate(ChannelUpdate),
    /// Channel deleted.
    ChannelDelete(ChannelDelete),
    /// Message pinned or unpinned in a text channel.
    ChannelPinsUpdate(ChannelPinsUpdate),

    /// Thread created or unarchived.
    ThreadCreate(ThreadCreate),
    /// Thread settings updated.
    ThreadUpdate(ThreadUpdate),
    /// Thread deleted.
    ThreadDelete(ThreadDelete),
    /// Active threads synchronized.
    ThreadListSync(ThreadListSync),
    /// Current user's thread member object updated.
    ThreadMemberUpdate(ThreadMemberUpdate),
    /// Members added to or removed from a thread.
    ThreadMembersUpdate(ThreadMembersUpdate),

    /// User joined a guild.
    GuildMemberAdd(GuildMemberAdd),
    /// Guild member settings/roles updated.
    GuildMemberUpdate(GuildMemberUpdate),
    /// User removed from guild (kicked, banned, or left).
    GuildMemberRemove(GuildMemberRemove),
    /// Response to Request Guild Members command.
    GuildMembersChunk(GuildMembersChunk),

    /// Guild role created.
    GuildRoleCreate(GuildRoleCreate),
    /// Guild role updated.
    GuildRoleUpdate(GuildRoleUpdate),
    /// Guild role deleted.
    GuildRoleDelete(GuildRoleDelete),

    /// Message created and sent.
    MessageCreate(MessageCreate),
    /// Message updated/edited.
    MessageUpdate(MessageUpdate),
    /// Message deleted.
    MessageDelete(MessageDelete),
    /// Multiple messages deleted in bulk.
    MessageDeleteBulk(MessageDeleteBulk),

    /// User added a reaction to a message.
    MessageReactionAdd(MessageReactionAdd),
    /// User removed a reaction from a message.
    MessageReactionRemove(MessageReactionRemove),
    /// All reactions removed from a message.
    MessageReactionRemoveAll(MessageReactionRemoveAll),
    /// All reactions of a specific emoji removed from a message.
    MessageReactionRemoveEmoji(MessageReactionRemoveEmoji),

    /// User invoked an application command or interaction.
    InteractionCreate(InteractionCreate),

    /// User voice status updated.
    VoiceStateUpdate(VoiceStateUpdate),
    /// Guild voice server connection updated.
    VoiceServerUpdate(VoiceServerUpdate),
    /// Voice channel effect (soundboard, animation) sent.
    VoiceChannelEffectSend(VoiceChannelEffectSend),

    /// User presence/activity updated.
    PresenceUpdate(PresenceUpdate),
    /// User began typing in a channel.
    TypingStart(TypingStart),

    /// Auto Moderation rule created.
    AutoModerationRuleCreate(AutoModerationRuleCreate),
    /// Auto Moderation rule updated.
    AutoModerationRuleUpdate(AutoModerationRuleUpdate),
    /// Auto Moderation rule deleted.
    AutoModerationRuleDelete(AutoModerationRuleDelete),
    /// Auto Moderation rule triggered and action executed.
    AutoModerationActionExecution(AutoModerationActionExecution),

    /// Entitlement created for application.
    EntitlementCreate(EntitlementCreate),
    /// Entitlement updated for application.
    EntitlementUpdate(EntitlementUpdate),
    /// Entitlement deleted for application.
    EntitlementDelete(EntitlementDelete),

    /// User subscription created.
    SubscriptionCreate(SubscriptionCreate),
    /// User subscription updated.
    SubscriptionUpdate(SubscriptionUpdate),
    /// User subscription deleted.
    SubscriptionDelete(SubscriptionDelete),

    /// Application command permissions updated in a guild.
    ApplicationCommandPermissionsUpdate(ApplicationCommandPermissionsUpdate),
    /// Audit log entry created.
    GuildAuditLogEntryCreate(GuildAuditLogEntryCreate),
    /// User banned from a guild.
    GuildBanAdd(GuildBanAdd),
    /// User unbanned from a guild.
    GuildBanRemove(GuildBanRemove),
    /// Guild custom emojis updated.
    GuildEmojisUpdate(GuildEmojisUpdate),
    /// Guild custom stickers updated.
    GuildStickersUpdate(GuildStickersUpdate),
    /// Guild integrations updated.
    GuildIntegrationsUpdate(GuildIntegrationsUpdate),

    /// Guild scheduled event created.
    GuildScheduledEventCreate(GuildScheduledEventCreate),
    /// Guild scheduled event updated.
    GuildScheduledEventUpdate(GuildScheduledEventUpdate),
    /// Guild scheduled event deleted.
    GuildScheduledEventDelete(GuildScheduledEventDelete),
    /// User subscribed to a scheduled event.
    GuildScheduledEventUserAdd(GuildScheduledEventUserAdd),
    /// User unsubscribed from a scheduled event.
    GuildScheduledEventUserRemove(GuildScheduledEventUserRemove),

    /// Guild soundboard sound created.
    GuildSoundboardSoundCreate(GuildSoundboardSoundCreate),
    /// Guild soundboard sound updated.
    GuildSoundboardSoundUpdate(GuildSoundboardSoundUpdate),
    /// Guild soundboard sound deleted.
    GuildSoundboardSoundDelete(GuildSoundboardSoundDelete),
    /// Guild soundboard sounds updated in bulk.
    GuildSoundboardSoundsUpdate(GuildSoundboardSoundsUpdate),

    /// Guild integration created.
    IntegrationCreate(IntegrationCreate),
    /// Guild integration updated.
    IntegrationUpdate(IntegrationUpdate),
    /// Guild integration deleted.
    IntegrationDelete(IntegrationDelete),

    /// Invite created.
    InviteCreate(InviteCreate),
    /// Invite deleted.
    InviteDelete(InviteDelete),

    /// User voted on a poll.
    MessagePollVoteAdd(MessagePollVoteAdd),
    /// User removed vote from a poll.
    MessagePollVoteRemove(MessagePollVoteRemove),

    /// Stage instance created.
    StageInstanceCreate(StageInstanceCreate),
    /// Stage instance updated.
    StageInstanceUpdate(StageInstanceUpdate),
    /// Stage instance deleted.
    StageInstanceDelete(StageInstanceDelete),

    /// Properties of the current user updated.
    UserUpdate(UserUpdate),
    /// Webhooks in a channel updated.
    WebhooksUpdate(WebhooksUpdate),

    /// An unhandled or future gateway event.
    Unknown {
        /// The raw event name string from the gateway `t` field.
        name: String,
        /// The raw JSON data from the gateway `d` field.
        data: Box<RawValue>,
    },
}

impl GatewayEvent {
    /// Parse a typed gateway event from its name and raw JSON payload.
    ///
    /// Unknown event names do not produce an error; they parse into [`GatewayEvent::Unknown`].
    pub fn parse(name: &str, raw: &RawValue) -> Result<Self, serde_json::Error> {
        match name {
            "READY" => serde_json::from_str::<Ready>(raw.get()).map(GatewayEvent::Ready),
            "RESUMED" => serde_json::from_str::<Resumed>(raw.get()).map(GatewayEvent::Resumed),

            "GUILD_CREATE" => serde_json::from_str::<GuildCreate>(raw.get()).map(GatewayEvent::GuildCreate),
            "GUILD_UPDATE" => serde_json::from_str::<GuildUpdate>(raw.get()).map(GatewayEvent::GuildUpdate),
            "GUILD_DELETE" => serde_json::from_str::<GuildDelete>(raw.get()).map(GatewayEvent::GuildDelete),

            "CHANNEL_CREATE" => serde_json::from_str::<ChannelCreate>(raw.get()).map(GatewayEvent::ChannelCreate),
            "CHANNEL_UPDATE" => serde_json::from_str::<ChannelUpdate>(raw.get()).map(GatewayEvent::ChannelUpdate),
            "CHANNEL_DELETE" => serde_json::from_str::<ChannelDelete>(raw.get()).map(GatewayEvent::ChannelDelete),
            "CHANNEL_PINS_UPDATE" => serde_json::from_str::<ChannelPinsUpdate>(raw.get()).map(GatewayEvent::ChannelPinsUpdate),

            "THREAD_CREATE" => serde_json::from_str::<ThreadCreate>(raw.get()).map(GatewayEvent::ThreadCreate),
            "THREAD_UPDATE" => serde_json::from_str::<ThreadUpdate>(raw.get()).map(GatewayEvent::ThreadUpdate),
            "THREAD_DELETE" => serde_json::from_str::<ThreadDelete>(raw.get()).map(GatewayEvent::ThreadDelete),
            "THREAD_LIST_SYNC" => serde_json::from_str::<ThreadListSync>(raw.get()).map(GatewayEvent::ThreadListSync),
            "THREAD_MEMBER_UPDATE" => serde_json::from_str::<ThreadMemberUpdate>(raw.get()).map(GatewayEvent::ThreadMemberUpdate),
            "THREAD_MEMBERS_UPDATE" => serde_json::from_str::<ThreadMembersUpdate>(raw.get()).map(GatewayEvent::ThreadMembersUpdate),

            "GUILD_MEMBER_ADD" => serde_json::from_str::<GuildMemberAdd>(raw.get()).map(GatewayEvent::GuildMemberAdd),
            "GUILD_MEMBER_UPDATE" => serde_json::from_str::<GuildMemberUpdate>(raw.get()).map(GatewayEvent::GuildMemberUpdate),
            "GUILD_MEMBER_REMOVE" => serde_json::from_str::<GuildMemberRemove>(raw.get()).map(GatewayEvent::GuildMemberRemove),
            "GUILD_MEMBERS_CHUNK" => serde_json::from_str::<GuildMembersChunk>(raw.get()).map(GatewayEvent::GuildMembersChunk),

            "GUILD_ROLE_CREATE" => serde_json::from_str::<GuildRoleCreate>(raw.get()).map(GatewayEvent::GuildRoleCreate),
            "GUILD_ROLE_UPDATE" => serde_json::from_str::<GuildRoleUpdate>(raw.get()).map(GatewayEvent::GuildRoleUpdate),
            "GUILD_ROLE_DELETE" => serde_json::from_str::<GuildRoleDelete>(raw.get()).map(GatewayEvent::GuildRoleDelete),

            "MESSAGE_CREATE" => serde_json::from_str::<MessageCreate>(raw.get()).map(GatewayEvent::MessageCreate),
            "MESSAGE_UPDATE" => serde_json::from_str::<MessageUpdate>(raw.get()).map(GatewayEvent::MessageUpdate),
            "MESSAGE_DELETE" => serde_json::from_str::<MessageDelete>(raw.get()).map(GatewayEvent::MessageDelete),
            "MESSAGE_DELETE_BULK" => serde_json::from_str::<MessageDeleteBulk>(raw.get()).map(GatewayEvent::MessageDeleteBulk),

            "MESSAGE_REACTION_ADD" => serde_json::from_str::<MessageReactionAdd>(raw.get()).map(GatewayEvent::MessageReactionAdd),
            "MESSAGE_REACTION_REMOVE" => serde_json::from_str::<MessageReactionRemove>(raw.get()).map(GatewayEvent::MessageReactionRemove),
            "MESSAGE_REACTION_REMOVE_ALL" => serde_json::from_str::<MessageReactionRemoveAll>(raw.get()).map(GatewayEvent::MessageReactionRemoveAll),
            "MESSAGE_REACTION_REMOVE_EMOJI" => serde_json::from_str::<MessageReactionRemoveEmoji>(raw.get()).map(GatewayEvent::MessageReactionRemoveEmoji),

            "INTERACTION_CREATE" => serde_json::from_str::<InteractionCreate>(raw.get()).map(GatewayEvent::InteractionCreate),

            "VOICE_STATE_UPDATE" => serde_json::from_str::<VoiceStateUpdate>(raw.get()).map(GatewayEvent::VoiceStateUpdate),
            "VOICE_SERVER_UPDATE" => serde_json::from_str::<VoiceServerUpdate>(raw.get()).map(GatewayEvent::VoiceServerUpdate),
            "VOICE_CHANNEL_EFFECT_SEND" => serde_json::from_str::<VoiceChannelEffectSend>(raw.get()).map(GatewayEvent::VoiceChannelEffectSend),

            "PRESENCE_UPDATE" => serde_json::from_str::<PresenceUpdate>(raw.get()).map(GatewayEvent::PresenceUpdate),
            "TYPING_START" => serde_json::from_str::<TypingStart>(raw.get()).map(GatewayEvent::TypingStart),

            "AUTO_MODERATION_RULE_CREATE" => serde_json::from_str::<AutoModerationRuleCreate>(raw.get()).map(GatewayEvent::AutoModerationRuleCreate),
            "AUTO_MODERATION_RULE_UPDATE" => serde_json::from_str::<AutoModerationRuleUpdate>(raw.get()).map(GatewayEvent::AutoModerationRuleUpdate),
            "AUTO_MODERATION_RULE_DELETE" => serde_json::from_str::<AutoModerationRuleDelete>(raw.get()).map(GatewayEvent::AutoModerationRuleDelete),
            "AUTO_MODERATION_ACTION_EXECUTION" => serde_json::from_str::<AutoModerationActionExecution>(raw.get()).map(GatewayEvent::AutoModerationActionExecution),

            "ENTITLEMENT_CREATE" => serde_json::from_str::<EntitlementCreate>(raw.get()).map(GatewayEvent::EntitlementCreate),
            "ENTITLEMENT_UPDATE" => serde_json::from_str::<EntitlementUpdate>(raw.get()).map(GatewayEvent::EntitlementUpdate),
            "ENTITLEMENT_DELETE" => serde_json::from_str::<EntitlementDelete>(raw.get()).map(GatewayEvent::EntitlementDelete),

            "SUBSCRIPTION_CREATE" => serde_json::from_str::<SubscriptionCreate>(raw.get()).map(GatewayEvent::SubscriptionCreate),
            "SUBSCRIPTION_UPDATE" => serde_json::from_str::<SubscriptionUpdate>(raw.get()).map(GatewayEvent::SubscriptionUpdate),
            "SUBSCRIPTION_DELETE" => serde_json::from_str::<SubscriptionDelete>(raw.get()).map(GatewayEvent::SubscriptionDelete),

            "APPLICATION_COMMAND_PERMISSIONS_UPDATE" => serde_json::from_str::<ApplicationCommandPermissionsUpdate>(raw.get()).map(GatewayEvent::ApplicationCommandPermissionsUpdate),
            "GUILD_AUDIT_LOG_ENTRY_CREATE" => serde_json::from_str::<GuildAuditLogEntryCreate>(raw.get()).map(GatewayEvent::GuildAuditLogEntryCreate),
            "GUILD_BAN_ADD" => serde_json::from_str::<GuildBanAdd>(raw.get()).map(GatewayEvent::GuildBanAdd),
            "GUILD_BAN_REMOVE" => serde_json::from_str::<GuildBanRemove>(raw.get()).map(GatewayEvent::GuildBanRemove),
            "GUILD_EMOJIS_UPDATE" => serde_json::from_str::<GuildEmojisUpdate>(raw.get()).map(GatewayEvent::GuildEmojisUpdate),
            "GUILD_STICKERS_UPDATE" => serde_json::from_str::<GuildStickersUpdate>(raw.get()).map(GatewayEvent::GuildStickersUpdate),
            "GUILD_INTEGRATIONS_UPDATE" => serde_json::from_str::<GuildIntegrationsUpdate>(raw.get()).map(GatewayEvent::GuildIntegrationsUpdate),

            "GUILD_SCHEDULED_EVENT_CREATE" => serde_json::from_str::<GuildScheduledEventCreate>(raw.get()).map(GatewayEvent::GuildScheduledEventCreate),
            "GUILD_SCHEDULED_EVENT_UPDATE" => serde_json::from_str::<GuildScheduledEventUpdate>(raw.get()).map(GatewayEvent::GuildScheduledEventUpdate),
            "GUILD_SCHEDULED_EVENT_DELETE" => serde_json::from_str::<GuildScheduledEventDelete>(raw.get()).map(GatewayEvent::GuildScheduledEventDelete),
            "GUILD_SCHEDULED_EVENT_USER_ADD" => serde_json::from_str::<GuildScheduledEventUserAdd>(raw.get()).map(GatewayEvent::GuildScheduledEventUserAdd),
            "GUILD_SCHEDULED_EVENT_USER_REMOVE" => serde_json::from_str::<GuildScheduledEventUserRemove>(raw.get()).map(GatewayEvent::GuildScheduledEventUserRemove),

            "GUILD_SOUNDBOARD_SOUND_CREATE" => serde_json::from_str::<GuildSoundboardSoundCreate>(raw.get()).map(GatewayEvent::GuildSoundboardSoundCreate),
            "GUILD_SOUNDBOARD_SOUND_UPDATE" => serde_json::from_str::<GuildSoundboardSoundUpdate>(raw.get()).map(GatewayEvent::GuildSoundboardSoundUpdate),
            "GUILD_SOUNDBOARD_SOUND_DELETE" => serde_json::from_str::<GuildSoundboardSoundDelete>(raw.get()).map(GatewayEvent::GuildSoundboardSoundDelete),
            "GUILD_SOUNDBOARD_SOUNDS_UPDATE" => serde_json::from_str::<GuildSoundboardSoundsUpdate>(raw.get()).map(GatewayEvent::GuildSoundboardSoundsUpdate),

            "INTEGRATION_CREATE" => serde_json::from_str::<IntegrationCreate>(raw.get()).map(GatewayEvent::IntegrationCreate),
            "INTEGRATION_UPDATE" => serde_json::from_str::<IntegrationUpdate>(raw.get()).map(GatewayEvent::IntegrationUpdate),
            "INTEGRATION_DELETE" => serde_json::from_str::<IntegrationDelete>(raw.get()).map(GatewayEvent::IntegrationDelete),

            "INVITE_CREATE" => serde_json::from_str::<InviteCreate>(raw.get()).map(GatewayEvent::InviteCreate),
            "INVITE_DELETE" => serde_json::from_str::<InviteDelete>(raw.get()).map(GatewayEvent::InviteDelete),

            "MESSAGE_POLL_VOTE_ADD" => serde_json::from_str::<MessagePollVoteAdd>(raw.get()).map(GatewayEvent::MessagePollVoteAdd),
            "MESSAGE_POLL_VOTE_REMOVE" => serde_json::from_str::<MessagePollVoteRemove>(raw.get()).map(GatewayEvent::MessagePollVoteRemove),

            "STAGE_INSTANCE_CREATE" => serde_json::from_str::<StageInstanceCreate>(raw.get()).map(GatewayEvent::StageInstanceCreate),
            "STAGE_INSTANCE_UPDATE" => serde_json::from_str::<StageInstanceUpdate>(raw.get()).map(GatewayEvent::StageInstanceUpdate),
            "STAGE_INSTANCE_DELETE" => serde_json::from_str::<StageInstanceDelete>(raw.get()).map(GatewayEvent::StageInstanceDelete),

            "USER_UPDATE" => serde_json::from_str::<UserUpdate>(raw.get()).map(GatewayEvent::UserUpdate),
            "WEBHOOKS_UPDATE" => serde_json::from_str::<WebhooksUpdate>(raw.get()).map(GatewayEvent::WebhooksUpdate),

            _ => {
                let boxed = RawValue::from_string(raw.get().to_string())?;
                Ok(GatewayEvent::Unknown {
                    name: name.to_string(),
                    data: boxed,
                })
            }
        }
    }

    /// Parse a typed gateway event from its name and raw JSON string.
    pub fn parse_str(name: &str, json: &str) -> Result<Self, serde_json::Error> {
        let raw = RawValue::from_string(json.to_string())?;
        Self::parse(name, &raw)
    }

    /// Parse a typed gateway event consuming a boxed raw JSON value.
    pub fn parse_boxed(name: String, raw: Box<RawValue>) -> Result<Self, serde_json::Error> {
        match name.as_str() {
            "READY" => serde_json::from_str::<Ready>(raw.get()).map(GatewayEvent::Ready),
            "RESUMED" => serde_json::from_str::<Resumed>(raw.get()).map(GatewayEvent::Resumed),

            "GUILD_CREATE" => serde_json::from_str::<GuildCreate>(raw.get()).map(GatewayEvent::GuildCreate),
            "GUILD_UPDATE" => serde_json::from_str::<GuildUpdate>(raw.get()).map(GatewayEvent::GuildUpdate),
            "GUILD_DELETE" => serde_json::from_str::<GuildDelete>(raw.get()).map(GatewayEvent::GuildDelete),

            "CHANNEL_CREATE" => serde_json::from_str::<ChannelCreate>(raw.get()).map(GatewayEvent::ChannelCreate),
            "CHANNEL_UPDATE" => serde_json::from_str::<ChannelUpdate>(raw.get()).map(GatewayEvent::ChannelUpdate),
            "CHANNEL_DELETE" => serde_json::from_str::<ChannelDelete>(raw.get()).map(GatewayEvent::ChannelDelete),
            "CHANNEL_PINS_UPDATE" => serde_json::from_str::<ChannelPinsUpdate>(raw.get()).map(GatewayEvent::ChannelPinsUpdate),

            "THREAD_CREATE" => serde_json::from_str::<ThreadCreate>(raw.get()).map(GatewayEvent::ThreadCreate),
            "THREAD_UPDATE" => serde_json::from_str::<ThreadUpdate>(raw.get()).map(GatewayEvent::ThreadUpdate),
            "THREAD_DELETE" => serde_json::from_str::<ThreadDelete>(raw.get()).map(GatewayEvent::ThreadDelete),
            "THREAD_LIST_SYNC" => serde_json::from_str::<ThreadListSync>(raw.get()).map(GatewayEvent::ThreadListSync),
            "THREAD_MEMBER_UPDATE" => serde_json::from_str::<ThreadMemberUpdate>(raw.get()).map(GatewayEvent::ThreadMemberUpdate),
            "THREAD_MEMBERS_UPDATE" => serde_json::from_str::<ThreadMembersUpdate>(raw.get()).map(GatewayEvent::ThreadMembersUpdate),

            "GUILD_MEMBER_ADD" => serde_json::from_str::<GuildMemberAdd>(raw.get()).map(GatewayEvent::GuildMemberAdd),
            "GUILD_MEMBER_UPDATE" => serde_json::from_str::<GuildMemberUpdate>(raw.get()).map(GatewayEvent::GuildMemberUpdate),
            "GUILD_MEMBER_REMOVE" => serde_json::from_str::<GuildMemberRemove>(raw.get()).map(GatewayEvent::GuildMemberRemove),
            "GUILD_MEMBERS_CHUNK" => serde_json::from_str::<GuildMembersChunk>(raw.get()).map(GatewayEvent::GuildMembersChunk),

            "GUILD_ROLE_CREATE" => serde_json::from_str::<GuildRoleCreate>(raw.get()).map(GatewayEvent::GuildRoleCreate),
            "GUILD_ROLE_UPDATE" => serde_json::from_str::<GuildRoleUpdate>(raw.get()).map(GatewayEvent::GuildRoleUpdate),
            "GUILD_ROLE_DELETE" => serde_json::from_str::<GuildRoleDelete>(raw.get()).map(GatewayEvent::GuildRoleDelete),

            "MESSAGE_CREATE" => serde_json::from_str::<MessageCreate>(raw.get()).map(GatewayEvent::MessageCreate),
            "MESSAGE_UPDATE" => serde_json::from_str::<MessageUpdate>(raw.get()).map(GatewayEvent::MessageUpdate),
            "MESSAGE_DELETE" => serde_json::from_str::<MessageDelete>(raw.get()).map(GatewayEvent::MessageDelete),
            "MESSAGE_DELETE_BULK" => serde_json::from_str::<MessageDeleteBulk>(raw.get()).map(GatewayEvent::MessageDeleteBulk),

            "MESSAGE_REACTION_ADD" => serde_json::from_str::<MessageReactionAdd>(raw.get()).map(GatewayEvent::MessageReactionAdd),
            "MESSAGE_REACTION_REMOVE" => serde_json::from_str::<MessageReactionRemove>(raw.get()).map(GatewayEvent::MessageReactionRemove),
            "MESSAGE_REACTION_REMOVE_ALL" => serde_json::from_str::<MessageReactionRemoveAll>(raw.get()).map(GatewayEvent::MessageReactionRemoveAll),
            "MESSAGE_REACTION_REMOVE_EMOJI" => serde_json::from_str::<MessageReactionRemoveEmoji>(raw.get()).map(GatewayEvent::MessageReactionRemoveEmoji),

            "INTERACTION_CREATE" => serde_json::from_str::<InteractionCreate>(raw.get()).map(GatewayEvent::InteractionCreate),

            "VOICE_STATE_UPDATE" => serde_json::from_str::<VoiceStateUpdate>(raw.get()).map(GatewayEvent::VoiceStateUpdate),
            "VOICE_SERVER_UPDATE" => serde_json::from_str::<VoiceServerUpdate>(raw.get()).map(GatewayEvent::VoiceServerUpdate),
            "VOICE_CHANNEL_EFFECT_SEND" => serde_json::from_str::<VoiceChannelEffectSend>(raw.get()).map(GatewayEvent::VoiceChannelEffectSend),

            "PRESENCE_UPDATE" => serde_json::from_str::<PresenceUpdate>(raw.get()).map(GatewayEvent::PresenceUpdate),
            "TYPING_START" => serde_json::from_str::<TypingStart>(raw.get()).map(GatewayEvent::TypingStart),

            "AUTO_MODERATION_RULE_CREATE" => serde_json::from_str::<AutoModerationRuleCreate>(raw.get()).map(GatewayEvent::AutoModerationRuleCreate),
            "AUTO_MODERATION_RULE_UPDATE" => serde_json::from_str::<AutoModerationRuleUpdate>(raw.get()).map(GatewayEvent::AutoModerationRuleUpdate),
            "AUTO_MODERATION_RULE_DELETE" => serde_json::from_str::<AutoModerationRuleDelete>(raw.get()).map(GatewayEvent::AutoModerationRuleDelete),
            "AUTO_MODERATION_ACTION_EXECUTION" => serde_json::from_str::<AutoModerationActionExecution>(raw.get()).map(GatewayEvent::AutoModerationActionExecution),

            "ENTITLEMENT_CREATE" => serde_json::from_str::<EntitlementCreate>(raw.get()).map(GatewayEvent::EntitlementCreate),
            "ENTITLEMENT_UPDATE" => serde_json::from_str::<EntitlementUpdate>(raw.get()).map(GatewayEvent::EntitlementUpdate),
            "ENTITLEMENT_DELETE" => serde_json::from_str::<EntitlementDelete>(raw.get()).map(GatewayEvent::EntitlementDelete),

            "SUBSCRIPTION_CREATE" => serde_json::from_str::<SubscriptionCreate>(raw.get()).map(GatewayEvent::SubscriptionCreate),
            "SUBSCRIPTION_UPDATE" => serde_json::from_str::<SubscriptionUpdate>(raw.get()).map(GatewayEvent::SubscriptionUpdate),
            "SUBSCRIPTION_DELETE" => serde_json::from_str::<SubscriptionDelete>(raw.get()).map(GatewayEvent::SubscriptionDelete),

            "APPLICATION_COMMAND_PERMISSIONS_UPDATE" => serde_json::from_str::<ApplicationCommandPermissionsUpdate>(raw.get()).map(GatewayEvent::ApplicationCommandPermissionsUpdate),
            "GUILD_AUDIT_LOG_ENTRY_CREATE" => serde_json::from_str::<GuildAuditLogEntryCreate>(raw.get()).map(GatewayEvent::GuildAuditLogEntryCreate),
            "GUILD_BAN_ADD" => serde_json::from_str::<GuildBanAdd>(raw.get()).map(GatewayEvent::GuildBanAdd),
            "GUILD_BAN_REMOVE" => serde_json::from_str::<GuildBanRemove>(raw.get()).map(GatewayEvent::GuildBanRemove),
            "GUILD_EMOJIS_UPDATE" => serde_json::from_str::<GuildEmojisUpdate>(raw.get()).map(GatewayEvent::GuildEmojisUpdate),
            "GUILD_STICKERS_UPDATE" => serde_json::from_str::<GuildStickersUpdate>(raw.get()).map(GatewayEvent::GuildStickersUpdate),
            "GUILD_INTEGRATIONS_UPDATE" => serde_json::from_str::<GuildIntegrationsUpdate>(raw.get()).map(GatewayEvent::GuildIntegrationsUpdate),

            "GUILD_SCHEDULED_EVENT_CREATE" => serde_json::from_str::<GuildScheduledEventCreate>(raw.get()).map(GatewayEvent::GuildScheduledEventCreate),
            "GUILD_SCHEDULED_EVENT_UPDATE" => serde_json::from_str::<GuildScheduledEventUpdate>(raw.get()).map(GatewayEvent::GuildScheduledEventUpdate),
            "GUILD_SCHEDULED_EVENT_DELETE" => serde_json::from_str::<GuildScheduledEventDelete>(raw.get()).map(GatewayEvent::GuildScheduledEventDelete),
            "GUILD_SCHEDULED_EVENT_USER_ADD" => serde_json::from_str::<GuildScheduledEventUserAdd>(raw.get()).map(GatewayEvent::GuildScheduledEventUserAdd),
            "GUILD_SCHEDULED_EVENT_USER_REMOVE" => serde_json::from_str::<GuildScheduledEventUserRemove>(raw.get()).map(GatewayEvent::GuildScheduledEventUserRemove),

            "GUILD_SOUNDBOARD_SOUND_CREATE" => serde_json::from_str::<GuildSoundboardSoundCreate>(raw.get()).map(GatewayEvent::GuildSoundboardSoundCreate),
            "GUILD_SOUNDBOARD_SOUND_UPDATE" => serde_json::from_str::<GuildSoundboardSoundUpdate>(raw.get()).map(GatewayEvent::GuildSoundboardSoundUpdate),
            "GUILD_SOUNDBOARD_SOUND_DELETE" => serde_json::from_str::<GuildSoundboardSoundDelete>(raw.get()).map(GatewayEvent::GuildSoundboardSoundDelete),
            "GUILD_SOUNDBOARD_SOUNDS_UPDATE" => serde_json::from_str::<GuildSoundboardSoundsUpdate>(raw.get()).map(GatewayEvent::GuildSoundboardSoundsUpdate),

            "INTEGRATION_CREATE" => serde_json::from_str::<IntegrationCreate>(raw.get()).map(GatewayEvent::IntegrationCreate),
            "INTEGRATION_UPDATE" => serde_json::from_str::<IntegrationUpdate>(raw.get()).map(GatewayEvent::IntegrationUpdate),
            "INTEGRATION_DELETE" => serde_json::from_str::<IntegrationDelete>(raw.get()).map(GatewayEvent::IntegrationDelete),

            "INVITE_CREATE" => serde_json::from_str::<InviteCreate>(raw.get()).map(GatewayEvent::InviteCreate),
            "INVITE_DELETE" => serde_json::from_str::<InviteDelete>(raw.get()).map(GatewayEvent::InviteDelete),

            "MESSAGE_POLL_VOTE_ADD" => serde_json::from_str::<MessagePollVoteAdd>(raw.get()).map(GatewayEvent::MessagePollVoteAdd),
            "MESSAGE_POLL_VOTE_REMOVE" => serde_json::from_str::<MessagePollVoteRemove>(raw.get()).map(GatewayEvent::MessagePollVoteRemove),

            "STAGE_INSTANCE_CREATE" => serde_json::from_str::<StageInstanceCreate>(raw.get()).map(GatewayEvent::StageInstanceCreate),
            "STAGE_INSTANCE_UPDATE" => serde_json::from_str::<StageInstanceUpdate>(raw.get()).map(GatewayEvent::StageInstanceUpdate),
            "STAGE_INSTANCE_DELETE" => serde_json::from_str::<StageInstanceDelete>(raw.get()).map(GatewayEvent::StageInstanceDelete),

            "USER_UPDATE" => serde_json::from_str::<UserUpdate>(raw.get()).map(GatewayEvent::UserUpdate),
            "WEBHOOKS_UPDATE" => serde_json::from_str::<WebhooksUpdate>(raw.get()).map(GatewayEvent::WebhooksUpdate),

            _ => Ok(GatewayEvent::Unknown {
                name,
                data: raw,
            }),
        }
    }
}

impl PartialEq for GatewayEvent {
    fn eq(&self, other: &Self) -> bool {
        macro_rules! match_eq {
            ($($variant:ident),* $(,)?) => {
                match (self, other) {
                    $((GatewayEvent::$variant(a), GatewayEvent::$variant(b)) => a == b,)*
                    (
                        GatewayEvent::Unknown { name: n1, data: d1 },
                        GatewayEvent::Unknown { name: n2, data: d2 },
                    ) => n1 == n2 && d1.get() == d2.get(),
                    _ => false,
                }
            };
        }

        match_eq!(
            Ready,
            Resumed,
            GuildCreate,
            GuildUpdate,
            GuildDelete,
            ChannelCreate,
            ChannelUpdate,
            ChannelDelete,
            ChannelPinsUpdate,
            ThreadCreate,
            ThreadUpdate,
            ThreadDelete,
            ThreadListSync,
            ThreadMemberUpdate,
            ThreadMembersUpdate,
            GuildMemberAdd,
            GuildMemberUpdate,
            GuildMemberRemove,
            GuildMembersChunk,
            GuildRoleCreate,
            GuildRoleUpdate,
            GuildRoleDelete,
            MessageCreate,
            MessageUpdate,
            MessageDelete,
            MessageDeleteBulk,
            MessageReactionAdd,
            MessageReactionRemove,
            MessageReactionRemoveAll,
            MessageReactionRemoveEmoji,
            InteractionCreate,
            VoiceStateUpdate,
            VoiceServerUpdate,
            VoiceChannelEffectSend,
            PresenceUpdate,
            TypingStart,
            AutoModerationRuleCreate,
            AutoModerationRuleUpdate,
            AutoModerationRuleDelete,
            AutoModerationActionExecution,
            EntitlementCreate,
            EntitlementUpdate,
            EntitlementDelete,
            SubscriptionCreate,
            SubscriptionUpdate,
            SubscriptionDelete,
            ApplicationCommandPermissionsUpdate,
            GuildAuditLogEntryCreate,
            GuildBanAdd,
            GuildBanRemove,
            GuildEmojisUpdate,
            GuildStickersUpdate,
            GuildIntegrationsUpdate,
            GuildScheduledEventCreate,
            GuildScheduledEventUpdate,
            GuildScheduledEventDelete,
            GuildScheduledEventUserAdd,
            GuildScheduledEventUserRemove,
            GuildSoundboardSoundCreate,
            GuildSoundboardSoundUpdate,
            GuildSoundboardSoundDelete,
            GuildSoundboardSoundsUpdate,
            IntegrationCreate,
            IntegrationUpdate,
            IntegrationDelete,
            InviteCreate,
            InviteDelete,
            MessagePollVoteAdd,
            MessagePollVoteRemove,
            StageInstanceCreate,
            StageInstanceUpdate,
            StageInstanceDelete,
            UserUpdate,
            WebhooksUpdate,
        )
    }
}
