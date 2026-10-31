//! Gateway command payloads (client to gateway).
//!
//! Provides typed definitions for Discord gateway commands:
//! - Opcode 3: [`UpdatePresence`]
//! - Opcode 4: [`UpdateVoiceState`]
//! - Opcode 8: [`RequestGuildMembers`]
//! - Opcode 31: [`RequestSoundboardSounds`]

use rift_core::Snowflake;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// Errors when building or validating gateway commands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandError {
    /// Neither `query` nor `user_ids` was provided for [`RequestGuildMembers`].
    MissingQueryOrUserIds,
    /// Both `query` and `user_ids` were provided for [`RequestGuildMembers`].
    ConflictingQueryAndUserIds,
    /// `nonce` exceeds the maximum allowed length of 32 bytes.
    NonceTooLong {
        /// Actual length of the nonce string in bytes.
        len: usize,
    },
    /// JSON serialization failed.
    Serialization(String),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingQueryOrUserIds => {
                write!(f, "RequestGuildMembers requires either `query` or `user_ids` to be set")
            }
            Self::ConflictingQueryAndUserIds => {
                write!(f, "RequestGuildMembers cannot specify both `query` and `user_ids`")
            }
            Self::NonceTooLong { len } => {
                write!(f, "RequestGuildMembers nonce exceeds 32 bytes (got {len} bytes)")
            }
            Self::Serialization(e) => write!(f, "command serialization error: {e}"),
        }
    }
}

impl std::error::Error for CommandError {}

impl From<serde_json::Error> for CommandError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serialization(e.to_string())
    }
}

fn ser_snowflake<S: Serializer>(v: &Snowflake, s: S) -> Result<S::Ok, S::Error> {
    s.collect_str(v)
}

fn ser_opt_snowflake<S: Serializer>(v: &Option<Snowflake>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(x) => s.collect_str(x),
        None => s.serialize_none(),
    }
}

fn ser_snowflakes<S: Serializer>(v: &[Snowflake], s: S) -> Result<S::Ok, S::Error> {
    s.collect_seq(v.iter().map(|x| x.to_string()))
}

fn ser_opt_snowflakes<S: Serializer>(v: &Option<Vec<Snowflake>>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(list) => s.collect_seq(list.iter().map(|x| x.to_string())),
        None => s.serialize_none(),
    }
}

fn de_snowflake<'de, D: Deserializer<'de>>(d: D) -> Result<Snowflake, D::Error> {
    let s = String::deserialize(d)?;
    s.parse::<Snowflake>().map_err(serde::de::Error::custom)
}

fn de_opt_snowflake<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Snowflake>, D::Error> {
    let opt = Option::<String>::deserialize(d)?;
    match opt {
        Some(s) => s.parse::<Snowflake>().map(Some).map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

fn de_snowflakes<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Snowflake>, D::Error> {
    let list = Vec::<String>::deserialize(d)?;
    list.into_iter()
        .map(|s| s.parse::<Snowflake>().map_err(serde::de::Error::custom))
        .collect()
}

fn de_opt_snowflakes<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<Snowflake>>, D::Error> {
    let opt = Option::<Vec<String>>::deserialize(d)?;
    match opt {
        Some(list) => list
            .into_iter()
            .map(|s| s.parse::<Snowflake>().map_err(serde::de::Error::custom))
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
        None => Ok(None),
    }
}

// ----------------------------------------------------------------------------
// Opcode 3: Update Presence
// ----------------------------------------------------------------------------

/// Client presence status string.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Online.
    Online,
    /// Do Not Disturb.
    Dnd,
    /// Idle.
    Idle,
    /// Invisible (appears offline to others).
    Invisible,
    /// Offline.
    Offline,
}

/// Type of activity being performed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ActivityType {
    /// Playing a game (0).
    Playing = 0,
    /// Streaming (1).
    Streaming = 1,
    /// Listening to audio (2).
    Listening = 2,
    /// Watching media (3).
    Watching = 3,
    /// Custom status (4).
    Custom = 4,
    /// Competing in an event (5).
    Competing = 5,
}

impl Serialize for ActivityType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u8(*self as u8)
    }
}

impl<'de> Deserialize<'de> for ActivityType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let val = u8::deserialize(deserializer)?;
        match val {
            0 => Ok(Self::Playing),
            1 => Ok(Self::Streaming),
            2 => Ok(Self::Listening),
            3 => Ok(Self::Watching),
            4 => Ok(Self::Custom),
            5 => Ok(Self::Competing),
            other => Err(serde::de::Error::custom(format!("invalid activity type: {other}"))),
        }
    }
}

/// One activity item in a presence update.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    /// Activity name.
    pub name: String,
    /// Activity type.
    #[serde(rename = "type")]
    pub kind: ActivityType,
    /// Stream URL (required/validated when `kind == ActivityType::Streaming`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// State or custom status text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

impl Activity {
    /// "Playing {name}" activity.
    pub fn playing(name: impl Into<String>) -> Self {
        Self { name: name.into(), kind: ActivityType::Playing, url: None, state: None }
    }

    /// "Streaming {name}" activity with a stream URL.
    pub fn streaming(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self { name: name.into(), kind: ActivityType::Streaming, url: Some(url.into()), state: None }
    }

    /// "Listening to {name}" activity.
    pub fn listening(name: impl Into<String>) -> Self {
        Self { name: name.into(), kind: ActivityType::Listening, url: None, state: None }
    }

    /// "Watching {name}" activity.
    pub fn watching(name: impl Into<String>) -> Self {
        Self { name: name.into(), kind: ActivityType::Watching, url: None, state: None }
    }

    /// Custom status message.
    pub fn custom(state: impl Into<String>) -> Self {
        Self { name: "Custom Status".to_owned(), kind: ActivityType::Custom, url: None, state: Some(state.into()) }
    }

    /// "Competing in {name}" activity.
    pub fn competing(name: impl Into<String>) -> Self {
        Self { name: name.into(), kind: ActivityType::Competing, url: None, state: None }
    }
}

/// Opcode 3: Update Presence command payload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdatePresence {
    /// Unix time (in milliseconds) of when the client went idle, or `None` if active.
    pub since: Option<u64>,
    /// Active activities.
    pub activities: Vec<Activity>,
    /// User status.
    pub status: Status,
    /// Whether or not the client is AFK.
    pub afk: bool,
}

impl UpdatePresence {
    /// Creates a presence update with the specified status and empty activities.
    pub fn new(status: Status) -> Self {
        Self { since: None, activities: Vec::new(), status, afk: false }
    }

    /// Convenience constructor for Online status.
    pub fn online() -> Self {
        Self::new(Status::Online)
    }

    /// Convenience constructor for Do Not Disturb status.
    pub fn dnd() -> Self {
        Self::new(Status::Dnd)
    }

    /// Convenience constructor for Idle status.
    pub fn idle() -> Self {
        Self::new(Status::Idle)
    }

    /// Convenience constructor for Invisible status.
    pub fn invisible() -> Self {
        Self::new(Status::Invisible)
    }

    /// Adds an activity.
    pub fn with_activity(mut self, activity: Activity) -> Self {
        self.activities.push(activity);
        self
    }

    /// Sets the activities list.
    pub fn with_activities(mut self, activities: Vec<Activity>) -> Self {
        self.activities = activities;
        self
    }

    /// Sets the AFK flag.
    pub fn with_afk(mut self, afk: bool) -> Self {
        self.afk = afk;
        self
    }

    /// Sets the `since` idle timestamp (Unix milliseconds).
    pub fn with_since(mut self, since: u64) -> Self {
        self.since = Some(since);
        self
    }

    /// Encodes this command as a gateway JSON payload envelope (`{"op": 3, "d": ...}`).
    pub fn to_payload(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&serde_json::json!({
            "op": 3,
            "d": self,
        }))
    }
}

// ----------------------------------------------------------------------------
// Opcode 4: Update Voice State
// ----------------------------------------------------------------------------

/// Opcode 4: Update Voice State command payload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateVoiceState {
    /// Guild id to join or leave.
    #[serde(serialize_with = "ser_snowflake", deserialize_with = "de_snowflake")]
    pub guild_id: Snowflake,
    /// Channel id to join, or `None` to disconnect.
    #[serde(serialize_with = "ser_opt_snowflake", deserialize_with = "de_opt_snowflake")]
    pub channel_id: Option<Snowflake>,
    /// Whether the client is muted.
    pub self_mute: bool,
    /// Whether the client is deafened.
    pub self_deaf: bool,
}

impl UpdateVoiceState {
    /// Connect to or move to a voice channel.
    pub fn join(guild_id: Snowflake, channel_id: Snowflake, self_mute: bool, self_deaf: bool) -> Self {
        Self { guild_id, channel_id: Some(channel_id), self_mute, self_deaf }
    }

    /// Disconnect from voice in the specified guild.
    pub fn leave(guild_id: Snowflake) -> Self {
        Self { guild_id, channel_id: None, self_mute: false, self_deaf: false }
    }

    /// Encodes this command as a gateway JSON payload envelope (`{"op": 4, "d": ...}`).
    pub fn to_payload(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&serde_json::json!({
            "op": 4,
            "d": self,
        }))
    }
}

// ----------------------------------------------------------------------------
// Opcode 8: Request Guild Members
// ----------------------------------------------------------------------------

/// Opcode 8: Request Guild Members command payload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestGuildMembers {
    /// Guild id to fetch members for.
    #[serde(serialize_with = "ser_snowflake", deserialize_with = "de_snowflake")]
    pub guild_id: Snowflake,
    /// String that username starts with, or empty string to match all (mutually exclusive with `user_ids`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// Maximum number of members to send matching the query (0 to match all).
    pub limit: u32,
    /// Whether to include member presences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presences: Option<bool>,
    /// Specific user ids to fetch (mutually exclusive with `query`).
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_opt_snowflakes",
        deserialize_with = "de_opt_snowflakes"
    )]
    pub user_ids: Option<Vec<Snowflake>>,
    /// Nonce string to identify the Guild Members Chunk response (max 32 bytes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
}

impl RequestGuildMembers {
    /// Request guild members by username prefix `query` (limit 0 matches all).
    pub fn by_query(guild_id: Snowflake, query: impl Into<String>, limit: u32) -> Self {
        Self {
            guild_id,
            query: Some(query.into()),
            limit,
            presences: None,
            user_ids: None,
            nonce: None,
        }
    }

    /// Request specific guild members by user id.
    pub fn by_user_ids(guild_id: Snowflake, user_ids: Vec<Snowflake>) -> Self {
        Self {
            guild_id,
            query: None,
            limit: 0,
            presences: None,
            user_ids: Some(user_ids),
            nonce: None,
        }
    }

    /// Request presences for fetched members.
    pub fn with_presences(mut self, presences: bool) -> Self {
        self.presences = Some(presences);
        self
    }

    /// Attach a nonce to identify chunk responses (max 32 bytes).
    pub fn with_nonce(mut self, nonce: impl Into<String>) -> Self {
        self.nonce = Some(nonce.into());
        self
    }

    /// Validates that either `query` or `user_ids` is set (not both, not neither),
    /// and that `nonce` does not exceed 32 bytes.
    pub fn validate(&self) -> Result<(), CommandError> {
        match (&self.query, &self.user_ids) {
            (None, None) => return Err(CommandError::MissingQueryOrUserIds),
            (Some(_), Some(_)) => return Err(CommandError::ConflictingQueryAndUserIds),
            _ => {}
        }

        if let Some(ref nonce) = self.nonce {
            if nonce.len() > 32 {
                return Err(CommandError::NonceTooLong { len: nonce.len() });
            }
        }

        Ok(())
    }

    /// Validates and encodes this command as a gateway JSON payload envelope (`{"op": 8, "d": ...}`).
    pub fn to_payload(&self) -> Result<String, CommandError> {
        self.validate()?;
        Ok(serde_json::to_string(&serde_json::json!({
            "op": 8,
            "d": self,
        }))?)
    }
}

// ----------------------------------------------------------------------------
// Opcode 31: Request Soundboard Sounds
// ----------------------------------------------------------------------------

/// Opcode 31: Request Soundboard Sounds command payload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestSoundboardSounds {
    /// Guild ids to request soundboard sounds for.
    #[serde(serialize_with = "ser_snowflakes", deserialize_with = "de_snowflakes")]
    pub guild_ids: Vec<Snowflake>,
}

impl RequestSoundboardSounds {
    /// Create a request for soundboard sounds from specified guilds.
    pub fn new(guild_ids: Vec<Snowflake>) -> Self {
        Self { guild_ids }
    }

    /// Encodes this command as a gateway JSON payload envelope (`{"op": 31, "d": ...}`).
    pub fn to_payload(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&serde_json::json!({
            "op": 31,
            "d": self,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_presence_payload() {
        let presence = UpdatePresence::online()
            .with_activity(Activity::playing("Rust"))
            .with_afk(false);
        let raw = presence.to_payload().unwrap();
        let val: serde_json::Value = serde_json::from_str(&raw).unwrap();

        assert_eq!(val["op"], 3);
        assert_eq!(val["d"]["status"], "online");
        assert_eq!(val["d"]["afk"], false);
        assert_eq!(val["d"]["activities"][0]["name"], "Rust");
        assert_eq!(val["d"]["activities"][0]["type"], 0);
    }

    #[test]
    fn activity_types_match_discord_specification() {
        assert_eq!(Activity::playing("Game").kind as u8, 0);
        assert_eq!(Activity::streaming("Twitch", "https://twitch.tv").kind as u8, 1);
        assert_eq!(Activity::listening("Music").kind as u8, 2);
        assert_eq!(Activity::watching("Movie").kind as u8, 3);
        assert_eq!(Activity::custom("Status").kind as u8, 4);
        assert_eq!(Activity::competing("Tournament").kind as u8, 5);
    }

    #[test]
    fn update_voice_state_join_and_leave() {
        let gid = Snowflake::new(100);
        let cid = Snowflake::new(200);

        let join = UpdateVoiceState::join(gid, cid, false, true);
        let raw_join = join.to_payload().unwrap();
        let val_join: serde_json::Value = serde_json::from_str(&raw_join).unwrap();
        assert_eq!(val_join["op"], 4);
        assert_eq!(val_join["d"]["guild_id"], "100");
        assert_eq!(val_join["d"]["channel_id"], "200");
        assert_eq!(val_join["d"]["self_mute"], false);
        assert_eq!(val_join["d"]["self_deaf"], true);

        let leave = UpdateVoiceState::leave(gid);
        let raw_leave = leave.to_payload().unwrap();
        let val_leave: serde_json::Value = serde_json::from_str(&raw_leave).unwrap();
        assert_eq!(val_leave["op"], 4);
        assert_eq!(val_leave["d"]["guild_id"], "100");
        assert!(val_leave["d"]["channel_id"].is_null());
    }

    #[test]
    fn request_guild_members_validation() {
        let gid = Snowflake::new(100);

        // Valid by query
        let q = RequestGuildMembers::by_query(gid, "alice", 10).with_nonce("req-1");
        assert!(q.validate().is_ok());
        let payload = q.to_payload().unwrap();
        assert!(payload.contains(r#""query":"alice""#));

        // Valid by user_ids
        let u = RequestGuildMembers::by_user_ids(gid, vec![Snowflake::new(200), Snowflake::new(201)]);
        assert!(u.validate().is_ok());

        // Invalid: missing both query and user_ids
        let mut invalid_none = q.clone();
        invalid_none.query = None;
        assert_eq!(invalid_none.validate(), Err(CommandError::MissingQueryOrUserIds));

        // Invalid: both query and user_ids provided
        let mut invalid_both = q.clone();
        invalid_both.user_ids = Some(vec![Snowflake::new(200)]);
        assert_eq!(invalid_both.validate(), Err(CommandError::ConflictingQueryAndUserIds));

        // Invalid: nonce > 32 bytes
        let mut invalid_nonce = q.clone();
        invalid_nonce.nonce = Some("a".repeat(33));
        assert_eq!(invalid_nonce.validate(), Err(CommandError::NonceTooLong { len: 33 }));
    }

    #[test]
    fn request_soundboard_sounds_payload() {
        let req = RequestSoundboardSounds::new(vec![Snowflake::new(111), Snowflake::new(222)]);
        let raw = req.to_payload().unwrap();
        let val: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(val["op"], 31);
        assert_eq!(val["d"]["guild_ids"], serde_json::json!(["111", "222"]));
    }
}
