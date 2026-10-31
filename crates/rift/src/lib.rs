//! # rift
//!
//! High-performance Discord library: one client that runs REST + sharded gateway
//! on a Rust core, with full permission math, obfuscation-safe caching, and an
//! ergonomic command & interaction routing framework.
//!
//! ## Quickstart: Low-Level Event Loop
//!
//! ```no_run
//! use rift::{Client, Intents};
//!
//! # async fn demo() -> Result<(), rift::Error> {
//! let mut client = Client::builder("TOKEN", Intents::GUILDS | Intents::GUILD_MESSAGES)
//!     .build()
//!     .await?;
//! while let Some(event) = client.next_event().await {
//!     println!("[shard {}] {}", event.shard_id, event.name);
//! }
//! # Ok(()) }
//! ```
//!
//! ## Quickstart: High-Level Framework with Prelude
//!
//! ```no_run
//! # #[cfg(feature = "framework")]
//! # async fn demo() -> Result<(), rift::Error> {
//! use rift::prelude::*;
//!
//! let router = CommandRouter::new()
//!     .slash("ping", "Check bot responsiveness", |ctx| async move {
//!         ctx.reply("Pong!").await?;
//!         Ok(())
//!     })
//!     .component_prefix("btn:", |ctx| async move {
//!         ctx.reply_ephemeral("Button clicked!").await?;
//!         Ok(())
//!     });
//!
//! let client = Client::builder("TOKEN", Intents::GUILDS).build().await?;
//! let mut framework = Framework::new(client, router);
//! framework.register_global_commands(Snowflake::new(123456789)).await?;
//! framework.run().await.unwrap();
//! # Ok(()) }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod client;
mod error;
#[cfg(feature = "framework")]
pub mod framework;
pub mod prelude;
mod supervisor;

pub use client::{Client, ClientBuilder};
pub use error::Error;

/// Re-export of `tracing` for structured diagnostics and event logging.
pub use tracing;

#[cfg(feature = "framework")]
pub use framework::{
    CommandHandler, CommandRouter, EventContext, EventModelExt, Framework,
    FrameworkError, HandlerResult,
};

#[cfg(all(feature = "framework", feature = "cache"))]
pub use framework::CachedClient;

#[cfg(feature = "cache")]
pub use rift_cache as cache;
pub use rift_core as core;
pub use rift_http as http;
pub use rift_model as model;
pub use rift_rest as rest;
pub use rift_runtime as runtime;
#[cfg(feature = "voice")]
pub use rift_voice as voice;

#[cfg(feature = "cache")]
pub use rift_cache::{Cache, CacheConfig, CacheStats};
pub use rift_core::{Capabilities, Intents, MessageFlags, Opcode, Overwrite, Permissions, Snowflake};
pub use rift_http::verify;
pub use rift_http::{
    verify_interaction, verify_interaction_custom, verify_interaction_raw,
    verify_interaction_with_timestamp, Ed25519Verifier, Verifier, VerifyError, HEADER_SIGNATURE,
    HEADER_TIMESTAMP,
};
pub use rift_model::GatewayEvent;
pub use rift_rest::{
    AllowedMentions, ArchivedThreadsPage, Channel, ChannelError, ChannelType, Choice, CommandData,
    CommandError, CommandOption, CommandSpec, CommandType, ComponentData, CreateInvite,
    CreateMessage, CreateThread, Embed, FollowedChannel, ForumTag, GatewayBot, GetMessages,
    IntegrationType, Interaction, InteractionContext, InteractionMessage, InteractionResponse,
    InteractionType, Invite, Member, MentionType, Message, MessageReference, ModifyGuildChannel,
    ModifyThread, OptionSpec, OptionType, PermissionOverwrite, RegisteredCommand, RestClient, User,
    ValidationError,
};
pub use rift_runtime::command;
pub use rift_runtime::{
    event_channel, Activity, ActivityType, Command, CommandError as GatewayCommandError, Event,
    EventBufferPolicy, EventReceiver, EventSendError, EventSender, RequestGuildMembers,
    RequestSoundboardSounds, Status, UpdatePresence, UpdateVoiceState,
};
