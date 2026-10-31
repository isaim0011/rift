//! Auto Moderation models and events.

use crate::serde_util::{
    de_opt_snowflake, de_snowflake, de_vec_snowflake, ser_opt_snowflake, ser_snowflake,
    ser_vec_snowflake,
};
use rift_core::Snowflake;
use serde::{Deserialize, Serialize};

/// Action metadata for an auto moderation action.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoModerationActionMetadata {
    /// Channel to which user content should be logged.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub channel_id: Option<Snowflake>,

    /// Timeout duration in seconds (max 2419200 / 4 weeks).
    #[serde(default)]
    pub duration_seconds: Option<u32>,

    /// Additional explanation that will be shown to members whenever their execution is blocked (max 150 chars).
    #[serde(default)]
    pub custom_message: Option<String>,
}

/// An action which will execute whenever a rule is triggered.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoModerationAction {
    /// The type of action (1=BlockMessage, 2=SendAlertMessage, 3=Timeout).
    #[serde(rename = "type")]
    pub kind: u8,

    /// Additional metadata needed during execution for this specific action type.
    #[serde(default)]
    pub metadata: Option<AutoModerationActionMetadata>,
}

/// Trigger metadata associated with an Auto Moderation rule.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoModerationTriggerMetadata {
    /// Substrings which will be searched for in content.
    #[serde(default)]
    pub keyword_filter: Option<Vec<String>>,

    /// Regular expression patterns which will be matched against content (max 10).
    #[serde(default)]
    pub regex_patterns: Option<Vec<String>>,

    /// The internally pre-defined wordsets which will be searched for in content.
    #[serde(default)]
    pub presets: Option<Vec<u8>>,

    /// Substrings which should not trigger the rule.
    #[serde(default)]
    pub allow_list: Option<Vec<String>>,

    /// Total number of unique role and user mentions allowed per message (max 50).
    #[serde(default)]
    pub mention_total_limit: Option<u32>,

    /// Whether to automatically detect mention raids.
    #[serde(default)]
    pub mention_raid_protection_enabled: Option<bool>,
}

/// An Auto Moderation rule object.
///
/// See <https://discord.com/developers/docs/resources/auto-moderation#auto-moderation-rule-object>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoModerationRule {
    /// ID of this rule.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// Guild ID which this rule belongs to.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// Rule name.
    pub name: String,

    /// User ID which first created this rule.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub creator_id: Snowflake,

    /// Rule event type.
    pub event_type: u8,

    /// Rule trigger type.
    pub trigger_type: u8,

    /// Rule trigger metadata.
    #[serde(default)]
    pub trigger_metadata: Option<AutoModerationTriggerMetadata>,

    /// Actions which will execute when this rule is triggered.
    pub actions: Vec<AutoModerationAction>,

    /// Whether the rule is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Role IDs that should not be affected by the rule (max 20).
    #[serde(
        default,
        deserialize_with = "de_vec_snowflake",
        serialize_with = "ser_vec_snowflake"
    )]
    pub exempt_roles: Vec<Snowflake>,

    /// Channel IDs that should not be affected by the rule (max 50).
    #[serde(
        default,
        deserialize_with = "de_vec_snowflake",
        serialize_with = "ser_vec_snowflake"
    )]
    pub exempt_channels: Vec<Snowflake>,
}

/// Dispatched when an Auto Moderation action is executed.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#auto-moderation-action-execution>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoModerationActionExecution {
    /// ID of the guild in which action was executed.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// Action which was executed.
    pub action: AutoModerationAction,

    /// ID of the rule which action belongs to.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub rule_id: Snowflake,

    /// Trigger type of rule which action belongs to.
    pub rule_trigger_type: u8,

    /// ID of the user which generated the content which triggered the action.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub user_id: Snowflake,

    /// ID of the channel in which user content was posted.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub channel_id: Option<Snowflake>,

    /// ID of any user message which content belongs to.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub message_id: Option<Snowflake>,

    /// ID of any system auto moderation messages posted as a result of this action.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub alert_system_message_id: Option<Snowflake>,

    /// User-generated text content.
    #[serde(default)]
    pub content: Option<String>,

    /// Word or phrase configured in the rule that triggered the action.
    #[serde(default)]
    pub matched_keyword: Option<String>,

    /// Substring in content that triggered the action.
    #[serde(default)]
    pub matched_content: Option<String>,
}
