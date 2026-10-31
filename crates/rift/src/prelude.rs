//! The Rift prelude.
//!
//! Convenient re-exports of common types and traits across the entire Rift ecosystem:
//!
//! ```no_run
//! use rift::prelude::*;
//! ```

// --- rift & framework ---
pub use crate::client::{Client, ClientBuilder};
pub use crate::error::Error;
#[cfg(feature = "framework")]
pub use crate::framework::{
    CommandHandler, CommandRouter, EventContext, EventModelExt, Framework,
    FrameworkError, HandlerResult,
};
#[cfg(all(feature = "framework", feature = "cache"))]
pub use crate::framework::CachedClient;

// --- rift-core ---
pub use rift_core::{
    Capabilities, Intents, MessageFlags, Opcode, Overwrite, Permissions, Snowflake,
};

// --- rift-cache ---
#[cfg(feature = "cache")]
pub use rift_cache::{Cache, CacheConfig, CacheStats};

// --- rift-voice ---
#[cfg(feature = "voice")]
pub use rift_voice as voice;

// --- rift-runtime ---
pub use rift_runtime::{
    Activity, ActivityType, Command, Event, EventBufferPolicy, RequestGuildMembers,
    RequestSoundboardSounds, Status, UpdatePresence, UpdateVoiceState,
};

// --- rift-rest ---
pub use rift_rest::component::{
    ActionRow, Button, ButtonStyle, Component, ComponentEmoji, Container, MediaGallery,
    MediaGalleryItem, Section, SelectDefaultValue, SelectMenu, SelectOption, Separator,
    SeparatorSpacing, TextDisplay, TextInput, TextInputStyle,
};
pub use rift_rest::{
    AllowedMentions, Channel, Choice, CommandData, CommandOption, CommandSpec, CommandType,
    ComponentData, CreateMessage, Embed, FollowedChannel, ForumTag, IntegrationType,
    Interaction, InteractionContext, InteractionMessage, InteractionResponse, InteractionType,
    Invite, Member, Message, MessageReference, ModalSubmitComponent, ModalSubmitData,
    ModalSubmitRow, OptionSpec, OptionType, RegisteredCommand, RestClient, User,
};

// --- rift-model ---
pub use rift_model::{
    AutoModerationActionExecution, GatewayEvent, PresenceUpdate, VoiceServerUpdate,
};
pub use rift_model::events::{
    ApplicationCommandPermissionsUpdate, AutoModerationRuleCreate, AutoModerationRuleDelete,
    AutoModerationRuleUpdate, ChannelCreate, ChannelDelete, ChannelPinsUpdate, ChannelUpdate,
    EntitlementCreate, EntitlementDelete, EntitlementUpdate, GuildAuditLogEntryCreate,
    GuildBanAdd, GuildBanRemove, GuildCreate, GuildDelete, GuildEmojisUpdate,
    GuildIntegrationsUpdate, GuildMemberAdd, GuildMemberRemove, GuildMemberUpdate,
    GuildMembersChunk, GuildRoleCreate, GuildRoleDelete, GuildRoleUpdate,
    GuildScheduledEventCreate, GuildScheduledEventDelete, GuildScheduledEventUpdate,
    GuildScheduledEventUserAdd, GuildScheduledEventUserRemove, GuildSoundboardSoundCreate,
    GuildSoundboardSoundDelete, GuildSoundboardSoundUpdate, GuildSoundboardSoundsUpdate,
    GuildStickersUpdate, GuildUpdate, IntegrationCreate, IntegrationDelete, IntegrationUpdate,
    InteractionCreate, InviteCreate, InviteDelete, MessageCreate, MessageDelete,
    MessageDeleteBulk, MessagePollVoteAdd, MessagePollVoteRemove, MessageReactionAdd,
    MessageReactionRemove, MessageReactionRemoveAll, MessageReactionRemoveEmoji, MessageUpdate,
    Ready, Resumed, StageInstanceCreate, StageInstanceDelete, StageInstanceUpdate,
    SubscriptionCreate, SubscriptionDelete, SubscriptionUpdate, ThreadCreate, ThreadDelete,
    ThreadListSync, ThreadMemberUpdate, ThreadMembersUpdate, ThreadUpdate, TypingStart,
    UserUpdate, VoiceChannelEffectSend, VoiceStateUpdate, WebhooksUpdate,
};
pub use rift_model::guild::{
    AuditLogEntry, Emoji, Guild, GuildMember, Role, SoundboardSound, StageInstance, Sticker,
    UnavailableGuild,
};
pub use rift_model::user::PresenceUser;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prelude_glob_imports_all_essential_types() {
        // Core
        let _sf = Snowflake::new(1);
        let _perm = Permissions::SEND_MESSAGES;
        let _intents = Intents::GUILDS;

        // Framework & Client
        let _b = Client::builder("tok", Intents::GUILDS);
        #[cfg(feature = "framework")]
        let _router = CommandRouter::new();

        // Model events
        let _gw = GatewayEvent::Resumed(Resumed { _trace: None });

        // Cache
        #[cfg(feature = "cache")]
        let _cache = Cache::new(CacheConfig::default());

        // Rest
        let _cmd = CommandSpec::slash("hello", "Say hello");
        let _btn = Button::primary("btn_1", "Click me");
    }
}
