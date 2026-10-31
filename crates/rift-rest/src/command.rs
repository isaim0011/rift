//! Typed application commands with local validation.
//!
//! Every limit comes from the "Application Commands" docs. Invalid commands
//! are rejected before any network call, which matters because command
//! creation has a daily per-guild limit (200 creates) that mistakes would eat.
//!
//! Not yet supported: localizations, entry-point commands, `file_types`.

use crate::client::RestClient;
use crate::error::Error;
use crate::message::{de_opt_snowflake, de_snowflake};
use crate::transport::Transport;
use rift_core::{Permissions, Snowflake};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use std::collections::HashSet;
use std::fmt;

const MAX_NAME: usize = 32;
const MAX_DESCRIPTION: usize = 100;
const MAX_OPTIONS: usize = 25;
const MAX_CHOICES: usize = 25;
const MAX_CHOICE_NAME: usize = 100;
const MAX_CHOICE_STRING: usize = 100;
const MAX_STRING_LENGTH: u32 = 6000;
/// Combined name + description + choice-value characters per slash command.
const MAX_TOTAL_CHARS: usize = 8000;
const MAX_SLASH_COMMANDS: usize = 100;
const MAX_USER_COMMANDS: usize = 15;
const MAX_MESSAGE_COMMANDS: usize = 15;
/// INTEGER options: any integer between -2^53+1 and 2^53-1.
const MAX_SAFE_INT: i64 = 9_007_199_254_740_991;
/// NUMBER options: any double between -2^53 and 2^53.
const MAX_NUMBER: f64 = 9_007_199_254_740_992.0;

// ---------- enums (serialized as integers) ----------

/// The kind of command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommandType {
    /// Slash command (`/name`).
    ChatInput,
    /// Right-click a user.
    User,
    /// Right-click a message.
    Message,
}

impl CommandType {
    const fn as_u8(self) -> u8 {
        match self {
            Self::ChatInput => 1,
            Self::User => 2,
            Self::Message => 3,
        }
    }
}

impl Serialize for CommandType {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(self.as_u8())
    }
}

/// The type of a slash-command option.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OptionType {
    /// A subcommand.
    SubCommand,
    /// A group of subcommands.
    SubCommandGroup,
    /// Text.
    String,
    /// Integer between -2^53+1 and 2^53-1.
    Integer,
    /// True/false.
    Boolean,
    /// A user.
    User,
    /// A channel (any type, including categories).
    Channel,
    /// A role.
    Role,
    /// A user or a role.
    Mentionable,
    /// A double between -2^53 and 2^53.
    Number,
    /// An uploaded file.
    Attachment,
}

impl OptionType {
    const fn as_u8(self) -> u8 {
        match self {
            Self::SubCommand => 1,
            Self::SubCommandGroup => 2,
            Self::String => 3,
            Self::Integer => 4,
            Self::Boolean => 5,
            Self::User => 6,
            Self::Channel => 7,
            Self::Role => 8,
            Self::Mentionable => 9,
            Self::Number => 10,
            Self::Attachment => 11,
        }
    }

    const fn is_sub(self) -> bool {
        matches!(self, Self::SubCommand | Self::SubCommandGroup)
    }

    /// Options that accept `choices` and `autocomplete`.
    const fn takes_choices(self) -> bool {
        matches!(self, Self::String | Self::Integer | Self::Number)
    }
}

impl Serialize for OptionType {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(self.as_u8())
    }
}

/// Where a command can be used. Global commands only; guild commands don't
/// support `BotDm`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractionContext {
    /// Inside servers.
    Guild,
    /// In DMs with the app's bot user.
    BotDm,
    /// In group DMs and DMs with other users (user-installed apps).
    PrivateChannel,
}

impl Serialize for InteractionContext {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(match self {
            Self::Guild => 0,
            Self::BotDm => 1,
            Self::PrivateChannel => 2,
        })
    }
}

/// Where the app is installed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrationType {
    /// Installed to a server.
    GuildInstall,
    /// Installed to a user.
    UserInstall,
}

impl Serialize for IntegrationType {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(match self {
            Self::GuildInstall => 0,
            Self::UserInstall => 1,
        })
    }
}

fn ser_permissions<S: Serializer>(v: &Option<Permissions>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        // Discord wants permissions as a decimal string.
        Some(p) => s.collect_str(p),
        None => s.serialize_none(),
    }
}

// ---------- errors ----------

/// Why a command definition was rejected locally.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandError {
    /// A command or option name breaks the naming rules.
    BadName {
        /// Where, e.g. `permissions/user/get`.
        path: String,
        /// The offending name.
        name: String,
        /// What's wrong with it.
        why: &'static str,
    },
    /// Description isn't 1-100 characters.
    BadDescription {
        /// Where.
        path: String,
        /// Actual length.
        len: usize,
    },
    /// A field isn't allowed here (e.g. `choices` on a boolean option).
    NotAllowed {
        /// Where.
        path: String,
        /// The field.
        what: &'static str,
    },
    /// Too many of something.
    TooMany {
        /// Where.
        path: String,
        /// What there are too many of.
        what: &'static str,
        /// How many.
        count: usize,
        /// The maximum.
        max: usize,
    },
    /// A name is used twice where it must be unique.
    Duplicate {
        /// Where.
        path: String,
        /// The repeated name.
        name: String,
    },
    /// A required option follows an optional one.
    RequiredAfterOptional {
        /// Where.
        path: String,
    },
    /// Conflicting or invalid values.
    Conflict {
        /// Where.
        path: String,
        /// What's wrong.
        why: &'static str,
    },
    /// Name, description and choice values together exceed 8000 characters.
    TooLarge {
        /// The combined size.
        total: usize,
    },
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadName { path, name, why } => write!(f, "{path}: name {name:?} {why}"),
            Self::BadDescription { path, len } => {
                write!(f, "{path}: description is {len} characters (must be 1-{MAX_DESCRIPTION})")
            }
            Self::NotAllowed { path, what } => write!(f, "{path}: {what} is not allowed here"),
            Self::TooMany { path, what, count, max } => write!(f, "{path}: {count} {what} (max {max})"),
            Self::Duplicate { path, name } => write!(f, "{path}: {name:?} is used more than once"),
            Self::RequiredAfterOptional { path } => {
                write!(f, "{path}: required options must come before optional ones")
            }
            Self::Conflict { path, why } => write!(f, "{path}: {why}"),
            Self::TooLarge { total } => write!(
                f,
                "command is {total} characters in total (names, descriptions and choice values; max {MAX_TOTAL_CHARS})"
            ),
        }
    }
}

impl std::error::Error for CommandError {}

// ---------- naming ----------

/// Letters, numbers, `-`, `_`, U+02BC, plus the Devanagari and Thai blocks.
/// Slightly permissive for non-Latin scripts (Discord is authoritative): it
/// never rejects a name Discord's regex accepts.
fn is_allowed_chat_char(c: char) -> bool {
    c == '-'
        || c == '_'
        || c == '\u{02BC}'
        || c.is_alphabetic()
        || c.is_numeric()
        || ('\u{0900}'..='\u{097F}').contains(&c)
        || ('\u{0E00}'..='\u{0E7F}').contains(&c)
}

/// "If there is a lowercase variant of any letters used, you must use those."
fn has_lowercase_variant(c: char) -> bool {
    let mut lower = c.to_lowercase();
    !(lower.next() == Some(c) && lower.next().is_none())
}

fn validate_name(path: &str, name: &str, chat_input_rules: bool) -> Result<(), CommandError> {
    let bad = |why: &'static str| CommandError::BadName {
        path: path.to_owned(),
        name: name.to_owned(),
        why,
    };
    let len = name.chars().count();
    if !(1..=MAX_NAME).contains(&len) {
        return Err(bad("must be 1-32 characters"));
    }
    if chat_input_rules {
        for c in name.chars() {
            if !is_allowed_chat_char(c) {
                return Err(bad("may only contain letters, numbers, '-' and '_'"));
            }
            if has_lowercase_variant(c) {
                return Err(bad("must use lowercase letters"));
            }
        }
    }
    Ok(())
}

// ---------- choices & options ----------

/// A fixed choice for a string, integer or number option. If you give choices,
/// they are the **only** valid values.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Choice {
    name: String,
    value: Value,
}

impl Choice {
    /// String choice (value up to 100 characters).
    pub fn string(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self { name: name.into(), value: Value::String(value.into()) }
    }
    /// Integer choice.
    pub fn int(name: impl Into<String>, value: i64) -> Self {
        Self { name: name.into(), value: Value::from(value) }
    }
    /// Number choice (must be finite).
    pub fn number(name: impl Into<String>, value: f64) -> Self {
        Self { name: name.into(), value: Value::from(value) }
    }
}

/// One option (parameter, subcommand or subcommand group) of a slash command.
///
/// Rules: required options come first; names are unique per array; a group
/// contains only subcommands, a subcommand only value options.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct OptionSpec {
    #[serde(rename = "type")]
    kind: OptionType,
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    choices: Vec<Choice>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    options: Vec<OptionSpec>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    channel_types: Vec<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_value: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_value: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    autocomplete: Option<bool>,
}

macro_rules! option_ctor {
    ($($fn_name:ident => $kind:ident),* $(,)?) => {
        $(
            #[doc = concat!("A `", stringify!($kind), "` option.")]
            pub fn $fn_name(name: impl Into<String>, description: impl Into<String>) -> Self {
                Self::new(OptionType::$kind, name, description)
            }
        )*
    };
}

impl OptionSpec {
    fn new(kind: OptionType, name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
            description: description.into(),
            required: None,
            choices: Vec::new(),
            options: Vec::new(),
            channel_types: Vec::new(),
            min_value: None,
            max_value: None,
            min_length: None,
            max_length: None,
            autocomplete: None,
        }
    }

    option_ctor! {
        subcommand => SubCommand,
        subcommand_group => SubCommandGroup,
        string => String,
        integer => Integer,
        boolean => Boolean,
        user => User,
        channel => Channel,
        role => Role,
        mentionable => Mentionable,
        number => Number,
        attachment => Attachment,
    }

    /// Mark as required (not valid on subcommands and groups).
    pub fn required(mut self) -> Self {
        self.required = Some(true);
        self
    }
    /// Add a fixed choice (string, integer, number options; max 25).
    pub fn choice(mut self, c: Choice) -> Self {
        self.choices.push(c);
        self
    }
    /// Suggest values dynamically via autocomplete interactions (not with choices).
    pub fn autocomplete(mut self) -> Self {
        self.autocomplete = Some(true);
        self
    }
    /// Nest an option (subcommands and groups only).
    pub fn option(mut self, o: OptionSpec) -> Self {
        self.options.push(o);
        self
    }
    /// Restrict a channel option to these channel type integers.
    pub fn channel_types(mut self, types: Vec<u8>) -> Self {
        self.channel_types = types;
        self
    }
    /// Minimum for an `INTEGER` option.
    pub fn min_int(mut self, v: i64) -> Self {
        self.min_value = Some(Value::from(v));
        self
    }
    /// Maximum for an `INTEGER` option.
    pub fn max_int(mut self, v: i64) -> Self {
        self.max_value = Some(Value::from(v));
        self
    }
    /// Minimum for a `NUMBER` option.
    pub fn min_number(mut self, v: f64) -> Self {
        self.min_value = Some(Value::from(v));
        self
    }
    /// Maximum for a `NUMBER` option.
    pub fn max_number(mut self, v: f64) -> Self {
        self.max_value = Some(Value::from(v));
        self
    }
    /// Minimum length for a `STRING` option (0-6000).
    pub fn min_length(mut self, v: u32) -> Self {
        self.min_length = Some(v);
        self
    }
    /// Maximum length for a `STRING` option (1-6000).
    pub fn max_length(mut self, v: u32) -> Self {
        self.max_length = Some(v);
        self
    }
}

// ---------- the command ----------

/// A command to register.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct CommandSpec {
    #[serde(rename = "type")]
    kind: CommandType,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    options: Vec<OptionSpec>,
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_permissions")]
    default_member_permissions: Option<Permissions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nsfw: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    contexts: Vec<InteractionContext>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    integration_types: Vec<IntegrationType>,
}

impl CommandSpec {
    fn base(kind: CommandType, name: impl Into<String>, description: Option<String>) -> Self {
        Self {
            kind,
            name: name.into(),
            description,
            options: Vec::new(),
            default_member_permissions: None,
            nsfw: None,
            contexts: Vec::new(),
            integration_types: Vec::new(),
        }
    }

    /// A slash command. `name` must be lowercase (see the naming rules);
    /// `description` is 1-100 characters.
    pub fn slash(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self::base(CommandType::ChatInput, name, Some(description.into()))
    }

    /// A user context-menu command (no description, no options; the name may
    /// be mixed case and contain spaces).
    pub fn user(name: impl Into<String>) -> Self {
        Self::base(CommandType::User, name, None)
    }

    /// A message context-menu command (same rules as user commands).
    pub fn message(name: impl Into<String>) -> Self {
        Self::base(CommandType::Message, name, None)
    }

    /// Add an option (slash commands only; max 25).
    pub fn option(mut self, o: OptionSpec) -> Self {
        self.options.push(o);
        self
    }

    /// Who can use it by default. `Permissions::NONE` means admins only.
    pub fn default_member_permissions(mut self, p: Permissions) -> Self {
        self.default_member_permissions = Some(p);
        self
    }

    /// Age-restrict the command.
    pub fn nsfw(mut self, v: bool) -> Self {
        self.nsfw = Some(v);
        self
    }

    /// Where it can be used (global commands).
    pub fn contexts(mut self, c: Vec<InteractionContext>) -> Self {
        self.contexts = c;
        self
    }

    /// Installation contexts (global commands).
    pub fn integration_types(mut self, t: Vec<IntegrationType>) -> Self {
        self.integration_types = t;
        self
    }

    /// Check everything the docs let us check without a network call.
    pub fn validate(&self) -> Result<(), CommandError> {
        let chat = self.kind == CommandType::ChatInput;
        validate_name(&self.name, &self.name, chat)?;
        let mut total = self.name.chars().count();

        match self.kind {
            CommandType::ChatInput => {
                let len = self.description.as_deref().unwrap_or("").chars().count();
                if !(1..=MAX_DESCRIPTION).contains(&len) {
                    return Err(CommandError::BadDescription { path: self.name.clone(), len });
                }
                total += len;
            }
            CommandType::User | CommandType::Message => {
                if self.description.is_some() {
                    return Err(CommandError::NotAllowed { path: self.name.clone(), what: "description" });
                }
                if !self.options.is_empty() {
                    return Err(CommandError::NotAllowed { path: self.name.clone(), what: "options" });
                }
            }
        }

        validate_options(&self.name, &self.options, None, &mut total)?;
        if total > MAX_TOTAL_CHARS {
            return Err(CommandError::TooLarge { total });
        }
        Ok(())
    }
}

fn validate_options(
    path: &str,
    options: &[OptionSpec],
    parent: Option<OptionType>,
    total: &mut usize,
) -> Result<(), CommandError> {
    if options.len() > MAX_OPTIONS {
        return Err(CommandError::TooMany {
            path: path.to_owned(),
            what: "options",
            count: options.len(),
            max: MAX_OPTIONS,
        });
    }

    let mut seen: HashSet<&str> = HashSet::new();
    let mut seen_optional = false;

    for o in options {
        let here = format!("{path}/{}", o.name);
        validate_name(&here, &o.name, true)?;
        if !seen.insert(o.name.as_str()) {
            return Err(CommandError::Duplicate { path: path.to_owned(), name: o.name.clone() });
        }

        let dlen = o.description.chars().count();
        if !(1..=MAX_DESCRIPTION).contains(&dlen) {
            return Err(CommandError::BadDescription { path: here, len: dlen });
        }
        *total += o.name.chars().count() + dlen;

        // Nesting: command -> (group ->) subcommand -> value options.
        match parent {
            Some(OptionType::SubCommandGroup) if o.kind != OptionType::SubCommand => {
                return Err(CommandError::Conflict {
                    path: here,
                    why: "a subcommand group may only contain subcommands",
                });
            }
            Some(OptionType::SubCommand) if o.kind.is_sub() => {
                return Err(CommandError::Conflict {
                    path: here,
                    why: "a subcommand can't contain subcommands or groups",
                });
            }
            _ => {}
        }

        if o.kind.is_sub() {
            if o.required.is_some() {
                return Err(CommandError::NotAllowed { path: here, what: "required" });
            }
            validate_option_fields(&here, o, total)?;
            validate_options(&here, &o.options, Some(o.kind), total)?;
        } else {
            if !o.options.is_empty() {
                return Err(CommandError::NotAllowed { path: here, what: "nested options" });
            }
            if o.required == Some(true) {
                if seen_optional {
                    return Err(CommandError::RequiredAfterOptional { path: here });
                }
            } else {
                seen_optional = true;
            }
            validate_option_fields(&here, o, total)?;
        }
    }
    Ok(())
}

fn validate_option_fields(path: &str, o: &OptionSpec, total: &mut usize) -> Result<(), CommandError> {
    let not_allowed = |what: &'static str| CommandError::NotAllowed { path: path.to_owned(), what };
    let conflict = |why: &'static str| CommandError::Conflict { path: path.to_owned(), why };

    if !o.choices.is_empty() {
        if !o.kind.takes_choices() {
            return Err(not_allowed("choices"));
        }
        if o.choices.len() > MAX_CHOICES {
            return Err(CommandError::TooMany {
                path: path.to_owned(),
                what: "choices",
                count: o.choices.len(),
                max: MAX_CHOICES,
            });
        }
        for c in &o.choices {
            let nlen = c.name.chars().count();
            if !(1..=MAX_CHOICE_NAME).contains(&nlen) {
                return Err(conflict("choice names must be 1-100 characters"));
            }
            *total += nlen;
            match o.kind {
                OptionType::String => {
                    let Some(v) = c.value.as_str() else {
                        return Err(conflict("choices for a string option need string values"));
                    };
                    let vlen = v.chars().count();
                    if vlen > MAX_CHOICE_STRING {
                        return Err(conflict("string choice values are at most 100 characters"));
                    }
                    *total += vlen;
                }
                OptionType::Integer => match c.value.as_i64() {
                    Some(v) if (-MAX_SAFE_INT..=MAX_SAFE_INT).contains(&v) => {}
                    _ => return Err(conflict("choices for an integer option need integer values within ±(2^53-1)")),
                },
                _ => match c.value.as_f64() {
                    Some(v) if v.is_finite() && v.abs() <= MAX_NUMBER => {}
                    _ => return Err(conflict("choices for a number option need finite values within ±2^53")),
                },
            }
        }
    }

    if o.autocomplete == Some(true) {
        if !o.kind.takes_choices() {
            return Err(not_allowed("autocomplete"));
        }
        if !o.choices.is_empty() {
            return Err(conflict("autocomplete can't be combined with choices"));
        }
    }

    if !o.channel_types.is_empty() && o.kind != OptionType::Channel {
        return Err(not_allowed("channel_types"));
    }

    if o.min_value.is_some() || o.max_value.is_some() {
        match o.kind {
            OptionType::Integer => {
                for v in [&o.min_value, &o.max_value].into_iter().flatten() {
                    match v.as_i64() {
                        Some(n) if (-MAX_SAFE_INT..=MAX_SAFE_INT).contains(&n) => {}
                        _ => return Err(conflict("INTEGER min/max must be integers within ±(2^53-1)")),
                    }
                }
            }
            OptionType::Number => {
                for v in [&o.min_value, &o.max_value].into_iter().flatten() {
                    match v.as_f64() {
                        Some(n) if n.is_finite() && n.abs() <= MAX_NUMBER => {}
                        _ => return Err(conflict("NUMBER min/max must be finite and within ±2^53")),
                    }
                }
            }
            _ => return Err(not_allowed("min_value/max_value")),
        }
        if let (Some(lo), Some(hi)) = (
            o.min_value.as_ref().and_then(Value::as_f64),
            o.max_value.as_ref().and_then(Value::as_f64),
        ) {
            if lo > hi {
                return Err(conflict("min_value is greater than max_value"));
            }
        }
    }

    if o.min_length.is_some() || o.max_length.is_some() {
        if o.kind != OptionType::String {
            return Err(not_allowed("min_length/max_length"));
        }
        if o.min_length.is_some_and(|v| v > MAX_STRING_LENGTH) {
            return Err(conflict("min_length is at most 6000"));
        }
        if o.max_length.is_some_and(|v| !(1..=MAX_STRING_LENGTH).contains(&v)) {
            return Err(conflict("max_length must be 1-6000"));
        }
        if let (Some(lo), Some(hi)) = (o.min_length, o.max_length) {
            if lo > hi {
                return Err(conflict("min_length is greater than max_length"));
            }
        }
    }
    Ok(())
}

/// Validate a whole set (what bulk overwrite sends): each command, unique
/// `(type, name)` pairs, and the per-scope limits of 100 slash / 15 user /
/// 15 message commands.
pub fn validate_commands(cmds: &[CommandSpec]) -> Result<(), CommandError> {
    let mut seen: HashSet<(CommandType, &str)> = HashSet::new();
    let (mut slash, mut user, mut message) = (0usize, 0usize, 0usize);
    for c in cmds {
        c.validate()?;
        if !seen.insert((c.kind, c.name.as_str())) {
            return Err(CommandError::Duplicate { path: "commands".into(), name: c.name.clone() });
        }
        match c.kind {
            CommandType::ChatInput => slash += 1,
            CommandType::User => user += 1,
            CommandType::Message => message += 1,
        }
    }
    let check = |count: usize, max: usize, what: &'static str| {
        if count > max {
            Err(CommandError::TooMany { path: "commands".into(), what, count, max })
        } else {
            Ok(())
        }
    };
    check(slash, MAX_SLASH_COMMANDS, "slash commands")?;
    check(user, MAX_USER_COMMANDS, "user commands")?;
    check(message, MAX_MESSAGE_COMMANDS, "message commands")
}

fn validate_guild_scope(cmds: &[CommandSpec]) -> Result<(), CommandError> {
    for c in cmds {
        if c.contexts.contains(&InteractionContext::BotDm) {
            return Err(CommandError::Conflict {
                path: c.name.clone(),
                why: "guild commands don't support the BOT_DM context",
            });
        }
    }
    Ok(())
}

// ---------- API ----------

/// A command as Discord stores it.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct RegisteredCommand {
    /// Command id.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// Command name.
    pub name: String,
    /// Command type (`None` means the default, 1 = slash command).
    #[serde(rename = "type", default)]
    pub kind: Option<u8>,
    /// Description (empty for user and message commands).
    #[serde(default)]
    pub description: String,
    /// The guild, for guild commands.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub guild_id: Option<Snowflake>,
}

impl<T: Transport> RestClient<T> {
    /// `PUT /applications/{app}/commands`: make this exact list your global
    /// commands (it **replaces all types**: slash, user and message).
    ///
    /// Idempotent: commands that already exist don't count toward the daily
    /// create limit, so it's safe to call on every startup.
    pub async fn bulk_overwrite_global_commands(
        &self,
        application_id: Snowflake,
        commands: &[CommandSpec],
    ) -> Result<Vec<RegisteredCommand>, Error> {
        validate_commands(commands)?;
        let body = serde_json::to_vec(commands).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/applications/{application_id}/commands");
        self.request("PUT", &path, Some(body)).await?.expect_success()?.json()
    }

    /// `PUT /applications/{app}/guilds/{guild}/commands`: same, for one guild.
    /// Guild commands update instantly, which makes them good for development.
    pub async fn bulk_overwrite_guild_commands(
        &self,
        application_id: Snowflake,
        guild_id: Snowflake,
        commands: &[CommandSpec],
    ) -> Result<Vec<RegisteredCommand>, Error> {
        validate_commands(commands)?;
        validate_guild_scope(commands)?;
        let body = serde_json::to_vec(commands).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/applications/{application_id}/guilds/{guild_id}/commands");
        self.request("PUT", &path, Some(body)).await?.expect_success()?.json()
    }

    /// `POST /applications/{app}/commands`: create **or overwrite** one global
    /// command with the same name and type.
    pub async fn create_global_command(
        &self,
        application_id: Snowflake,
        command: &CommandSpec,
    ) -> Result<RegisteredCommand, Error> {
        command.validate()?;
        let body = serde_json::to_vec(command).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/applications/{application_id}/commands");
        self.request("POST", &path, Some(body)).await?.expect_success()?.json()
    }

    /// `POST /applications/{app}/guilds/{guild}/commands`: create or overwrite
    /// one guild command.
    pub async fn create_guild_command(
        &self,
        application_id: Snowflake,
        guild_id: Snowflake,
        command: &CommandSpec,
    ) -> Result<RegisteredCommand, Error> {
        command.validate()?;
        validate_guild_scope(std::slice::from_ref(command))?;
        let body = serde_json::to_vec(command).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/applications/{application_id}/guilds/{guild_id}/commands");
        self.request("POST", &path, Some(body)).await?.expect_success()?.json()
    }

    /// `GET /applications/{app}/commands`.
    pub async fn get_global_commands(&self, application_id: Snowflake) -> Result<Vec<RegisteredCommand>, Error> {
        let path = format!("/applications/{application_id}/commands");
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `GET /applications/{app}/guilds/{guild}/commands`.
    pub async fn get_guild_commands(
        &self,
        application_id: Snowflake,
        guild_id: Snowflake,
    ) -> Result<Vec<RegisteredCommand>, Error> {
        let path = format!("/applications/{application_id}/guilds/{guild_id}/commands");
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `DELETE /applications/{app}/commands/{id}`.
    pub async fn delete_global_command(&self, application_id: Snowflake, command_id: Snowflake) -> Result<(), Error> {
        let path = format!("/applications/{application_id}/commands/{command_id}");
        self.request("DELETE", &path, None).await?.expect_success()?;
        Ok(())
    }

    /// `DELETE /applications/{app}/guilds/{guild}/commands/{id}`.
    pub async fn delete_guild_command(
        &self,
        application_id: Snowflake,
        guild_id: Snowflake,
        command_id: Snowflake,
    ) -> Result<(), Error> {
        let path = format!("/applications/{application_id}/guilds/{guild_id}/commands/{command_id}");
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

    fn opt(name: &str) -> OptionSpec {
        OptionSpec::string(name, "d")
    }

    fn slash_with(options: Vec<OptionSpec>) -> CommandSpec {
        options.into_iter().fold(CommandSpec::slash("cmd", "d"), CommandSpec::option)
    }

    // ----- serialization -----

    #[test]
    fn serializes_the_docs_example_command() {
        let c = CommandSpec::slash("blep", "Send a random adorable animal photo")
            .option(
                OptionSpec::string("animal", "The type of animal")
                    .required()
                    .choice(Choice::string("Dog", "animal_dog"))
                    .choice(Choice::string("Cat", "animal_cat"))
                    .choice(Choice::string("Penguin", "animal_penguin")),
            )
            .option(OptionSpec::boolean("only_smol", "Whether to show only baby animals"));
        assert!(c.validate().is_ok());
        assert_eq!(
            serde_json::to_value(&c).unwrap(),
            json!({
                "name": "blep", "type": 1, "description": "Send a random adorable animal photo",
                "options": [
                    { "name": "animal", "description": "The type of animal", "type": 3, "required": true,
                      "choices": [
                        { "name": "Dog", "value": "animal_dog" },
                        { "name": "Cat", "value": "animal_cat" },
                        { "name": "Penguin", "value": "animal_penguin" } ] },
                    { "name": "only_smol", "description": "Whether to show only baby animals", "type": 5 }
                ]
            })
        );
    }

    #[test]
    fn user_and_message_commands_have_no_description() {
        let u = CommandSpec::user("View Stats");
        assert!(u.validate().is_ok()); // mixed case and spaces are fine here
        assert_eq!(serde_json::to_value(&u).unwrap(), json!({ "name": "View Stats", "type": 2 }));
        assert_eq!(
            serde_json::to_value(CommandSpec::message("Challenge")).unwrap(),
            json!({ "name": "Challenge", "type": 3 })
        );

        let mut bad = CommandSpec::user("x");
        bad.description = Some("nope".into());
        assert!(matches!(bad.validate(), Err(CommandError::NotAllowed { what: "description", .. })));
        let with_opts = CommandSpec::user("x").option(opt("a"));
        assert!(matches!(with_opts.validate(), Err(CommandError::NotAllowed { what: "options", .. })));
    }

    #[test]
    fn permissions_serialize_as_decimal_strings_and_none_means_admins_only() {
        let admins = CommandSpec::slash("a", "d").default_member_permissions(Permissions::NONE);
        assert_eq!(serde_json::to_value(&admins).unwrap()["default_member_permissions"], "0");
        let guild_mgr = CommandSpec::slash("a", "d").default_member_permissions(Permissions::MANAGE_GUILD);
        assert_eq!(serde_json::to_value(&guild_mgr).unwrap()["default_member_permissions"], "32");
    }

    #[test]
    fn contexts_and_install_types_serialize_as_integers() {
        let c = CommandSpec::slash("a", "d")
            .contexts(vec![InteractionContext::Guild, InteractionContext::BotDm])
            .integration_types(vec![IntegrationType::GuildInstall, IntegrationType::UserInstall])
            .nsfw(true);
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["contexts"], json!([0, 1]));
        assert_eq!(v["integration_types"], json!([0, 1]));
        assert_eq!(v["nsfw"], true);
    }

    // ----- naming -----

    #[test]
    fn valid_slash_names_including_other_scripts() {
        for n in ["ping", "my-cmd_2", "café", "नमस्ते", "สวัสดี", "日本語", "a"] {
            assert!(CommandSpec::slash(n, "d").validate().is_ok(), "{n}");
        }
        assert!(CommandSpec::slash("a".repeat(32), "d").validate().is_ok());
    }

    #[test]
    fn invalid_slash_names_are_rejected_with_a_reason() {
        let why = |n: &str| match CommandSpec::slash(n, "d").validate() {
            Err(CommandError::BadName { why, .. }) => why,
            other => panic!("{n:?}: {other:?}"),
        };
        assert_eq!(why("Ping"), "must use lowercase letters");
        assert_eq!(why("my cmd"), "may only contain letters, numbers, '-' and '_'");
        assert_eq!(why("a!"), "may only contain letters, numbers, '-' and '_'");
        assert_eq!(why("🙂"), "may only contain letters, numbers, '-' and '_'");
        assert_eq!(why(""), "must be 1-32 characters");
        assert_eq!(why(&"a".repeat(33)), "must be 1-32 characters");
    }

    #[test]
    fn option_names_follow_the_same_rules() {
        let c = slash_with(vec![OptionSpec::string("Text", "d")]);
        assert!(matches!(c.validate(), Err(CommandError::BadName { .. })));
    }

    #[test]
    fn descriptions_are_1_to_100_characters() {
        assert!(CommandSpec::slash("a", "x".repeat(100)).validate().is_ok());
        assert!(matches!(
            CommandSpec::slash("a", "").validate(),
            Err(CommandError::BadDescription { len: 0, .. })
        ));
        assert!(matches!(
            CommandSpec::slash("a", "x".repeat(101)).validate(),
            Err(CommandError::BadDescription { len: 101, .. })
        ));
        let c = slash_with(vec![OptionSpec::string("o", "")]);
        assert!(matches!(c.validate(), Err(CommandError::BadDescription { len: 0, .. })));
    }

    // ----- options -----

    #[test]
    fn at_most_25_unique_options() {
        let many: Vec<_> = (0..26).map(|i| opt(&format!("o{i}"))).collect();
        assert!(matches!(slash_with(many).validate(), Err(CommandError::TooMany { count: 26, max: 25, .. })));
        let ok: Vec<_> = (0..25).map(|i| opt(&format!("o{i}"))).collect();
        assert!(slash_with(ok).validate().is_ok());

        let dup = slash_with(vec![opt("same"), opt("same")]);
        assert!(matches!(dup.validate(), Err(CommandError::Duplicate { .. })));
    }

    #[test]
    fn required_options_must_come_first() {
        let bad = slash_with(vec![OptionSpec::boolean("a", "d"), opt("b").required()]);
        assert!(matches!(bad.validate(), Err(CommandError::RequiredAfterOptional { .. })));
        let ok = slash_with(vec![opt("a").required(), opt("b").required(), OptionSpec::boolean("c", "d")]);
        assert!(ok.validate().is_ok());
    }

    #[test]
    fn fields_are_only_allowed_on_the_option_types_that_support_them() {
        let na = |o: OptionSpec, what: &str| match slash_with(vec![o]).validate() {
            Err(CommandError::NotAllowed { what: w, .. }) => assert_eq!(w, what),
            other => panic!("{what}: {other:?}"),
        };
        na(OptionSpec::boolean("b", "d").choice(Choice::string("x", "y")), "choices");
        na(OptionSpec::boolean("b", "d").autocomplete(), "autocomplete");
        na(opt("s").channel_types(vec![0]), "channel_types");
        na(opt("s").min_int(1), "min_value/max_value");
        na(OptionSpec::integer("i", "d").min_length(1), "min_length/max_length");
        na(OptionSpec::subcommand("sub", "d").required(), "required");
    }

    #[test]
    fn choices_follow_their_option_type_and_limits() {
        let conflict = |o: OptionSpec| matches!(slash_with(vec![o]).validate(), Err(CommandError::Conflict { .. }));
        assert!(conflict(opt("s").choice(Choice::int("one", 1)))); // int value on a string option
        assert!(conflict(OptionSpec::integer("i", "d").choice(Choice::string("one", "1"))));
        assert!(conflict(opt("s").choice(Choice::string("n", "v".repeat(101)))));
        assert!(conflict(opt("s").choice(Choice::string("", "v"))));
        assert!(conflict(OptionSpec::number("n", "d").choice(Choice::number("nan", f64::NAN))));
        assert!(conflict(opt("s").autocomplete().choice(Choice::string("a", "b"))));

        let many = (0..26).fold(opt("s"), |o, i| o.choice(Choice::string(format!("c{i}"), "v")));
        assert!(matches!(slash_with(vec![many]).validate(), Err(CommandError::TooMany { what: "choices", .. })));
        let ok = (0..25).fold(opt("s"), |o, i| o.choice(Choice::string(format!("c{i}"), "v")));
        assert!(slash_with(vec![ok]).validate().is_ok());
    }

    #[test]
    fn extreme_integers_are_rejected_instead_of_overflowing() {
        // i64::MIN.abs() overflows; the range check must not use it.
        let min_choice = OptionSpec::integer("i", "d").choice(Choice::int("min", i64::MIN));
        assert!(matches!(slash_with(vec![min_choice]).validate(), Err(CommandError::Conflict { .. })));
        let min_bound = OptionSpec::integer("i", "d").min_int(i64::MIN);
        assert!(matches!(slash_with(vec![min_bound]).validate(), Err(CommandError::Conflict { .. })));
        // The documented boundary values are accepted.
        let edge = OptionSpec::integer("i", "d").min_int(-9_007_199_254_740_991).max_int(9_007_199_254_740_991);
        assert!(slash_with(vec![edge]).validate().is_ok());
    }

    #[test]
    fn ranges_and_lengths_are_checked() {
        let bad = |o: OptionSpec| matches!(slash_with(vec![o]).validate(), Err(CommandError::Conflict { .. }));
        assert!(bad(OptionSpec::integer("i", "d").min_int(5).max_int(1)));
        assert!(bad(OptionSpec::integer("i", "d").max_int(1 << 60)));
        assert!(bad(OptionSpec::integer("i", "d").min_number(1.5))); // a float on an INTEGER option
        assert!(bad(OptionSpec::number("n", "d").min_number(2.0).max_number(1.0)));
        assert!(bad(opt("s").min_length(5).max_length(1)));
        assert!(bad(opt("s").max_length(0)));
        assert!(bad(opt("s").max_length(6001)));

        let ok = slash_with(vec![
            OptionSpec::integer("i", "d").min_int(-5).max_int(5),
            OptionSpec::number("n", "d").min_number(0.5).max_number(1.5),
            opt("s").min_length(0).max_length(6000),
        ]);
        assert!(ok.validate().is_ok());
    }

    // ----- nesting & size -----

    fn permissions_example() -> CommandSpec {
        let sub = |name: &str, desc: &str, who: OptionSpec| {
            OptionSpec::subcommand(name, desc)
                .option(who.required())
                .option(OptionSpec::channel("channel", "The channel permissions. If omitted, the guild's"))
        };
        CommandSpec::slash("permissions", "Get or edit permissions for a user or a role")
            .option(
                OptionSpec::subcommand_group("user", "Get or edit permissions for a user")
                    .option(sub("get", "Get permissions for a user", OptionSpec::user("user", "The user")))
                    .option(sub("edit", "Edit permissions for a user", OptionSpec::user("user", "The user"))),
            )
            .option(
                OptionSpec::subcommand_group("role", "Get or edit permissions for a role")
                    .option(sub("get", "Get permissions for a role", OptionSpec::role("role", "The role")))
                    .option(sub("edit", "Edit permissions for a role", OptionSpec::role("role", "The role"))),
            )
    }

    #[test]
    fn the_docs_permissions_example_validates_and_nests() {
        let c = permissions_example();
        assert!(c.validate().is_ok());
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["options"][0]["type"], 2); // SUB_COMMAND_GROUP
        assert_eq!(v["options"][0]["options"][0]["type"], 1); // SUB_COMMAND
        assert_eq!(
            v["options"][0]["options"][0]["options"][0],
            json!({ "name": "user", "description": "The user", "type": 6, "required": true })
        );
    }

    #[test]
    fn nesting_rules() {
        // A group may only contain subcommands.
        let g = slash_with(vec![OptionSpec::subcommand_group("g", "d").option(opt("x"))]);
        assert!(matches!(g.validate(), Err(CommandError::Conflict { .. })));
        // A subcommand can't contain a subcommand or a group.
        let s = slash_with(vec![OptionSpec::subcommand("s", "d").option(OptionSpec::subcommand("t", "d"))]);
        assert!(matches!(s.validate(), Err(CommandError::Conflict { .. })));
        let sg = slash_with(vec![OptionSpec::subcommand("s", "d").option(OptionSpec::subcommand_group("t", "d"))]);
        assert!(matches!(sg.validate(), Err(CommandError::Conflict { .. })));
        // Value options can't have children.
        let v = slash_with(vec![opt("x").option(opt("y"))]);
        assert!(matches!(v.validate(), Err(CommandError::NotAllowed { what: "nested options", .. })));
    }

    #[test]
    fn a_command_over_8000_characters_is_rejected() {
        let subs = (0..25).map(|i| {
            (0..25).fold(OptionSpec::subcommand(format!("s{i}"), "d"), |s, j| {
                s.option(OptionSpec::string(format!("o{j}"), "x".repeat(100)))
            })
        });
        let big = subs.fold(CommandSpec::slash("big", "d"), CommandSpec::option);
        assert!(matches!(big.validate(), Err(CommandError::TooLarge { total }) if total > 8000));
    }

    // ----- sets & scopes -----

    #[test]
    fn command_sets_enforce_uniqueness_and_per_type_limits() {
        let many: Vec<_> = (0..101).map(|i| CommandSpec::slash(format!("c{i}"), "d")).collect();
        assert!(matches!(
            validate_commands(&many),
            Err(CommandError::TooMany { what: "slash commands", count: 101, max: 100, .. })
        ));
        assert!(validate_commands(&many[..100]).is_ok());

        let users: Vec<_> = (0..16).map(|i| CommandSpec::user(format!("U{i}"))).collect();
        assert!(matches!(validate_commands(&users), Err(CommandError::TooMany { what: "user commands", .. })));

        // Same name twice in one type is a duplicate; across types it is fine.
        let dup = [CommandSpec::slash("x", "d"), CommandSpec::slash("x", "d")];
        assert!(matches!(validate_commands(&dup), Err(CommandError::Duplicate { .. })));
        let mixed = [CommandSpec::slash("x", "d"), CommandSpec::user("x")];
        assert!(validate_commands(&mixed).is_ok());
    }

    // ----- API -----

    #[tokio::test(start_paused = true)]
    async fn bulk_overwrite_global_sends_a_put_with_the_array() {
        let reply = r#"[{"id":"1","application_id":"20","name":"ping","type":1,"description":"d","version":"9"}]"#;
        let c = client(vec![resp(200, reply)]);
        let out = c
            .bulk_overwrite_global_commands(sf(20), &[CommandSpec::slash("ping", "d")])
            .await
            .unwrap();
        assert_eq!((out[0].id, out[0].name.as_str(), out[0].kind), (sf(1), "ping", Some(1)));

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PUT");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/applications/20/commands");
        let body: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(body, json!([{ "name": "ping", "type": 1, "description": "d" }]));
    }

    #[tokio::test(start_paused = true)]
    async fn guild_routes_and_the_bot_dm_restriction() {
        let c = client(vec![resp(200, "[]"), resp(201, r#"{"id":"5","name":"x","type":1,"description":"d"}"#)]);
        c.bulk_overwrite_guild_commands(sf(20), sf(30), &[CommandSpec::slash("x", "d")]).await.unwrap();
        let created = c.create_guild_command(sf(20), sf(30), &CommandSpec::slash("x", "d")).await.unwrap();
        assert_eq!(created.id, sf(5)); // 201 counts as success

        {
            let seen = c.transport().seen.lock().unwrap();
            assert_eq!(seen[0].url, "https://discord.com/api/v10/applications/20/guilds/30/commands");
            assert_eq!((seen[1].method.as_str(), seen[1].url.as_str()),
                ("POST", "https://discord.com/api/v10/applications/20/guilds/30/commands"));
        }

        // Guild commands can't use the BOT_DM context: rejected without a request.
        let dm = CommandSpec::slash("x", "d").contexts(vec![InteractionContext::BotDm]);
        let r = c.create_guild_command(sf(20), sf(30), &dm).await;
        assert!(matches!(r, Err(Error::InvalidCommand(CommandError::Conflict { .. }))));
        assert_eq!(c.transport().seen.lock().unwrap().len(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn invalid_commands_are_never_sent() {
        let c = client(vec![]);
        let r = c.bulk_overwrite_global_commands(sf(20), &[CommandSpec::slash("Bad Name", "d")]).await;
        assert!(matches!(r, Err(Error::InvalidCommand(CommandError::BadName { .. }))));
        assert!(c.transport().seen.lock().unwrap().is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn list_and_delete_use_the_documented_routes() {
        let c = client(vec![resp(200, "[]"), resp(200, "[]"), resp(204, ""), resp(204, "")]);
        c.get_global_commands(sf(20)).await.unwrap();
        c.get_guild_commands(sf(20), sf(30)).await.unwrap();
        c.delete_global_command(sf(20), sf(5)).await.unwrap();
        c.delete_guild_command(sf(20), sf(30), sf(5)).await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        let routes: Vec<(&str, &str)> = seen.iter().map(|r| (r.method.as_str(), r.url.as_str())).collect();
        assert_eq!(
            routes,
            vec![
                ("GET", "https://discord.com/api/v10/applications/20/commands"),
                ("GET", "https://discord.com/api/v10/applications/20/guilds/30/commands"),
                ("DELETE", "https://discord.com/api/v10/applications/20/commands/5"),
                ("DELETE", "https://discord.com/api/v10/applications/20/guilds/30/commands/5"),
            ]
        );
    }

    #[test]
    fn error_messages_say_where_and_why() {
        let e = CommandSpec::slash("ok", "d")
            .option(OptionSpec::boolean("flag", "d").choice(Choice::string("a", "b")))
            .validate()
            .unwrap_err();
        assert_eq!(e.to_string(), "ok/flag: choices is not allowed here");
    }
}
