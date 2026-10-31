//! # rift-rest
//!
//! Async REST client. All pacing decisions come from the sans-IO
//! [`rift_http::RateLimiter`]; all network access goes through the
//! [`Transport`] trait, so the retry/429/circuit-breaker logic is tested
//! without a network.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod application;
mod channel;
mod client;
mod command;
pub mod component;
pub use component::*;
mod error;
mod gateway;
mod guild;
mod interaction;
mod invite;
pub mod lobby;
pub use lobby::*;
mod message;
mod misc;
mod multipart;
mod oauth;
mod stage;
mod sticker;
#[cfg(test)]
mod testing;
mod transport;
mod user;
mod webhook;

pub use application::*;
pub use oauth::*;
pub use webhook::*;

pub use invite::*;
pub use misc::{Gateway, GetSkuSubscriptions, MiscError, Subscription, SubscriptionStatus};
pub use stage::*;
pub use sticker::*;
pub use user::{
    ApplicationRoleConnection, CurrentUser, CurrentUserGuild, CurrentUserGuildMember, Entitlement,
    GetCurrentUserEntitlements, GetCurrentUserGuilds, ModifyCurrentUser,
    UpdateApplicationRoleConnection, UserConnection, UserError, UserResponse,
};

pub use client::{user_agent, RestClient, Response, API_BASE};
pub use command::{
    Choice, CommandError, CommandSpec, CommandType, IntegrationType, InteractionContext, OptionSpec,
    OptionType, RegisteredCommand, validate_commands,
};
pub use channel::{
    flags as channel_flags, flags, ArchivedThreadsPage, Channel, ChannelError, ChannelType,
    CreateForumThread, CreateInvite, CreateThread, FollowedChannel, ForumTag, GetMessages, Invite,
    ListThreadMembers, ModifyGuildChannel, ModifyThread, PermissionOverwrite, ThreadMetadata,
};
pub use error::Error;
pub use gateway::{GatewayBot, SessionStartLimit};
pub use guild::{
    ActionGuildJoinRequest, ActiveThreads, AddGuildMember, AuditLog, AuditLogChange, AuditLogEntry,
    AutoModerationRule, Ban, BeginPrune, BulkBanResponse, BulkBanUsers, CreateAutoModerationRule,
    CreateGuildBan, CreateGuildChannel, CreateGuildEmoji, CreateGuildRole,
    CreateGuildScheduledEvent, CreateGuildScheduledEventException, CreateGuildSticker,
    CreateGuildTemplate, CreateSoundboardSound, Emoji, GetAuditLog, Guild, GuildError,
    GuildIncidentsData, GuildJoinRequest, GuildJoinRequestsPage, GuildMember, GuildOnboarding,
    GuildPreview, GuildScheduledEvent, GuildScheduledEventException, GuildScheduledEventUser,
    GuildScheduledEventUserCount, GuildSticker, GuildTemplate, Integration, IntegrationAccount,
    ListGuildMembers, ModifyAutoModerationRule, ModifyCurrentMember, ModifyCurrentVoiceState,
    ModifyGuild, ModifyGuildChannelPosition, ModifyGuildEmoji, ModifyGuildMember, ModifyGuildRole,
    ModifyGuildRolePosition, ModifyGuildScheduledEvent, ModifyGuildScheduledEventException,
    ModifyGuildSticker, ModifyGuildTemplate, ModifyVoiceState, ModifyWelcomeScreen,
    ModifyWidgetSettings, PruneCount, Role, SearchGuildMembers, SoundboardSoundsPage,
    ThreadMember, VanityUrl, VoiceState, WelcomeScreen, WelcomeScreenChannel,
    Widget, WidgetChannel, WidgetMember, WidgetSettings,
};
pub use interaction::{
    CommandData, CommandOption, ComponentData, Interaction, InteractionMessage, InteractionResponse,
    InteractionType, Member, ModalSubmitComponent, ModalSubmitData, ModalSubmitRow, User,
};
pub use message::{
    AllowedMentions, Attachment, CreateMessage, EditMessage, Embed, EmbedAuthor, EmbedField,
    EmbedFooter, EmbedImage, MentionType, Message, MessageReference, ValidationError,
};
pub use multipart::UploadFile;
pub use transport::{Multipart, Part, RawResponse, ReqwestTransport, Request, Transport};
