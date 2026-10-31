//! Voice models and gateway voice payloads.

use crate::guild::GuildMember;
use crate::message::ReactionEmoji;
use crate::serde_util::{
    de_opt_snowflake, de_snowflake, ser_opt_snowflake, ser_snowflake,
};
use rift_core::Snowflake;
use serde::{Deserialize, Serialize};

/// A user's voice connection status.
///
/// See <https://discord.com/developers/docs/resources/voice#voice-state-object>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VoiceState {
    /// The guild id this voice state is for.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,

    /// The channel id this user is connected to.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub channel_id: Option<Snowflake>,

    /// The user id this voice state is for.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub user_id: Snowflake,

    /// The guild member this voice state is for.
    #[serde(default)]
    pub member: Option<GuildMember>,

    /// The session id for this voice state.
    pub session_id: String,

    /// Whether this user is deafened by the server.
    #[serde(default)]
    pub deaf: bool,

    /// Whether this user is muted by the server.
    #[serde(default)]
    pub mute: bool,

    /// Whether this user is locally deafened.
    #[serde(default)]
    pub self_deaf: bool,

    /// Whether this user is locally muted.
    #[serde(default)]
    pub self_mute: bool,

    /// Whether this user is streaming using "Go Live".
    #[serde(default)]
    pub self_stream: Option<bool>,

    /// Whether this user's camera is enabled.
    #[serde(default)]
    pub self_video: bool,

    /// Whether this user is muted by the current user.
    #[serde(default)]
    pub suppress: bool,

    /// The time at which the user requested to speak.
    #[serde(default)]
    pub request_to_speak_timestamp: Option<String>,
}

/// Dispatched when a guild's voice server is updated.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#voice-server-update>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceServerUpdate {
    /// Voice connection token.
    pub token: String,

    /// The guild this voice server update is for.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// The voice server host.
    #[serde(default)]
    pub endpoint: Option<String>,
}

/// Dispatched when a voice channel effect is sent.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#voice-channel-effect-send>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VoiceChannelEffect {
    /// ID of the channel.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub channel_id: Snowflake,

    /// ID of the guild.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// ID of the user that sent the effect.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub user_id: Snowflake,

    /// The emoji sent, for emoji reaction and animation types.
    #[serde(default)]
    pub emoji: Option<ReactionEmoji>,

    /// The type of voice channel effect animation.
    #[serde(default)]
    pub animation_type: Option<u8>,

    /// ID of the animation, for animation types.
    #[serde(default)]
    pub animation_id: Option<u32>,

    /// ID of the soundboard sound, for sound types.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub sound_id: Option<Snowflake>,

    /// The volume of the soundboard sound, from 0 to 1.
    #[serde(default)]
    pub sound_volume: Option<f64>,
}
