//! Interactions: parsing what Discord sends and responding to it.
//!
//! Everything here follows "Receiving and Responding to Interactions". The
//! rules that bite:
//! * You must send the **initial response within 3 seconds**, over HTTP (never
//!   over the gateway). The token is then valid for 15 minutes for edits and
//!   follow-ups.
//! * A deferred message response (`defer`) may only carry the `EPHEMERAL` flag.
//! * Ephemeral state can't be changed after the first response.

use crate::client::RestClient;
use crate::command::Choice;
use crate::component::{ActionRow, ActionRowChild, Component, ComponentError};
use crate::error::Error;
use crate::message::{
    de_opt_snowflake, de_snowflake, ser_flags, validate_text_parts, AllowedMentions, Embed, Message,
    ValidationError,
};
use crate::transport::Transport;
use rift_core::{MessageFlags, Permissions, Snowflake};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::value::RawValue;

// ---------- incoming ----------

/// What kind of interaction this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractionType {
    /// Webhook verification ping (HTTP mode only).
    Ping,
    /// A slash, user or message command.
    ApplicationCommand,
    /// A button, select menu, ... was used.
    MessageComponent,
    /// A command option is being autocompleted.
    ApplicationCommandAutocomplete,
    /// A modal was submitted.
    ModalSubmit,
    /// A type this version doesn't know (future-proofing; never an error).
    Unknown(u8),
}

impl InteractionType {
    /// Parse the raw value.
    pub const fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Ping,
            2 => Self::ApplicationCommand,
            3 => Self::MessageComponent,
            4 => Self::ApplicationCommandAutocomplete,
            5 => Self::ModalSubmit,
            other => Self::Unknown(other),
        }
    }
}

impl<'de> Deserialize<'de> for InteractionType {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Self::from_u8(u8::deserialize(d)?))
    }
}

/// The parts of a user object most bots need.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct User {
    /// User id.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// Username.
    #[serde(default)]
    pub username: String,
    /// Whether this is a bot.
    #[serde(default)]
    pub bot: bool,
}

/// The parts of a guild member object most bots need.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Member {
    /// The user.
    #[serde(default)]
    pub user: Option<User>,
    /// Server nickname.
    #[serde(default)]
    pub nick: Option<String>,
    /// Permissions in the channel of the interaction, as Discord's decimal string.
    #[serde(default)]
    pub permissions: Option<String>,
}

/// An interaction received over the gateway (`INTERACTION_CREATE`) or HTTP.
///
/// `data` is kept raw and parsed on demand by [`command`](Self::command) /
/// [`component`](Self::component), so unrelated interactions cost nothing extra.
#[derive(Clone, Debug, Deserialize)]
pub struct Interaction {
    /// Interaction id.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// Your application's id.
    #[serde(deserialize_with = "de_snowflake")]
    pub application_id: Snowflake,
    /// Interaction type.
    #[serde(rename = "type")]
    pub kind: InteractionType,
    /// Continuation token. Treat as a secret; valid for 15 minutes.
    pub token: String,
    #[serde(default)]
    data: Option<Box<RawValue>>,
    /// Guild, if invoked in one.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub guild_id: Option<Snowflake>,
    /// Channel.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub channel_id: Option<Snowflake>,
    /// Invoking member (guild) ...
    #[serde(default)]
    pub member: Option<Member>,
    /// ... or invoking user (DM).
    #[serde(default)]
    pub user: Option<User>,
    /// The invoking user's locale.
    #[serde(default)]
    pub locale: Option<String>,
    /// Permissions the app has where this was invoked (decimal string).
    #[serde(default)]
    pub app_permissions: Option<String>,
}

impl Interaction {
    /// The invoking user, whether from `member` (guild) or `user` (DM).
    pub fn invoking_user(&self) -> Option<&User> {
        self.member.as_ref().and_then(|m| m.user.as_ref()).or(self.user.as_ref())
    }

    /// The invoking member's permissions in this channel.
    pub fn member_permissions(&self) -> Option<Permissions> {
        self.member.as_ref()?.permissions.as_deref()?.parse::<Permissions>().ok()
    }

    /// Permissions the app has in this channel.
    pub fn app_permissions(&self) -> Option<Permissions> {
        self.app_permissions.as_deref()?.parse::<Permissions>().ok()
    }

    /// Command data (for `ApplicationCommand` and autocomplete interactions).
    pub fn command(&self) -> Result<CommandData, Error> {
        match self.kind {
            InteractionType::ApplicationCommand | InteractionType::ApplicationCommandAutocomplete => {
                self.parse_data()
            }
            _ => Err(Error::Decode("not a command interaction".into())),
        }
    }

    /// Component data (for `MessageComponent` interactions).
    pub fn component(&self) -> Result<ComponentData, Error> {
        match self.kind {
            InteractionType::MessageComponent => self.parse_data(),
            _ => Err(Error::Decode("not a component interaction".into())),
        }
    }

    /// Modal submission data (for `ModalSubmit` interactions, type 5).
    pub fn modal_submit(&self) -> Result<ModalSubmitData, Error> {
        match self.kind {
            InteractionType::ModalSubmit => self.parse_data(),
            _ => Err(Error::Decode("not a modal submit interaction".into())),
        }
    }

    /// The focused option being autocompleted (for `ApplicationCommandAutocomplete` interactions, type 4).
    ///
    /// Searches recursively through top-level options, subcommands, and subcommand groups.
    pub fn autocomplete(&self) -> Result<CommandOption, Error> {
        match self.kind {
            InteractionType::ApplicationCommandAutocomplete => {
                let cmd = self.command()?;
                find_focused_option(&cmd.options)
                    .cloned()
                    .ok_or_else(|| Error::Decode("no focused option found in autocomplete interaction".into()))
            }
            _ => Err(Error::Decode("not an autocomplete interaction".into())),
        }
    }

    fn parse_data<T: DeserializeOwned>(&self) -> Result<T, Error> {
        let raw = self.data.as_ref().ok_or_else(|| Error::Decode("interaction has no data".into()))?;
        serde_json::from_str(raw.get()).map_err(|e| Error::Decode(e.to_string()))
    }
}

/// Data of a command interaction.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct CommandData {
    /// Command id.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// Command name.
    pub name: String,
    /// Command type (1 = chat input, ...).
    #[serde(rename = "type", default)]
    pub kind: Option<u8>,
    /// Options the user supplied (nested for subcommands and groups).
    #[serde(default)]
    pub options: Vec<CommandOption>,
    /// Target of a user or message command.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub target_id: Option<Snowflake>,
}

impl CommandData {
    /// Find a top-level option by name.
    pub fn option(&self, name: &str) -> Option<&CommandOption> {
        self.options.iter().find(|o| o.name == name)
    }

    /// Find the focused option being autocompleted (recursive search).
    pub fn focused_option(&self) -> Option<&CommandOption> {
        find_focused_option(&self.options)
    }
}

fn find_focused_option(options: &[CommandOption]) -> Option<&CommandOption> {
    for opt in options {
        if opt.focused {
            return Some(opt);
        }
        if let Some(nested) = find_focused_option(&opt.options) {
            return Some(nested);
        }
    }
    None
}

/// One option of a command interaction. `value` and `options` are mutually
/// exclusive: a subcommand or group has `options`, a parameter has `value`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct CommandOption {
    /// Option name.
    pub name: String,
    /// Option type.
    #[serde(rename = "type")]
    pub kind: u8,
    /// The user's input.
    #[serde(default)]
    pub value: Option<serde_json::Value>,
    /// Nested options (subcommands / groups).
    #[serde(default)]
    pub options: Vec<CommandOption>,
    /// True for the option being autocompleted.
    #[serde(default)]
    pub focused: bool,
}

impl CommandOption {
    /// String value.
    pub fn as_str(&self) -> Option<&str> {
        self.value.as_ref()?.as_str()
    }
    /// Integer value.
    pub fn as_i64(&self) -> Option<i64> {
        self.value.as_ref()?.as_i64()
    }
    /// Floating-point value.
    pub fn as_f64(&self) -> Option<f64> {
        self.value.as_ref()?.as_f64()
    }
    /// Boolean value.
    pub fn as_bool(&self) -> Option<bool> {
        self.value.as_ref()?.as_bool()
    }
    /// Snowflake value (user, channel, role, mentionable and attachment options).
    pub fn as_snowflake(&self) -> Option<Snowflake> {
        self.as_str()?.parse().ok()
    }
}

/// Data of a message-component interaction.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ComponentData {
    /// The component's `custom_id`.
    pub custom_id: String,
    /// Component type.
    pub component_type: u8,
    /// Selected values (select menus).
    #[serde(default)]
    pub values: Vec<String>,
}

/// Data of a modal submit interaction (type 5).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ModalSubmitData {
    /// The modal's `custom_id`.
    pub custom_id: String,
    /// The submitted rows of components.
    #[serde(default)]
    pub components: Vec<ModalSubmitRow>,
}

impl ModalSubmitData {
    /// Find the submitted string value of a text input component by its `custom_id`.
    pub fn value(&self, custom_id: &str) -> Option<&str> {
        for row in &self.components {
            for comp in &row.components {
                if comp.custom_id == custom_id {
                    return comp.value.as_deref();
                }
            }
        }
        None
    }

    /// Find a submitted component by its `custom_id`.
    pub fn component(&self, custom_id: &str) -> Option<&ModalSubmitComponent> {
        for row in &self.components {
            for comp in &row.components {
                if comp.custom_id == custom_id {
                    return Some(comp);
                }
            }
        }
        None
    }
}

/// A row of submitted components in a modal submission.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ModalSubmitRow {
    /// Component type (usually 1 for ActionRow).
    #[serde(rename = "type", default = "default_action_row_type")]
    pub kind: u8,
    /// The components in this row.
    #[serde(default)]
    pub components: Vec<ModalSubmitComponent>,
}

const fn default_action_row_type() -> u8 {
    1
}

/// A single component submitted in a modal (e.g. TextInput or SelectMenu).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ModalSubmitComponent {
    /// The component's `custom_id`.
    pub custom_id: String,
    /// Component type (usually 4 for TextInput).
    #[serde(rename = "type")]
    pub component_type: u8,
    /// The value entered by the user (for text inputs).
    #[serde(default)]
    pub value: Option<String>,
    /// Selected values (for select menus).
    #[serde(default)]
    pub values: Vec<String>,
}

// ---------- outgoing ----------

/// The message part of an interaction response, follow-up or edit.
/// Supports the fields the docs list as available: not every message field is.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct InteractionMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    tts: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    embeds: Vec<Embed>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_mentions: Option<AllowedMentions>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    components: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_flags")]
    flags: Option<MessageFlags>,
}

impl InteractionMessage {
    /// Empty message.
    pub fn new() -> Self {
        Self::default()
    }
    /// Text (max 2000 characters).
    pub fn content(mut self, v: impl Into<String>) -> Self {
        self.content = Some(v.into());
        self
    }
    /// Add an embed (max 10).
    pub fn embed(mut self, e: Embed) -> Self {
        self.embeds.push(e);
        self
    }
    /// Text-to-speech.
    pub fn tts(mut self, v: bool) -> Self {
        self.tts = Some(v);
        self
    }
    /// Control who gets pinged. Use [`AllowedMentions::none`] when echoing user input.
    pub fn allowed_mentions(mut self, a: AllowedMentions) -> Self {
        self.allowed_mentions = Some(a);
        self
    }
    /// Raw component JSON (typed components come later).
    pub fn components(mut self, c: Vec<serde_json::Value>) -> Self {
        self.components = c;
        self
    }
    /// Add typed action rows to the message.
    pub fn action_rows(mut self, rows: impl IntoIterator<Item = ActionRow>) -> Result<Self, ComponentError> {
        for row in rows {
            row.validate()?;
            let val = serde_json::to_value(&row).map_err(|_| ComponentError::InvalidModalComponents {
                reason: "failed to serialize action row",
            })?;
            self.components.push(val);
        }
        Ok(self)
    }
    /// Add typed components (including Components V2) to the message.
    pub fn typed_components(mut self, components: impl IntoIterator<Item = Component>) -> Result<Self, ComponentError> {
        for comp in components {
            comp.validate()?;
            let val = serde_json::to_value(&comp).map_err(|_| ComponentError::InvalidModalComponents {
                reason: "failed to serialize component",
            })?;
            self.components.push(val);
        }
        Ok(self)
    }
    /// Set flags (only `SUPPRESS_EMBEDS`, `EPHEMERAL`, `IS_COMPONENTS_V2`,
    /// `IS_VOICE_MESSAGE`, `SUPPRESS_NOTIFICATIONS` are accepted).
    pub fn flags(mut self, f: MessageFlags) -> Self {
        self.flags = Some(f);
        self
    }
    /// Only the invoking user sees it. Must be set on the *first* response.
    pub fn ephemeral(mut self) -> Self {
        self.flags = Some(self.flags.unwrap_or(MessageFlags::NONE) | MessageFlags::EPHEMERAL);
        self
    }
    /// Send without a notification.
    pub fn silent(mut self) -> Self {
        self.flags = Some(self.flags.unwrap_or(MessageFlags::NONE) | MessageFlags::SUPPRESS_NOTIFICATIONS);
        self
    }

    /// Local checks (used for edits, where an empty body is legal).
    pub fn validate(&self) -> Result<(), ValidationError> {
        let flags = self.flags.unwrap_or(MessageFlags::NONE);
        let bad = flags.unsettable_in_interaction_response();
        if bad != 0 {
            return Err(ValidationError::FlagNotSettable { bits: bad });
        }
        if flags.contains(MessageFlags::IS_COMPONENTS_V2) {
            if self.content.is_some() {
                return Err(ValidationError::ComponentsV2Conflict { field: "content" });
            }
            if !self.embeds.is_empty() {
                return Err(ValidationError::ComponentsV2Conflict { field: "embeds" });
            }
        }
        validate_text_parts(self.content.as_deref(), &self.embeds, self.allowed_mentions.as_ref())
    }

    /// Local checks for a *new* message (must have something to show).
    pub(crate) fn validate_new(&self) -> Result<(), ValidationError> {
        self.validate()?;
        let has_payload = self.content.as_deref().is_some_and(|c| !c.is_empty())
            || !self.embeds.is_empty()
            || !self.components.is_empty();
        if has_payload {
            Ok(())
        } else {
            Err(ValidationError::Empty)
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub(crate) enum InteractionResponseData {
    Message(InteractionMessage),
    Autocomplete(AutocompleteData),
    Modal(ModalData),
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub(crate) struct AutocompleteData {
    pub choices: Vec<Choice>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub(crate) struct ModalData {
    pub custom_id: String,
    pub title: String,
    pub components: Vec<ActionRow>,
}

/// The initial response to an interaction.
#[derive(Clone, Debug, PartialEq)]
pub struct InteractionResponse {
    pub(crate) kind: u8,
    pub(crate) data: Option<InteractionResponseData>,
}

impl Serialize for InteractionResponse {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = s.serialize_struct("InteractionResponse", 2)?;
        state.serialize_field("type", &self.kind)?;
        if let Some(ref data) = self.data {
            state.serialize_field("data", data)?;
        }
        state.end()
    }
}

impl InteractionResponse {
    /// Answer a `Ping` (HTTP mode).
    pub fn pong() -> Self {
        Self { kind: 1, data: None }
    }

    /// Respond with a message (`CHANNEL_MESSAGE_WITH_SOURCE`, type 4).
    pub fn message(m: InteractionMessage) -> Self {
        Self {
            kind: 4,
            data: Some(InteractionResponseData::Message(m)),
        }
    }

    /// Acknowledge now and answer later via
    /// [`RestClient::edit_original_response`] (`DEFERRED_CHANNEL_MESSAGE_WITH_SOURCE`, type 5).
    /// The user sees a loading state. `ephemeral` can't be changed afterwards.
    pub fn defer(ephemeral: bool) -> Self {
        Self {
            kind: 5,
            data: ephemeral.then(|| InteractionResponseData::Message(InteractionMessage::new().ephemeral())),
        }
    }

    /// For components: acknowledge and edit the original message later, with no
    /// loading state (`DEFERRED_UPDATE_MESSAGE`, type 6).
    pub fn defer_update() -> Self {
        Self { kind: 6, data: None }
    }

    /// For components: edit the message the component is on (`UPDATE_MESSAGE`, type 7).
    pub fn update_message(m: InteractionMessage) -> Self {
        Self {
            kind: 7,
            data: Some(InteractionResponseData::Message(m)),
        }
    }

    /// Answer an autocomplete request (`APPLICATION_COMMAND_AUTOCOMPLETE_RESULT`, type 8).
    ///
    /// Validates locally that `choices` contains at most 25 items.
    pub fn autocomplete(choices: Vec<Choice>) -> Result<Self, ComponentError> {
        if choices.len() > 25 {
            return Err(ComponentError::TooManyAutocompleteChoices {
                count: choices.len(),
                max: 25,
            });
        }
        Ok(Self {
            kind: 8,
            data: Some(InteractionResponseData::Autocomplete(AutocompleteData { choices })),
        })
    }

    /// Respond with a modal popup (`MODAL`, type 9).
    ///
    /// Validates locally:
    /// - `custom_id`: 1..=100 characters.
    /// - `title`: 1..=45 characters.
    /// - `components`: 1..=5 ActionRows containing only TextInput components.
    pub fn modal(
        custom_id: impl Into<String>,
        title: impl Into<String>,
        components: Vec<ActionRow>,
    ) -> Result<Self, ComponentError> {
        let cid = custom_id.into();
        let cid_len = cid.chars().count();
        if !(1..=crate::component::MAX_CUSTOM_ID_LEN).contains(&cid_len) {
            return Err(ComponentError::InvalidModalCustomIdLength {
                len: cid_len,
                max: crate::component::MAX_CUSTOM_ID_LEN,
            });
        }

        let t = title.into();
        let t_len = t.chars().count();
        if !(1..=crate::component::MAX_TEXT_INPUT_LABEL_LEN).contains(&t_len) {
            return Err(ComponentError::InvalidModalTitleLength {
                len: t_len,
                max: crate::component::MAX_TEXT_INPUT_LABEL_LEN,
            });
        }

        if components.is_empty() || components.len() > crate::component::MAX_ACTION_ROWS_PER_MESSAGE {
            return Err(ComponentError::InvalidModalComponents {
                reason: "modal must contain between 1 and 5 action rows",
            });
        }

        for row in &components {
            row.validate()?;
            if row.components.is_empty() {
                return Err(ComponentError::InvalidModalComponents {
                    reason: "action row cannot be empty",
                });
            }
            for comp in &row.components {
                if !matches!(comp, ActionRowChild::TextInput(_)) {
                    return Err(ComponentError::InvalidModalComponents {
                        reason: "action row in a modal may only contain text input components",
                    });
                }
            }
        }

        Ok(Self {
            kind: 9,
            data: Some(InteractionResponseData::Modal(ModalData {
                custom_id: cid,
                title: t,
                components,
            })),
        })
    }

    /// Local checks, run before sending.
    pub fn validate(&self) -> Result<(), ValidationError> {
        match (self.kind, &self.data) {
            (4, Some(InteractionResponseData::Message(m))) => m.validate_new(),
            (7, Some(InteractionResponseData::Message(m))) => m.validate(),
            _ => Ok(()),
        }
    }
}

impl<T: Transport> RestClient<T> {
    /// `POST /interactions/{id}/{token}/callback`: the initial response. Must
    /// arrive within 3 seconds of the interaction. Not bucket-limited or
    /// globally limited; respond as early as you can.
    pub async fn create_interaction_response(
        &self,
        interaction_id: Snowflake,
        token: &str,
        response: &InteractionResponse,
    ) -> Result<(), Error> {
        response.validate()?;
        let body = serde_json::to_vec(response).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/interactions/{interaction_id}/{token}/callback");
        self.request("POST", &path, Some(body)).await?.expect_success()?;
        Ok(())
    }

    /// `PATCH /webhooks/{app}/{token}/messages/@original`: edit the initial
    /// response (the way to finish a deferred one). Valid for 15 minutes.
    pub async fn edit_original_response(
        &self,
        application_id: Snowflake,
        token: &str,
        msg: &InteractionMessage,
    ) -> Result<Message, Error> {
        msg.validate()?;
        let body = serde_json::to_vec(msg).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/webhooks/{application_id}/{token}/messages/@original");
        self.request("PATCH", &path, Some(body)).await?.expect_success()?.json()
    }

    /// `DELETE /webhooks/{app}/{token}/messages/@original`.
    pub async fn delete_original_response(&self, application_id: Snowflake, token: &str) -> Result<(), Error> {
        let path = format!("/webhooks/{application_id}/{token}/messages/@original");
        self.request("DELETE", &path, None).await?.expect_success()?;
        Ok(())
    }

    /// `POST /webhooks/{app}/{token}`: a new follow-up message. Right after a
    /// deferred response, Discord treats the first follow-up as an edit of the
    /// loading message (deprecated; prefer [`edit_original_response`](Self::edit_original_response)).
    pub async fn create_followup(
        &self,
        application_id: Snowflake,
        token: &str,
        msg: &InteractionMessage,
    ) -> Result<Message, Error> {
        msg.validate_new()?;
        let body = serde_json::to_vec(msg).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/webhooks/{application_id}/{token}");
        self.request("POST", &path, Some(body)).await?.expect_success()?.json()
    }

    /// `PATCH /webhooks/{app}/{token}/messages/{id}`: edit a follow-up.
    pub async fn edit_followup(
        &self,
        application_id: Snowflake,
        token: &str,
        message_id: Snowflake,
        msg: &InteractionMessage,
    ) -> Result<Message, Error> {
        msg.validate()?;
        let body = serde_json::to_vec(msg).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/webhooks/{application_id}/{token}/messages/{message_id}");
        self.request("PATCH", &path, Some(body)).await?.expect_success()?.json()
    }

    /// `DELETE /webhooks/{app}/{token}/messages/{id}`: delete a follow-up.
    pub async fn delete_followup(
        &self,
        application_id: Snowflake,
        token: &str,
        message_id: Snowflake,
    ) -> Result<(), Error> {
        let path = format!("/webhooks/{application_id}/{token}/messages/{message_id}");
        self.request("DELETE", &path, None).await?.expect_success()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{client, resp};
    use serde_json::json;

    fn sf(n: u64) -> Snowflake {
        Snowflake::new(n)
    }

    const SLASH: &str = r#"{
        "id": "1000", "application_id": "2000", "type": 2, "token": "tok123", "version": 1,
        "guild_id": "3000", "channel_id": "4000", "locale": "en-US", "app_permissions": "1",
        "member": { "user": { "id": "5000", "username": "gugu" }, "permissions": "2147483647", "nick": null },
        "data": { "id": "6000", "name": "ping", "type": 1, "options": [
            { "name": "text", "type": 3, "value": "hi" },
            { "name": "count", "type": 4, "value": 42 },
            { "name": "ratio", "type": 10, "value": 0.5 },
            { "name": "loud", "type": 5, "value": true },
            { "name": "who", "type": 6, "value": "7000" }
        ] }
    }"#;

    #[test]
    fn parses_a_slash_command_interaction() {
        let i: Interaction = serde_json::from_str(SLASH).unwrap();
        assert_eq!(i.kind, InteractionType::ApplicationCommand);
        assert_eq!((i.id, i.application_id), (sf(1000), sf(2000)));
        assert_eq!(i.token, "tok123");
        assert_eq!((i.guild_id, i.channel_id), (Some(sf(3000)), Some(sf(4000))));
        assert_eq!(i.invoking_user().unwrap().id, sf(5000));
        assert_eq!(i.member_permissions(), Some(Permissions::from_bits_retain(2_147_483_647)));
        assert_eq!(i.app_permissions(), Some(Permissions::CREATE_INSTANT_INVITE));

        let c = i.command().unwrap();
        assert_eq!((c.id, c.name.as_str(), c.kind), (sf(6000), "ping", Some(1)));
        assert_eq!(c.option("text").unwrap().as_str(), Some("hi"));
        assert_eq!(c.option("count").unwrap().as_i64(), Some(42));
        assert_eq!(c.option("ratio").unwrap().as_f64(), Some(0.5));
        assert_eq!(c.option("loud").unwrap().as_bool(), Some(true));
        assert_eq!(c.option("who").unwrap().as_snowflake(), Some(sf(7000)));
        assert!(c.option("missing").is_none());
        // Asking for the wrong type gives None, not a panic.
        assert_eq!(c.option("text").unwrap().as_i64(), None);
    }

    #[test]
    fn subcommands_nest_their_options() {
        let raw = r#"{"id":"1","application_id":"2","type":2,"token":"t","data":{"id":"3","name":"config",
            "options":[{"name":"set","type":1,"options":[{"name":"key","type":3,"value":"theme"}]}]}}"#;
        let c = serde_json::from_str::<Interaction>(raw).unwrap().command().unwrap();
        let sub = c.option("set").unwrap();
        assert!(sub.value.is_none());
        assert_eq!(sub.options[0].as_str(), Some("theme"));
    }

    #[test]
    fn parses_a_component_interaction_from_a_dm() {
        let raw = r#"{"id":"1","application_id":"2","type":3,"token":"t",
            "user":{"id":"9","username":"u"},
            "data":{"custom_id":"menu","component_type":3,"values":["a","b"]},
            "message":{"id":"5","content":"ignored"}}"#;
        let i: Interaction = serde_json::from_str(raw).unwrap();
        assert_eq!(i.invoking_user().unwrap().id, sf(9));
        let c = i.component().unwrap();
        assert_eq!((c.custom_id.as_str(), c.component_type), ("menu", 3));
        assert_eq!(c.values, vec!["a", "b"]);
        // It's not a command.
        assert!(i.command().is_err());
    }

    #[test]
    fn ping_has_no_data_and_unknown_types_do_not_fail() {
        let ping: Interaction =
            serde_json::from_str(r#"{"id":"1","application_id":"2","type":1,"token":"t"}"#).unwrap();
        assert_eq!(ping.kind, InteractionType::Ping);
        assert!(ping.command().is_err());

        let future: Interaction =
            serde_json::from_str(r#"{"id":"1","application_id":"2","type":99,"token":"t"}"#).unwrap();
        assert_eq!(future.kind, InteractionType::Unknown(99));
    }

    #[test]
    fn responses_serialize_to_the_documented_shapes() {
        let to = |r: &InteractionResponse| serde_json::to_value(r).unwrap();
        assert_eq!(to(&InteractionResponse::pong()), json!({ "type": 1 }));
        assert_eq!(to(&InteractionResponse::defer(false)), json!({ "type": 5 }));
        assert_eq!(to(&InteractionResponse::defer(true)), json!({ "type": 5, "data": { "flags": 64 } }));
        assert_eq!(to(&InteractionResponse::defer_update()), json!({ "type": 6 }));
        assert_eq!(
            to(&InteractionResponse::message(InteractionMessage::new().content("hi").ephemeral())),
            json!({ "type": 4, "data": { "content": "hi", "flags": 64 } })
        );
        assert_eq!(
            to(&InteractionResponse::update_message(InteractionMessage::new().content("v2"))),
            json!({ "type": 7, "data": { "content": "v2" } })
        );
    }

    #[test]
    fn validation_matches_the_docs() {
        let ok = InteractionResponse::message(InteractionMessage::new().content("hi").ephemeral().silent());
        assert!(ok.validate().is_ok());

        // Something must be shown.
        assert_eq!(
            InteractionResponse::message(InteractionMessage::new()).validate(),
            Err(ValidationError::Empty)
        );
        // CROSSPOSTED is not allowed in a response.
        let bad = InteractionMessage::new().content("x").flags(MessageFlags::CROSSPOSTED);
        assert_eq!(bad.validate(), Err(ValidationError::FlagNotSettable { bits: 1 }));
        // Components V2 can't be mixed with content.
        let v2 = InteractionMessage::new().content("x").flags(MessageFlags::IS_COMPONENTS_V2);
        assert_eq!(v2.validate(), Err(ValidationError::ComponentsV2Conflict { field: "content" }));
        // Same text limits as normal messages.
        let long = InteractionMessage::new().content("a".repeat(2001));
        assert_eq!(long.validate(), Err(ValidationError::ContentTooLong { len: 2001 }));
        // An edit may legitimately be empty; a new message may not.
        assert!(InteractionMessage::new().validate().is_ok());
        assert_eq!(InteractionMessage::new().validate_new(), Err(ValidationError::Empty));
    }

    #[tokio::test(start_paused = true)]
    async fn initial_response_goes_to_the_callback_route() {
        let c = client(vec![resp(204, "")]);
        let r = InteractionResponse::message(InteractionMessage::new().content("pong"));
        c.create_interaction_response(sf(10), "tok", &r).await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/interactions/10/tok/callback");
        let body: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(body, json!({ "type": 4, "data": { "content": "pong" } }));
    }

    #[tokio::test(start_paused = true)]
    async fn invalid_responses_are_never_sent() {
        let c = client(vec![]);
        let r = InteractionResponse::message(InteractionMessage::new());
        let out = c.create_interaction_response(sf(10), "tok", &r).await;
        assert!(matches!(out, Err(Error::Validation(ValidationError::Empty))));
        assert!(c.transport().seen.lock().unwrap().is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn deferred_flow_defers_then_edits_the_original() {
        let msg = r#"{"id":"77","channel_id":"88","content":"done"}"#;
        let c = client(vec![resp(204, ""), resp(200, msg)]);

        c.create_interaction_response(sf(10), "tok", &InteractionResponse::defer(true)).await.unwrap();
        let done = c
            .edit_original_response(sf(20), "tok", &InteractionMessage::new().content("done"))
            .await
            .unwrap();
        assert_eq!((done.id, done.content.as_str()), (sf(77), "done"));

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[1].method, "PATCH");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/webhooks/20/tok/messages/@original");
    }

    #[tokio::test(start_paused = true)]
    async fn followup_routes_match_the_docs() {
        let msg = r#"{"id":"77","channel_id":"88","content":"x"}"#;
        let c = client(vec![resp(200, msg), resp(200, msg), resp(204, ""), resp(204, "")]);
        let m = InteractionMessage::new().content("x");

        c.create_followup(sf(20), "tok", &m).await.unwrap();
        c.edit_followup(sf(20), "tok", sf(77), &m).await.unwrap();
        c.delete_followup(sf(20), "tok", sf(77)).await.unwrap();
        c.delete_original_response(sf(20), "tok").await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        let routes: Vec<(&str, &str)> = seen.iter().map(|r| (r.method.as_str(), r.url.as_str())).collect();
        assert_eq!(
            routes,
            vec![
                ("POST", "https://discord.com/api/v10/webhooks/20/tok"),
                ("PATCH", "https://discord.com/api/v10/webhooks/20/tok/messages/77"),
                ("DELETE", "https://discord.com/api/v10/webhooks/20/tok/messages/77"),
                ("DELETE", "https://discord.com/api/v10/webhooks/20/tok/messages/@original"),
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_dead_interaction_token_is_not_retried() {
        // 404 on a webhook route marks that token dead; the limiter then refuses
        // further calls with it instead of hammering Discord.
        let c = client(vec![resp(404, r#"{"code":10015,"message":"Unknown Webhook"}"#)]);
        let m = InteractionMessage::new().content("late");
        let first = c.create_followup(sf(20), "expired", &m).await;
        assert!(matches!(first, Err(Error::Api { status: 404, .. })));

        let second = c.create_followup(sf(20), "expired", &m).await;
        assert!(matches!(second, Err(Error::Blocked(_))));
        assert_eq!(c.transport().seen.lock().unwrap().len(), 1);
    }

    #[test]
    fn autocomplete_response_serialization_and_validation() {
        let choices = vec![
            Choice::string("Apple", "apple"),
            Choice::int("Forty Two", 42),
            Choice::number("Half", 0.5),
        ];
        let resp = InteractionResponse::autocomplete(choices).unwrap();
        let val = serde_json::to_value(&resp).unwrap();
        assert_eq!(
            val,
            json!({
                "type": 8,
                "data": {
                    "choices": [
                        { "name": "Apple", "value": "apple" },
                        { "name": "Forty Two", "value": 42 },
                        { "name": "Half", "value": 0.5 }
                    ]
                }
            })
        );

        // Validation rejects more than 25 choices locally
        let too_many = (0..26)
            .map(|i| Choice::string(format!("Name {i}"), format!("val_{i}")))
            .collect::<Vec<_>>();
        let err = InteractionResponse::autocomplete(too_many).unwrap_err();
        assert_eq!(
            err,
            ComponentError::TooManyAutocompleteChoices { count: 26, max: 25 }
        );
    }

    #[test]
    fn modal_response_serialization_and_validation() {
        use crate::component::TextInput;

        let input1 = TextInput::short("name", "Your Name").placeholder("Jane Doe");
        let input2 = TextInput::paragraph("bio", "About You").min_length(10);
        let row1 = ActionRow::text_input(input1);
        let row2 = ActionRow::text_input(input2);

        let resp = InteractionResponse::modal("survey_modal", "User Survey", vec![row1, row2]).unwrap();
        let val = serde_json::to_value(&resp).unwrap();
        assert_eq!(
            val,
            json!({
                "type": 9,
                "data": {
                    "custom_id": "survey_modal",
                    "title": "User Survey",
                    "components": [
                        {
                            "type": 1,
                            "components": [
                                {
                                    "type": 4,
                                    "custom_id": "name",
                                    "style": 1,
                                    "label": "Your Name",
                                    "placeholder": "Jane Doe"
                                }
                            ]
                        },
                        {
                            "type": 1,
                            "components": [
                                {
                                    "type": 4,
                                    "custom_id": "bio",
                                    "style": 2,
                                    "label": "About You",
                                    "min_length": 10
                                }
                            ]
                        }
                    ]
                }
            })
        );

        // Validation: custom_id bounds
        assert_eq!(
            InteractionResponse::modal("", "Title", vec![ActionRow::text_input(TextInput::short("k", "v"))]).unwrap_err(),
            ComponentError::InvalidModalCustomIdLength { len: 0, max: 100 }
        );
        assert_eq!(
            InteractionResponse::modal("a".repeat(101), "Title", vec![ActionRow::text_input(TextInput::short("k", "v"))]).unwrap_err(),
            ComponentError::InvalidModalCustomIdLength { len: 101, max: 100 }
        );

        // Validation: title bounds
        assert_eq!(
            InteractionResponse::modal("id", "", vec![ActionRow::text_input(TextInput::short("k", "v"))]).unwrap_err(),
            ComponentError::InvalidModalTitleLength { len: 0, max: 45 }
        );
        assert_eq!(
            InteractionResponse::modal("id", "a".repeat(46), vec![ActionRow::text_input(TextInput::short("k", "v"))]).unwrap_err(),
            ComponentError::InvalidModalTitleLength { len: 46, max: 45 }
        );

        // Validation: components row count 1..=5
        assert_eq!(
            InteractionResponse::modal("id", "Title", vec![]).unwrap_err(),
            ComponentError::InvalidModalComponents { reason: "modal must contain between 1 and 5 action rows" }
        );
        let six_rows = (0..6)
            .map(|i| ActionRow::text_input(TextInput::short(format!("id_{i}"), "Label")))
            .collect::<Vec<_>>();
        assert_eq!(
            InteractionResponse::modal("id", "Title", six_rows).unwrap_err(),
            ComponentError::InvalidModalComponents { reason: "modal must contain between 1 and 5 action rows" }
        );

        // Validation: action rows in modal cannot contain buttons
        use crate::component::Button;
        let button_row = ActionRow::buttons(vec![Button::primary("b", "Btn")]);
        assert_eq!(
            InteractionResponse::modal("id", "Title", vec![button_row]).unwrap_err(),
            ComponentError::InvalidModalComponents {
                reason: "action row in a modal may only contain text input components"
            }
        );
    }

    #[test]
    fn parses_modal_submit_interaction() {
        let raw = r#"{
            "id": "1111",
            "application_id": "2222",
            "type": 5,
            "token": "tok_modal",
            "data": {
                "custom_id": "feedback_form",
                "components": [
                    {
                        "type": 1,
                        "components": [
                            {
                                "type": 4,
                                "custom_id": "feedback_input",
                                "value": "I love rift!"
                            }
                        ]
                    },
                    {
                        "type": 1,
                        "components": [
                            {
                                "type": 4,
                                "custom_id": "rating_input",
                                "value": "5"
                            }
                        ]
                    }
                ]
            }
        }"#;

        let interaction: Interaction = serde_json::from_str(raw).unwrap();
        assert_eq!(interaction.kind, InteractionType::ModalSubmit);
        let submit = interaction.modal_submit().unwrap();
        assert_eq!(submit.custom_id, "feedback_form");
        assert_eq!(submit.value("feedback_input"), Some("I love rift!"));
        assert_eq!(submit.value("rating_input"), Some("5"));
        assert_eq!(submit.value("missing"), None);

        let comp = submit.component("feedback_input").unwrap();
        assert_eq!(comp.component_type, 4);
        assert_eq!(comp.value.as_deref(), Some("I love rift!"));

        // Not a command
        assert!(interaction.command().is_err());
    }

    #[test]
    fn inspects_autocomplete_focused_option_nested() {
        let raw = r#"{
            "id": "1111",
            "application_id": "2222",
            "type": 4,
            "token": "tok_auto",
            "data": {
                "id": "3333",
                "name": "tag",
                "type": 1,
                "options": [
                    {
                        "name": "search",
                        "type": 1,
                        "options": [
                            {
                                "name": "category",
                                "type": 3,
                                "value": "help",
                                "focused": false
                            },
                            {
                                "name": "query",
                                "type": 3,
                                "value": "doc",
                                "focused": true
                            }
                        ]
                    }
                ]
            }
        }"#;

        let interaction: Interaction = serde_json::from_str(raw).unwrap();
        assert_eq!(interaction.kind, InteractionType::ApplicationCommandAutocomplete);

        let focused = interaction.autocomplete().unwrap();
        assert_eq!(focused.name, "query");
        assert_eq!(focused.as_str(), Some("doc"));
        assert!(focused.focused);

        let cmd = interaction.command().unwrap();
        assert_eq!(cmd.focused_option().unwrap().name, "query");
    }

    #[test]
    fn interaction_message_typed_components() {
        use crate::component::{Button, TextDisplay};

        let row = ActionRow::buttons(vec![Button::primary("b1", "B1")]);
        let msg = InteractionMessage::new()
            .action_rows(vec![row])
            .unwrap();
        assert_eq!(msg.components.len(), 1);

        let v2_comp = Component::TextDisplay(TextDisplay::new("Hello v2"));
        let msg2 = InteractionMessage::new()
            .typed_components(vec![v2_comp])
            .unwrap();
        assert_eq!(msg2.components.len(), 1);
    }
}
