//! Typed Discord message components (both V1 and Components V2) with local validation.
//!
//! Every limit and constraint here conforms to Discord's official Component Specifications.
//! All validation errors are caught locally before any network calls are dispatched.

use crate::message::{
    de_opt_snowflake, de_snowflake, ser_opt_snowflake, ser_snowflake,
};
use rift_core::Snowflake;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

// ---------- Constants for Discord Limits ----------

/// Maximum characters in a custom ID (100).
pub const MAX_CUSTOM_ID_LEN: usize = 100;
/// Maximum characters in a button label (80).
pub const MAX_BUTTON_LABEL_LEN: usize = 80;
/// Maximum buttons in an ActionRow (5).
pub const MAX_BUTTONS_PER_ROW: usize = 5;
/// Maximum characters in a select menu placeholder (150).
pub const MAX_SELECT_PLACEHOLDER_LEN: usize = 150;
/// Maximum select options in a string select menu (25).
pub const MAX_SELECT_OPTIONS: usize = 25;
/// Maximum characters in a select option label (100).
pub const MAX_SELECT_OPTION_LABEL_LEN: usize = 100;
/// Maximum characters in a select option value (100).
pub const MAX_SELECT_OPTION_VALUE_LEN: usize = 100;
/// Maximum characters in a select option description (100).
pub const MAX_SELECT_OPTION_DESC_LEN: usize = 100;
/// Maximum `min_values` or `max_values` for a select menu (25).
pub const MAX_SELECT_VALUES: u8 = 25;
/// Maximum characters in a text input label (45).
pub const MAX_TEXT_INPUT_LABEL_LEN: usize = 45;
/// Maximum characters in a text input placeholder (100).
pub const MAX_TEXT_INPUT_PLACEHOLDER_LEN: usize = 100;
/// Maximum length bounds for a text input (4000).
pub const MAX_TEXT_INPUT_BOUND: u32 = 4000;
/// Maximum characters in text input value (4000).
pub const MAX_TEXT_INPUT_VALUE_LEN: usize = 4000;
/// Maximum characters in a TextDisplay component (4000).
pub const MAX_TEXT_DISPLAY_LEN: usize = 4000;
/// Maximum items in a MediaGallery component (10).
pub const MAX_MEDIA_GALLERY_ITEMS: usize = 10;
/// Maximum characters in media description / alt text (1024).
pub const MAX_MEDIA_DESC_LEN: usize = 1024;
/// Maximum total components when Components V2 is active (40).
pub const MAX_COMPONENTS_V2_TOTAL: usize = 40;
/// Maximum action rows in a message (5).
pub const MAX_ACTION_ROWS_PER_MESSAGE: usize = 5;

// ---------- Component Error ----------

/// Error returned when a component or hierarchy fails local validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComponentError {
    /// An ActionRow is empty (must have 1..=5 buttons, 1 select menu, or 1 text input).
    EmptyActionRow,
    /// An ActionRow exceeds the component limit.
    TooManyComponentsInActionRow {
        /// Actual component count.
        count: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// An ActionRow mixes incompatible components (e.g. buttons with a select menu or text input).
    MixedActionRowComponents,
    /// `custom_id` was missing for a component style that requires it.
    MissingCustomId,
    /// `custom_id` length is invalid.
    InvalidCustomIdLength {
        /// Actual length.
        len: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// Button label is too long.
    ButtonLabelTooLong {
        /// Actual length.
        len: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// A Link button is missing its URL.
    MissingLinkUrl,
    /// A URL was specified on a non-Link button.
    UnexpectedLinkUrl,
    /// A Premium button is missing its SKU ID.
    MissingPremiumSkuId,
    /// A SKU ID was specified on a non-Premium button.
    UnexpectedPremiumSkuId,
    /// A `custom_id` was specified on a Link or Premium button.
    UnexpectedCustomIdOnLinkOrPremium,
    /// A label was specified on a Premium button (not allowed by Discord).
    LabelNotAllowedOnPremiumButton,
    /// A button has neither a label nor an emoji.
    MissingButtonLabelOrEmoji,
    /// A select menu placeholder exceeds the character limit.
    PlaceholderTooLong {
        /// Actual length.
        len: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// `min_values` is greater than `max_values`.
    MinValuesGreaterThanMaxValues {
        /// `min_values`
        min: u8,
        /// `max_values`
        max: u8,
    },
    /// `min_values` is out of bounds (0..=25).
    InvalidMinValues {
        /// The value given.
        value: u8,
    },
    /// `max_values` is out of bounds (1..=25).
    InvalidMaxValues {
        /// The value given.
        value: u8,
    },
    /// A String select menu has an invalid number of options (must be 1..=25).
    InvalidOptionCount {
        /// Actual count.
        count: usize,
        /// Minimum allowed.
        min: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// Options were provided on a non-String select menu (User, Role, Mentionable, Channel).
    OptionsNotAllowedOnAutoPopulatedSelect,
    /// Select option label length is invalid.
    InvalidOptionLabelLength {
        /// Actual length.
        len: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// Select option value length is invalid.
    InvalidOptionValueLength {
        /// Actual length.
        len: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// Select option description is too long.
    OptionDescriptionTooLong {
        /// Actual length.
        len: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// TextInput label length is invalid (must be 1..=45).
    InvalidTextInputLabelLength {
        /// Actual length.
        len: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// TextInput placeholder is too long.
    TextInputPlaceholderTooLong {
        /// Actual length.
        len: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// TextInput `min_length` is greater than `max_length`.
    MinLengthGreaterThanMaxLength {
        /// `min_length`
        min: u32,
        /// `max_length`
        max: u32,
    },
    /// TextInput `min_length` exceeds limit (4000).
    InvalidMinLength {
        /// Value given.
        value: u32,
        /// Maximum allowed.
        max: u32,
    },
    /// TextInput `max_length` is 0 or exceeds limit (4000).
    InvalidMaxLength {
        /// Value given.
        value: u32,
        /// Maximum allowed.
        max: u32,
    },
    /// TextInput prefilled `value` is too long.
    TextInputValueTooLong {
        /// Actual length.
        len: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// TextDisplay content is empty or exceeds 4000 characters.
    InvalidTextDisplayContentLength {
        /// Actual length.
        len: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// Media description / alt text is too long.
    MediaDescriptionTooLong {
        /// Actual length.
        len: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// Media URL is empty.
    EmptyMediaUrl,
    /// MediaGallery has an invalid item count (must be 1..=10).
    InvalidMediaGalleryItemCount {
        /// Actual count.
        count: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// Section has no components or too many components.
    InvalidSectionComponentCount {
        /// Actual count.
        count: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// Container has no inner components.
    EmptyContainer,
    /// Container accent color exceeds 24-bit RGB (0xFFFFFF).
    InvalidAccentColor {
        /// Value given.
        value: u32,
    },
    /// Total components exceed the maximum limit (e.g. 40 in Components V2).
    TooManyComponents {
        /// Actual total.
        count: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// Modal title length is invalid (must be 1..=45 characters).
    InvalidModalTitleLength {
        /// Actual length.
        len: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// Modal custom ID length is invalid (must be 1..=100 characters).
    InvalidModalCustomIdLength {
        /// Actual length.
        len: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// Modal components must be 1..=5 ActionRows containing only TextInput.
    InvalidModalComponents {
        /// Detailed reason.
        reason: &'static str,
    },
    /// Autocomplete choices exceed the maximum of 25.
    TooManyAutocompleteChoices {
        /// Actual count.
        count: usize,
        /// Maximum allowed.
        max: usize,
    },
}

impl fmt::Display for ComponentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyActionRow => write!(f, "action row must not be empty"),
            Self::TooManyComponentsInActionRow { count, max } => {
                write!(f, "action row has {count} components (max {max})")
            }
            Self::MixedActionRowComponents => {
                write!(f, "action row components cannot be mixed (buttons cannot be combined with select menus or text inputs)")
            }
            Self::MissingCustomId => write!(f, "custom_id is required for this component style"),
            Self::InvalidCustomIdLength { len, max } => {
                write!(f, "custom_id is {len} characters (must be 1-{max})")
            }
            Self::ButtonLabelTooLong { len, max } => {
                write!(f, "button label is {len} characters (max {max})")
            }
            Self::MissingLinkUrl => write!(f, "link button requires a url"),
            Self::UnexpectedLinkUrl => write!(f, "url is only allowed on link buttons"),
            Self::MissingPremiumSkuId => write!(f, "premium button requires a sku_id"),
            Self::UnexpectedPremiumSkuId => write!(f, "sku_id is only allowed on premium buttons"),
            Self::UnexpectedCustomIdOnLinkOrPremium => {
                write!(f, "custom_id is not allowed on link or premium buttons")
            }
            Self::LabelNotAllowedOnPremiumButton => {
                write!(f, "label is not allowed on premium buttons")
            }
            Self::MissingButtonLabelOrEmoji => {
                write!(f, "button must have at least a label or an emoji")
            }
            Self::PlaceholderTooLong { len, max } => {
                write!(f, "placeholder is {len} characters (max {max})")
            }
            Self::MinValuesGreaterThanMaxValues { min, max } => {
                write!(f, "min_values ({min}) cannot be greater than max_values ({max})")
            }
            Self::InvalidMinValues { value } => {
                write!(f, "min_values {value} is out of bounds (0-{MAX_SELECT_VALUES})")
            }
            Self::InvalidMaxValues { value } => {
                write!(f, "max_values {value} is out of bounds (1-{MAX_SELECT_VALUES})")
            }
            Self::InvalidOptionCount { count, min, max } => {
                write!(f, "select menu has {count} options (must be {min}-{max})")
            }
            Self::OptionsNotAllowedOnAutoPopulatedSelect => {
                write!(f, "options array is only valid for string select menus")
            }
            Self::InvalidOptionLabelLength { len, max } => {
                write!(f, "select option label is {len} characters (must be 1-{max})")
            }
            Self::InvalidOptionValueLength { len, max } => {
                write!(f, "select option value is {len} characters (must be 1-{max})")
            }
            Self::OptionDescriptionTooLong { len, max } => {
                write!(f, "select option description is {len} characters (max {max})")
            }
            Self::InvalidTextInputLabelLength { len, max } => {
                write!(f, "text input label is {len} characters (must be 1-{max})")
            }
            Self::TextInputPlaceholderTooLong { len, max } => {
                write!(f, "text input placeholder is {len} characters (max {max})")
            }
            Self::MinLengthGreaterThanMaxLength { min, max } => {
                write!(f, "min_length ({min}) cannot be greater than max_length ({max})")
            }
            Self::InvalidMinLength { value, max } => {
                write!(f, "min_length {value} exceeds maximum of {max}")
            }
            Self::InvalidMaxLength { value, max } => {
                write!(f, "max_length {value} must be between 1 and {max}")
            }
            Self::TextInputValueTooLong { len, max } => {
                write!(f, "text input value is {len} characters (max {max})")
            }
            Self::InvalidTextDisplayContentLength { len, max } => {
                write!(f, "text display content is {len} characters (must be 1-{max})")
            }
            Self::MediaDescriptionTooLong { len, max } => {
                write!(f, "media description is {len} characters (max {max})")
            }
            Self::EmptyMediaUrl => write!(f, "media URL cannot be empty"),
            Self::InvalidMediaGalleryItemCount { count, max } => {
                write!(f, "media gallery has {count} items (must be 1-{max})")
            }
            Self::InvalidSectionComponentCount { count, max } => {
                write!(f, "section has {count} components (must be 1-{max})")
            }
            Self::EmptyContainer => write!(f, "container must contain at least one component"),
            Self::InvalidAccentColor { value } => {
                write!(f, "accent color {value:#x} exceeds 24-bit RGB maximum 0xFFFFFF")
            }
            Self::TooManyComponents { count, max } => {
                write!(f, "total components count {count} exceeds maximum {max}")
            }
            Self::InvalidModalTitleLength { len, max } => {
                write!(f, "modal title is {len} characters (must be 1-{max})")
            }
            Self::InvalidModalCustomIdLength { len, max } => {
                write!(f, "modal custom_id is {len} characters (must be 1-{max})")
            }
            Self::InvalidModalComponents { reason } => {
                write!(f, "invalid modal components: {reason}")
            }
            Self::TooManyAutocompleteChoices { count, max } => {
                write!(f, "autocomplete response has {count} choices (max {max})")
            }
        }
    }
}

impl std::error::Error for ComponentError {}

// ---------- Emoji for Components ----------

/// An emoji rendered on a button or select menu option.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComponentEmoji {
    /// Custom emoji snowflake ID, if this is a guild emoji.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_opt_snowflake",
        deserialize_with = "de_opt_snowflake"
    )]
    pub id: Option<Snowflake>,
    /// Emoji name or unicode character (e.g. `"👋"` or `"my_emoji"`).
    pub name: String,
    /// Whether the custom emoji is animated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animated: Option<bool>,
}

impl ComponentEmoji {
    /// Standard Unicode emoji (e.g. `"🚀"`).
    pub fn unicode(name: impl Into<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
            animated: None,
        }
    }

    /// Custom server emoji with an ID and name.
    pub fn custom(id: impl Into<Snowflake>, name: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            name: name.into(),
            animated: None,
        }
    }

    /// Set whether the emoji is animated.
    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = Some(animated);
        self
    }
}

// ---------- Button ----------

/// Visual style of a button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonStyle {
    /// Blurple button (style 1).
    Primary,
    /// Grey button (style 2).
    Secondary,
    /// Green button (style 3).
    Success,
    /// Red button (style 4).
    Danger,
    /// Link button with a URL (style 5).
    Link,
    /// Premium button promoting a SKU (style 6).
    Premium,
}

impl ButtonStyle {
    /// Convert style to raw Discord integer.
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Primary => 1,
            Self::Secondary => 2,
            Self::Success => 3,
            Self::Danger => 4,
            Self::Link => 5,
            Self::Premium => 6,
        }
    }

    /// Parse raw integer to ButtonStyle.
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::Primary),
            2 => Some(Self::Secondary),
            3 => Some(Self::Success),
            4 => Some(Self::Danger),
            5 => Some(Self::Link),
            6 => Some(Self::Premium),
            _ => None,
        }
    }
}

impl Serialize for ButtonStyle {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(self.as_u8())
    }
}

impl<'de> Deserialize<'de> for ButtonStyle {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = u8::deserialize(d)?;
        Self::from_u8(v).ok_or_else(|| serde::de::Error::custom(format!("invalid button style {v}")))
    }
}

/// An interactive button component (type 2).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Button {
    #[serde(rename = "type")]
    kind: u8,
    style: ButtonStyle,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    emoji: Option<ComponentEmoji>,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_opt_snowflake",
        deserialize_with = "de_opt_snowflake"
    )]
    sku_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disabled: Option<bool>,
}

impl Button {
    fn new(style: ButtonStyle) -> Self {
        Self {
            kind: 2,
            style,
            label: None,
            emoji: None,
            custom_id: None,
            url: None,
            sku_id: None,
            disabled: None,
        }
    }

    /// Create a Primary (blurple) button.
    pub fn primary(custom_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(ButtonStyle::Primary)
            .custom_id(custom_id)
            .label(label)
    }

    /// Create a Secondary (grey) button.
    pub fn secondary(custom_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(ButtonStyle::Secondary)
            .custom_id(custom_id)
            .label(label)
    }

    /// Create a Success (green) button.
    pub fn success(custom_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(ButtonStyle::Success)
            .custom_id(custom_id)
            .label(label)
    }

    /// Create a Danger (red) button.
    pub fn danger(custom_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(ButtonStyle::Danger)
            .custom_id(custom_id)
            .label(label)
    }

    /// Create a Link button (navigates to URL).
    pub fn link(url: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(ButtonStyle::Link).url(url).label(label)
    }

    /// Create a Premium button (prompts in-app SKU purchase).
    pub fn premium(sku_id: impl Into<Snowflake>) -> Self {
        Self::new(ButtonStyle::Premium).sku_id(sku_id)
    }

    /// Set the button's custom ID (1..=100 characters).
    pub fn custom_id(mut self, id: impl Into<String>) -> Self {
        self.custom_id = Some(id.into());
        self
    }

    /// Set the button's text label (max 80 characters).
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the button's emoji icon.
    pub fn emoji(mut self, emoji: ComponentEmoji) -> Self {
        self.emoji = Some(emoji);
        self
    }

    /// Set the button's URL (Link buttons only).
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Set the button's SKU ID (Premium buttons only).
    pub fn sku_id(mut self, sku_id: impl Into<Snowflake>) -> Self {
        self.sku_id = Some(sku_id.into());
        self
    }

    /// Set whether the button is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }

    /// The button style.
    pub fn style(&self) -> ButtonStyle {
        self.style
    }

    /// The button custom ID, if set.
    pub fn get_custom_id(&self) -> Option<&str> {
        self.custom_id.as_deref()
    }

    /// The button label, if set.
    pub fn get_label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// The button URL, if set.
    pub fn get_url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    /// Validate the button against Discord's documented constraints.
    pub fn validate(&self) -> Result<(), ComponentError> {
        if let Some(ref l) = self.label {
            let count = l.chars().count();
            if count > MAX_BUTTON_LABEL_LEN {
                return Err(ComponentError::ButtonLabelTooLong {
                    len: count,
                    max: MAX_BUTTON_LABEL_LEN,
                });
            }
        }

        match self.style {
            ButtonStyle::Link => {
                if self.url.as_ref().map_or(true, |u| u.is_empty()) {
                    return Err(ComponentError::MissingLinkUrl);
                }
                if self.custom_id.is_some() {
                    return Err(ComponentError::UnexpectedCustomIdOnLinkOrPremium);
                }
                if self.sku_id.is_some() {
                    return Err(ComponentError::UnexpectedPremiumSkuId);
                }
                if self.label.is_none() && self.emoji.is_none() {
                    return Err(ComponentError::MissingButtonLabelOrEmoji);
                }
            }
            ButtonStyle::Premium => {
                if self.sku_id.is_none() {
                    return Err(ComponentError::MissingPremiumSkuId);
                }
                if self.custom_id.is_some() {
                    return Err(ComponentError::UnexpectedCustomIdOnLinkOrPremium);
                }
                if self.url.is_some() {
                    return Err(ComponentError::UnexpectedLinkUrl);
                }
                if self.label.is_some() {
                    return Err(ComponentError::LabelNotAllowedOnPremiumButton);
                }
            }
            ButtonStyle::Primary
            | ButtonStyle::Secondary
            | ButtonStyle::Success
            | ButtonStyle::Danger => {
                if self.url.is_some() {
                    return Err(ComponentError::UnexpectedLinkUrl);
                }
                if self.sku_id.is_some() {
                    return Err(ComponentError::UnexpectedPremiumSkuId);
                }
                let cid = self.custom_id.as_ref().ok_or(ComponentError::MissingCustomId)?;
                let count = cid.chars().count();
                if !(1..=MAX_CUSTOM_ID_LEN).contains(&count) {
                    return Err(ComponentError::InvalidCustomIdLength {
                        len: count,
                        max: MAX_CUSTOM_ID_LEN,
                    });
                }
                if self.label.is_none() && self.emoji.is_none() {
                    return Err(ComponentError::MissingButtonLabelOrEmoji);
                }
            }
        }

        Ok(())
    }
}

// ---------- Select Menu ----------

/// Type of select menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectType {
    /// String select menu (type 3).
    String,
    /// User select menu (type 5).
    User,
    /// Role select menu (type 6).
    Role,
    /// Mentionable (user or role) select menu (type 7).
    Mentionable,
    /// Channel select menu (type 8).
    Channel,
}

impl SelectType {
    /// Convert SelectType to Discord's component type integer.
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::String => 3,
            Self::User => 5,
            Self::Role => 6,
            Self::Mentionable => 7,
            Self::Channel => 8,
        }
    }

    /// Parse integer into SelectType.
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            3 => Some(Self::String),
            5 => Some(Self::User),
            6 => Some(Self::Role),
            7 => Some(Self::Mentionable),
            8 => Some(Self::Channel),
            _ => None,
        }
    }
}

impl Serialize for SelectType {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(self.as_u8())
    }
}

impl<'de> Deserialize<'de> for SelectType {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = u8::deserialize(d)?;
        Self::from_u8(v).ok_or_else(|| serde::de::Error::custom(format!("invalid select type {v}")))
    }
}

/// An option in a String select menu.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectOption {
    label: String,
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    emoji: Option<ComponentEmoji>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default: Option<bool>,
}

impl SelectOption {
    /// Create a new select option with label and value (1..=100 characters each).
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            description: None,
            emoji: None,
            default: None,
        }
    }

    /// Set an optional description (max 100 characters).
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set an optional emoji.
    pub fn emoji(mut self, emoji: ComponentEmoji) -> Self {
        self.emoji = Some(emoji);
        self
    }

    /// Set whether this option is selected by default.
    pub fn default(mut self, default: bool) -> Self {
        self.default = Some(default);
        self
    }

    /// Option label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Option value.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Option description, if set.
    pub fn get_description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Validate option limits.
    pub fn validate(&self) -> Result<(), ComponentError> {
        let label_len = self.label.chars().count();
        if !(1..=MAX_SELECT_OPTION_LABEL_LEN).contains(&label_len) {
            return Err(ComponentError::InvalidOptionLabelLength {
                len: label_len,
                max: MAX_SELECT_OPTION_LABEL_LEN,
            });
        }
        let val_len = self.value.chars().count();
        if !(1..=MAX_SELECT_OPTION_VALUE_LEN).contains(&val_len) {
            return Err(ComponentError::InvalidOptionValueLength {
                len: val_len,
                max: MAX_SELECT_OPTION_VALUE_LEN,
            });
        }
        if let Some(ref d) = self.description {
            let d_len = d.chars().count();
            if d_len > MAX_SELECT_OPTION_DESC_LEN {
                return Err(ComponentError::OptionDescriptionTooLong {
                    len: d_len,
                    max: MAX_SELECT_OPTION_DESC_LEN,
                });
            }
        }
        Ok(())
    }
}

/// The kind of default value entity.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SelectDefaultValueType {
    /// User default value.
    User,
    /// Role default value.
    Role,
    /// Channel default value.
    Channel,
}

/// Default value entry for auto-populated select menus.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectDefaultValue {
    /// Snowflake ID of the entity.
    #[serde(serialize_with = "ser_snowflake", deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// Type of entity ("user", "role", or "channel").
    #[serde(rename = "type")]
    pub kind: SelectDefaultValueType,
}

impl SelectDefaultValue {
    /// Default user selection.
    pub fn user(id: impl Into<Snowflake>) -> Self {
        Self {
            id: id.into(),
            kind: SelectDefaultValueType::User,
        }
    }

    /// Default role selection.
    pub fn role(id: impl Into<Snowflake>) -> Self {
        Self {
            id: id.into(),
            kind: SelectDefaultValueType::Role,
        }
    }

    /// Default channel selection.
    pub fn channel(id: impl Into<Snowflake>) -> Self {
        Self {
            id: id.into(),
            kind: SelectDefaultValueType::Channel,
        }
    }
}

/// A select menu component (types 3, 5, 6, 7, 8).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectMenu {
    #[serde(rename = "type")]
    kind: SelectType,
    custom_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    options: Vec<SelectOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_values: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_values: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    channel_types: Vec<u8>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    default_values: Vec<SelectDefaultValue>,
}

impl SelectMenu {
    fn base(kind: SelectType, custom_id: impl Into<String>) -> Self {
        Self {
            kind,
            custom_id: custom_id.into(),
            options: Vec::new(),
            placeholder: None,
            min_values: None,
            max_values: None,
            disabled: None,
            channel_types: Vec::new(),
            default_values: Vec::new(),
        }
    }

    /// Create a String select menu (type 3) with pre-provided options.
    pub fn string(
        custom_id: impl Into<String>,
        options: impl IntoIterator<Item = SelectOption>,
    ) -> Self {
        let mut s = Self::base(SelectType::String, custom_id);
        s.options = options.into_iter().collect();
        s
    }

    /// Create a User select menu (type 5).
    pub fn user(custom_id: impl Into<String>) -> Self {
        Self::base(SelectType::User, custom_id)
    }

    /// Create a Role select menu (type 6).
    pub fn role(custom_id: impl Into<String>) -> Self {
        Self::base(SelectType::Role, custom_id)
    }

    /// Create a Mentionable (user or role) select menu (type 7).
    pub fn mentionable(custom_id: impl Into<String>) -> Self {
        Self::base(SelectType::Mentionable, custom_id)
    }

    /// Create a Channel select menu (type 8).
    pub fn channel(custom_id: impl Into<String>) -> Self {
        Self::base(SelectType::Channel, custom_id)
    }

    /// Add an option to a string select menu.
    pub fn option(mut self, opt: SelectOption) -> Self {
        self.options.push(opt);
        self
    }

    /// Set placeholder text (max 150 characters).
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = Some(text.into());
        self
    }

    /// Set minimum number of selections (0..=25).
    pub fn min_values(mut self, min: u8) -> Self {
        self.min_values = Some(min);
        self
    }

    /// Set maximum number of selections (1..=25).
    pub fn max_values(mut self, max: u8) -> Self {
        self.max_values = Some(max);
        self
    }

    /// Set whether the select menu is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }

    /// Channel types to filter for in a Channel select menu.
    pub fn channel_types(mut self, types: Vec<u8>) -> Self {
        self.channel_types = types;
        self
    }

    /// Default values for auto-populated select menus.
    pub fn default_values(mut self, values: Vec<SelectDefaultValue>) -> Self {
        self.default_values = values;
        self
    }

    /// The select menu's type.
    pub fn select_type(&self) -> SelectType {
        self.kind
    }

    /// The select menu's custom ID.
    pub fn custom_id(&self) -> &str {
        &self.custom_id
    }

    /// Validate the select menu against Discord constraints.
    pub fn validate(&self) -> Result<(), ComponentError> {
        let cid_len = self.custom_id.chars().count();
        if !(1..=MAX_CUSTOM_ID_LEN).contains(&cid_len) {
            return Err(ComponentError::InvalidCustomIdLength {
                len: cid_len,
                max: MAX_CUSTOM_ID_LEN,
            });
        }

        if let Some(ref p) = self.placeholder {
            let p_len = p.chars().count();
            if p_len > MAX_SELECT_PLACEHOLDER_LEN {
                return Err(ComponentError::PlaceholderTooLong {
                    len: p_len,
                    max: MAX_SELECT_PLACEHOLDER_LEN,
                });
            }
        }

        if let Some(min) = self.min_values {
            if min > MAX_SELECT_VALUES {
                return Err(ComponentError::InvalidMinValues { value: min });
            }
        }

        if let Some(max) = self.max_values {
            if !(1..=MAX_SELECT_VALUES).contains(&max) {
                return Err(ComponentError::InvalidMaxValues { value: max });
            }
        }

        if let (Some(min), Some(max)) = (self.min_values, self.max_values) {
            if min > max {
                return Err(ComponentError::MinValuesGreaterThanMaxValues { min, max });
            }
        }

        if self.kind == SelectType::String {
            let count = self.options.len();
            if !(1..=MAX_SELECT_OPTIONS).contains(&count) {
                return Err(ComponentError::InvalidOptionCount {
                    count,
                    min: 1,
                    max: MAX_SELECT_OPTIONS,
                });
            }
            for opt in &self.options {
                opt.validate()?;
            }
        } else if !self.options.is_empty() {
            return Err(ComponentError::OptionsNotAllowedOnAutoPopulatedSelect);
        }

        Ok(())
    }
}

// ---------- Text Input ----------

/// Style of a text input component (Short or Paragraph).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextInputStyle {
    /// Single-line text input (style 1).
    Short,
    /// Multi-line text area (style 2).
    Paragraph,
}

impl TextInputStyle {
    /// Convert style to Discord integer.
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Short => 1,
            Self::Paragraph => 2,
        }
    }

    /// Parse integer to TextInputStyle.
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::Short),
            2 => Some(Self::Paragraph),
            _ => None,
        }
    }
}

impl Serialize for TextInputStyle {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(self.as_u8())
    }
}

impl<'de> Deserialize<'de> for TextInputStyle {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = u8::deserialize(d)?;
        Self::from_u8(v).ok_or_else(|| serde::de::Error::custom(format!("invalid text input style {v}")))
    }
}

/// A text input component used in modals (type 4).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextInput {
    #[serde(rename = "type")]
    kind: u8,
    custom_id: String,
    style: TextInputStyle,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    placeholder: Option<String>,
}

impl TextInput {
    fn new(style: TextInputStyle, custom_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            kind: 4,
            custom_id: custom_id.into(),
            style,
            label: label.into(),
            min_length: None,
            max_length: None,
            required: None,
            value: None,
            placeholder: None,
        }
    }

    /// Create a single-line text input.
    pub fn short(custom_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(TextInputStyle::Short, custom_id, label)
    }

    /// Create a multi-line paragraph text input.
    pub fn paragraph(custom_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(TextInputStyle::Paragraph, custom_id, label)
    }

    /// Minimum length (0..=4000).
    pub fn min_length(mut self, min: u32) -> Self {
        self.min_length = Some(min);
        self
    }

    /// Maximum length (1..=4000).
    pub fn max_length(mut self, max: u32) -> Self {
        self.max_length = Some(max);
        self
    }

    /// Whether this input is required (defaults to true on Discord).
    pub fn required(mut self, req: bool) -> Self {
        self.required = Some(req);
        self
    }

    /// Pre-filled value (max 4000 characters).
    pub fn value(mut self, val: impl Into<String>) -> Self {
        self.value = Some(val.into());
        self
    }

    /// Placeholder text (max 100 characters).
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = Some(text.into());
        self
    }

    /// Text input custom ID.
    pub fn custom_id(&self) -> &str {
        &self.custom_id
    }

    /// Text input label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Text input style.
    pub fn style(&self) -> TextInputStyle {
        self.style
    }

    /// Validate text input limits.
    pub fn validate(&self) -> Result<(), ComponentError> {
        let cid_len = self.custom_id.chars().count();
        if !(1..=MAX_CUSTOM_ID_LEN).contains(&cid_len) {
            return Err(ComponentError::InvalidCustomIdLength {
                len: cid_len,
                max: MAX_CUSTOM_ID_LEN,
            });
        }

        let label_len = self.label.chars().count();
        if !(1..=MAX_TEXT_INPUT_LABEL_LEN).contains(&label_len) {
            return Err(ComponentError::InvalidTextInputLabelLength {
                len: label_len,
                max: MAX_TEXT_INPUT_LABEL_LEN,
            });
        }

        if let Some(min) = self.min_length {
            if min > MAX_TEXT_INPUT_BOUND {
                return Err(ComponentError::InvalidMinLength {
                    value: min,
                    max: MAX_TEXT_INPUT_BOUND,
                });
            }
        }

        if let Some(max) = self.max_length {
            if !(1..=MAX_TEXT_INPUT_BOUND).contains(&max) {
                return Err(ComponentError::InvalidMaxLength {
                    value: max,
                    max: MAX_TEXT_INPUT_BOUND,
                });
            }
        }

        if let (Some(min), Some(max)) = (self.min_length, self.max_length) {
            if min > max {
                return Err(ComponentError::MinLengthGreaterThanMaxLength { min, max });
            }
        }

        if let Some(ref p) = self.placeholder {
            let p_len = p.chars().count();
            if p_len > MAX_TEXT_INPUT_PLACEHOLDER_LEN {
                return Err(ComponentError::TextInputPlaceholderTooLong {
                    len: p_len,
                    max: MAX_TEXT_INPUT_PLACEHOLDER_LEN,
                });
            }
        }

        if let Some(ref v) = self.value {
            let v_len = v.chars().count();
            if v_len > MAX_TEXT_INPUT_VALUE_LEN {
                return Err(ComponentError::TextInputValueTooLong {
                    len: v_len,
                    max: MAX_TEXT_INPUT_VALUE_LEN,
                });
            }
        }

        Ok(())
    }
}

// ---------- ActionRow Child Components ----------

/// A child component placed inside an [`ActionRow`].
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ActionRowChild {
    /// A button.
    Button(Button),
    /// A select menu (String, User, Role, Mentionable, Channel).
    SelectMenu(SelectMenu),
    /// A text input (used in modals).
    TextInput(TextInput),
}

impl ActionRowChild {
    /// Validate this child component.
    pub fn validate(&self) -> Result<(), ComponentError> {
        match self {
            Self::Button(b) => b.validate(),
            Self::SelectMenu(s) => s.validate(),
            Self::TextInput(t) => t.validate(),
        }
    }
}

impl From<Button> for ActionRowChild {
    fn from(b: Button) -> Self {
        Self::Button(b)
    }
}

impl From<SelectMenu> for ActionRowChild {
    fn from(s: SelectMenu) -> Self {
        Self::SelectMenu(s)
    }
}

impl From<TextInput> for ActionRowChild {
    fn from(t: TextInput) -> Self {
        Self::TextInput(t)
    }
}

// ---------- ActionRow ----------

/// An ActionRow component (type 1) grouping buttons, select menus, or text inputs.
///
/// Discord rules:
/// - 1 to 5 Buttons, OR
/// - Exactly 1 SelectMenu, OR
/// - Exactly 1 TextInput (in modals).
///
/// Mixing buttons with select menus or text inputs is not permitted.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionRow {
    #[serde(rename = "type")]
    kind: u8,
    /// The components contained in this action row.
    pub components: Vec<ActionRowChild>,
}

impl ActionRow {
    /// Create an action row with 1..=5 buttons.
    pub fn buttons(buttons: impl IntoIterator<Item = Button>) -> Self {
        Self {
            kind: 1,
            components: buttons.into_iter().map(ActionRowChild::Button).collect(),
        }
    }

    /// Create an action row with a single select menu.
    pub fn select_menu(menu: SelectMenu) -> Self {
        Self {
            kind: 1,
            components: vec![ActionRowChild::SelectMenu(menu)],
        }
    }

    /// Create an action row with a single text input (for modals).
    pub fn text_input(input: TextInput) -> Self {
        Self {
            kind: 1,
            components: vec![ActionRowChild::TextInput(input)],
        }
    }

    /// Create an action row from arbitrary child components.
    pub fn new(components: Vec<ActionRowChild>) -> Self {
        Self {
            kind: 1,
            components,
        }
    }

    /// Add a button to this action row.
    pub fn add_button(mut self, button: Button) -> Self {
        self.components.push(ActionRowChild::Button(button));
        self
    }

    /// Validate action row composition and each child component.
    pub fn validate(&self) -> Result<(), ComponentError> {
        let count = self.components.len();
        if count == 0 {
            return Err(ComponentError::EmptyActionRow);
        }

        let mut has_button = false;
        let mut has_select = false;
        let mut has_text = false;

        for child in &self.components {
            match child {
                ActionRowChild::Button(b) => {
                    has_button = true;
                    b.validate()?;
                }
                ActionRowChild::SelectMenu(s) => {
                    has_select = true;
                    s.validate()?;
                }
                ActionRowChild::TextInput(t) => {
                    has_text = true;
                    t.validate()?;
                }
            }
        }

        let types_present = has_button as u8 + has_select as u8 + has_text as u8;
        if types_present > 1 {
            return Err(ComponentError::MixedActionRowComponents);
        }

        if has_button && count > MAX_BUTTONS_PER_ROW {
            return Err(ComponentError::TooManyComponentsInActionRow {
                count,
                max: MAX_BUTTONS_PER_ROW,
            });
        }

        if (has_select || has_text) && count > 1 {
            return Err(ComponentError::TooManyComponentsInActionRow { count, max: 1 });
        }

        Ok(())
    }
}

// ---------- Components V2 Types ----------

/// Media reference for unfurled media components.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnfurlMedia {
    /// Direct URL of the media asset.
    pub url: String,
    /// Proxy URL, if cached by Discord.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<String>,
}

impl UnfurlMedia {
    /// Create media reference from URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            proxy_url: None,
        }
    }
}

/// A TextDisplay component (Components V2, type 10).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextDisplay {
    #[serde(rename = "type")]
    kind: u8,
    /// Markdown content string (1..=4000 characters).
    pub content: String,
}

impl TextDisplay {
    /// Create a TextDisplay component with markdown content.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            kind: 10,
            content: content.into(),
        }
    }

    /// Validate content length.
    pub fn validate(&self) -> Result<(), ComponentError> {
        let count = self.content.chars().count();
        if !(1..=MAX_TEXT_DISPLAY_LEN).contains(&count) {
            return Err(ComponentError::InvalidTextDisplayContentLength {
                len: count,
                max: MAX_TEXT_DISPLAY_LEN,
            });
        }
        Ok(())
    }
}

/// A Thumbnail component (Components V2, type 11).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Thumbnail {
    #[serde(rename = "type")]
    kind: u8,
    /// Media item reference.
    pub media: UnfurlMedia,
    /// Alt text description (max 1024 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether the thumbnail is hidden behind a spoiler overlay.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spoiler: Option<bool>,
}

impl Thumbnail {
    /// Create a thumbnail component from an image URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            kind: 11,
            media: UnfurlMedia::new(url),
            description: None,
            spoiler: None,
        }
    }

    /// Set alt text description (max 1024 characters).
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set whether this thumbnail is a spoiler.
    pub fn spoiler(mut self, spoiler: bool) -> Self {
        self.spoiler = Some(spoiler);
        self
    }

    /// Validate thumbnail limits.
    pub fn validate(&self) -> Result<(), ComponentError> {
        if self.media.url.is_empty() {
            return Err(ComponentError::EmptyMediaUrl);
        }
        if let Some(ref d) = self.description {
            let count = d.chars().count();
            if count > MAX_MEDIA_DESC_LEN {
                return Err(ComponentError::MediaDescriptionTooLong {
                    len: count,
                    max: MAX_MEDIA_DESC_LEN,
                });
            }
        }
        Ok(())
    }
}

/// Accessory component in a Section (Button or Thumbnail).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum SectionAccessory {
    /// Button accessory.
    Button(Button),
    /// Thumbnail accessory.
    Thumbnail(Thumbnail),
}

impl SectionAccessory {
    /// Validate accessory limits.
    pub fn validate(&self) -> Result<(), ComponentError> {
        match self {
            Self::Button(b) => b.validate(),
            Self::Thumbnail(t) => t.validate(),
        }
    }
}

impl From<Button> for SectionAccessory {
    fn from(b: Button) -> Self {
        Self::Button(b)
    }
}

impl From<Thumbnail> for SectionAccessory {
    fn from(t: Thumbnail) -> Self {
        Self::Thumbnail(t)
    }
}

/// A Section component (Components V2, type 9).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Section {
    #[serde(rename = "type")]
    kind: u8,
    /// Text display components inside this section (1..=3).
    pub components: Vec<TextDisplay>,
    /// Optional right-aligned accessory (Button or Thumbnail).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessory: Option<SectionAccessory>,
}

impl Section {
    /// Create a section with text display components.
    pub fn new(components: Vec<TextDisplay>) -> Self {
        Self {
            kind: 9,
            components,
            accessory: None,
        }
    }

    /// Set an accessory on this section (Button or Thumbnail).
    pub fn accessory(mut self, accessory: impl Into<SectionAccessory>) -> Self {
        self.accessory = Some(accessory.into());
        self
    }

    /// Validate section limits.
    pub fn validate(&self) -> Result<(), ComponentError> {
        let count = self.components.len();
        if count == 0 || count > 5 {
            return Err(ComponentError::InvalidSectionComponentCount { count, max: 5 });
        }
        for c in &self.components {
            c.validate()?;
        }
        if let Some(ref a) = self.accessory {
            a.validate()?;
        }
        Ok(())
    }
}

/// An item in a MediaGallery component.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaGalleryItem {
    /// Media URL.
    pub media: UnfurlMedia,
    /// Alt text description (max 1024 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether this media is hidden behind a spoiler.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spoiler: Option<bool>,
}

impl MediaGalleryItem {
    /// Create a new gallery item from a media URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            media: UnfurlMedia::new(url),
            description: None,
            spoiler: None,
        }
    }

    /// Set alt text description (max 1024 characters).
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set whether this item is a spoiler.
    pub fn spoiler(mut self, spoiler: bool) -> Self {
        self.spoiler = Some(spoiler);
        self
    }

    /// Validate gallery item limits.
    pub fn validate(&self) -> Result<(), ComponentError> {
        if self.media.url.is_empty() {
            return Err(ComponentError::EmptyMediaUrl);
        }
        if let Some(ref d) = self.description {
            let count = d.chars().count();
            if count > MAX_MEDIA_DESC_LEN {
                return Err(ComponentError::MediaDescriptionTooLong {
                    len: count,
                    max: MAX_MEDIA_DESC_LEN,
                });
            }
        }
        Ok(())
    }
}

/// A MediaGallery component (Components V2, type 12).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaGallery {
    #[serde(rename = "type")]
    kind: u8,
    /// Items in the gallery (1..=10).
    pub items: Vec<MediaGalleryItem>,
}

impl MediaGallery {
    /// Create a MediaGallery component with items.
    pub fn new(items: Vec<MediaGalleryItem>) -> Self {
        Self { kind: 12, items }
    }

    /// Add an item to this media gallery.
    pub fn add_item(mut self, item: MediaGalleryItem) -> Self {
        self.items.push(item);
        self
    }

    /// Validate media gallery limits.
    pub fn validate(&self) -> Result<(), ComponentError> {
        let count = self.items.len();
        if count == 0 || count > MAX_MEDIA_GALLERY_ITEMS {
            return Err(ComponentError::InvalidMediaGalleryItemCount {
                count,
                max: MAX_MEDIA_GALLERY_ITEMS,
            });
        }
        for item in &self.items {
            item.validate()?;
        }
        Ok(())
    }
}

/// Reference to an uploaded attachment.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttachmentReference {
    /// Snowflake ID of the uploaded attachment.
    #[serde(serialize_with = "ser_snowflake", deserialize_with = "de_snowflake")]
    pub id: Snowflake,
}

impl AttachmentReference {
    /// Create reference from snowflake ID.
    pub fn new(id: impl Into<Snowflake>) -> Self {
        Self { id: id.into() }
    }
}

/// A File component referencing an attachment (Components V2, type 13).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileComponent {
    #[serde(rename = "type")]
    kind: u8,
    /// Attachment reference.
    pub file: AttachmentReference,
    /// Whether the file is hidden behind a spoiler overlay.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spoiler: Option<bool>,
}

/// Alias for `FileComponent`.
pub type ComponentFile = FileComponent;

impl FileComponent {
    /// Create a File component referencing an attachment ID.
    pub fn new(id: impl Into<Snowflake>) -> Self {
        Self {
            kind: 13,
            file: AttachmentReference::new(id),
            spoiler: None,
        }
    }

    /// Set whether this file is marked as a spoiler.
    pub fn spoiler(mut self, spoiler: bool) -> Self {
        self.spoiler = Some(spoiler);
        self
    }

    /// Validate file component.
    pub fn validate(&self) -> Result<(), ComponentError> {
        Ok(())
    }
}

/// Vertical spacing above and below a Separator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SeparatorSpacing {
    /// Small spacing (1).
    Small,
    /// Large spacing (2).
    Large,
}

impl SeparatorSpacing {
    /// Convert to Discord integer.
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Small => 1,
            Self::Large => 2,
        }
    }

    /// Parse integer to SeparatorSpacing.
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::Small),
            2 => Some(Self::Large),
            _ => None,
        }
    }
}

impl Serialize for SeparatorSpacing {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(self.as_u8())
    }
}

impl<'de> Deserialize<'de> for SeparatorSpacing {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = u8::deserialize(d)?;
        Self::from_u8(v).ok_or_else(|| serde::de::Error::custom(format!("invalid separator spacing {v}")))
    }
}

/// A visual Separator component (Components V2, type 14).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Separator {
    #[serde(rename = "type")]
    kind: u8,
    /// Whether to show a visible dividing line (default true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub divider: Option<bool>,
    /// Spacing size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spacing: Option<SeparatorSpacing>,
}

impl Default for Separator {
    fn default() -> Self {
        Self::new()
    }
}

impl Separator {
    /// Create a separator component.
    pub fn new() -> Self {
        Self {
            kind: 14,
            divider: None,
            spacing: None,
        }
    }

    /// Set whether a divider line is shown.
    pub fn divider(mut self, divider: bool) -> Self {
        self.divider = Some(divider);
        self
    }

    /// Set spacing size.
    pub fn spacing(mut self, spacing: SeparatorSpacing) -> Self {
        self.spacing = Some(spacing);
        self
    }

    /// Validate separator.
    pub fn validate(&self) -> Result<(), ComponentError> {
        Ok(())
    }
}

/// A Container component grouping nested components (Components V2, type 17).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Container {
    #[serde(rename = "type")]
    kind: u8,
    /// 24-bit RGB accent border color (0x000000..=0xFFFFFF).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<u32>,
    /// Whether the entire container is hidden behind a spoiler overlay.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spoiler: Option<bool>,
    /// Inner components inside this container.
    pub components: Vec<Component>,
}

impl Container {
    /// Create a Container component with inner components.
    pub fn new(components: Vec<Component>) -> Self {
        Self {
            kind: 17,
            accent_color: None,
            spoiler: None,
            components,
        }
    }

    /// Set 24-bit RGB accent color.
    pub fn accent_color(mut self, color: u32) -> Self {
        self.accent_color = Some(color);
        self
    }

    /// Set whether the container is a spoiler.
    pub fn spoiler(mut self, spoiler: bool) -> Self {
        self.spoiler = Some(spoiler);
        self
    }

    /// Validate container and all nested components.
    pub fn validate(&self) -> Result<(), ComponentError> {
        if self.components.is_empty() {
            return Err(ComponentError::EmptyContainer);
        }
        if let Some(color) = self.accent_color {
            if color > 0xFF_FFFF {
                return Err(ComponentError::InvalidAccentColor { value: color });
            }
        }
        for c in &self.components {
            c.validate()?;
        }
        Ok(())
    }
}

// ---------- Top-level Component Enum ----------

/// A top-level Discord message component (V1 or Components V2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Component {
    /// ActionRow (type 1).
    ActionRow(ActionRow),
    /// Button (type 2).
    Button(Button),
    /// SelectMenu (types 3, 5, 6, 7, 8).
    SelectMenu(SelectMenu),
    /// TextInput (type 4).
    TextInput(TextInput),
    /// Section (Components V2, type 9).
    Section(Section),
    /// TextDisplay (Components V2, type 10).
    TextDisplay(TextDisplay),
    /// Thumbnail (Components V2, type 11).
    Thumbnail(Thumbnail),
    /// MediaGallery (Components V2, type 12).
    MediaGallery(MediaGallery),
    /// File (Components V2, type 13).
    File(FileComponent),
    /// Separator (Components V2, type 14).
    Separator(Separator),
    /// Container (Components V2, type 17).
    Container(Container),
}

impl Component {
    /// Discord component type integer.
    pub fn component_type(&self) -> u8 {
        match self {
            Self::ActionRow(_) => 1,
            Self::Button(_) => 2,
            Self::SelectMenu(s) => s.select_type().as_u8(),
            Self::TextInput(_) => 4,
            Self::Section(_) => 9,
            Self::TextDisplay(_) => 10,
            Self::Thumbnail(_) => 11,
            Self::MediaGallery(_) => 12,
            Self::File(_) => 13,
            Self::Separator(_) => 14,
            Self::Container(_) => 17,
        }
    }

    /// Count the total number of components in this component subtree.
    pub fn count_total(&self) -> usize {
        match self {
            Self::ActionRow(a) => 1 + a.components.len(),
            Self::Section(s) => 1 + s.components.len() + s.accessory.is_some() as usize,
            Self::Container(c) => 1 + c.components.iter().map(|child| child.count_total()).sum::<usize>(),
            Self::MediaGallery(g) => 1 + g.items.len(),
            _ => 1,
        }
    }

    /// Validate this component and all its nested children.
    pub fn validate(&self) -> Result<(), ComponentError> {
        match self {
            Self::ActionRow(a) => a.validate(),
            Self::Button(b) => b.validate(),
            Self::SelectMenu(s) => s.validate(),
            Self::TextInput(t) => t.validate(),
            Self::Section(s) => s.validate(),
            Self::TextDisplay(t) => t.validate(),
            Self::Thumbnail(t) => t.validate(),
            Self::MediaGallery(m) => m.validate(),
            Self::File(f) => f.validate(),
            Self::Separator(s) => s.validate(),
            Self::Container(c) => c.validate(),
        }
    }
}

impl Serialize for Component {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::ActionRow(c) => c.serialize(s),
            Self::Button(c) => c.serialize(s),
            Self::SelectMenu(c) => c.serialize(s),
            Self::TextInput(c) => c.serialize(s),
            Self::Section(c) => c.serialize(s),
            Self::TextDisplay(c) => c.serialize(s),
            Self::Thumbnail(c) => c.serialize(s),
            Self::MediaGallery(c) => c.serialize(s),
            Self::File(c) => c.serialize(s),
            Self::Separator(c) => c.serialize(s),
            Self::Container(c) => c.serialize(s),
        }
    }
}

impl<'de> Deserialize<'de> for Component {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let val = serde_json::Value::deserialize(d)?;
        let kind = val.get("type").and_then(|v| v.as_u64()).ok_or_else(|| {
            serde::de::Error::custom("missing or invalid 'type' field in component JSON")
        })?;
        match kind {
            1 => serde_json::from_value(val).map(Self::ActionRow),
            2 => serde_json::from_value(val).map(Self::Button),
            3 | 5 | 6 | 7 | 8 => serde_json::from_value(val).map(Self::SelectMenu),
            4 => serde_json::from_value(val).map(Self::TextInput),
            9 => serde_json::from_value(val).map(Self::Section),
            10 => serde_json::from_value(val).map(Self::TextDisplay),
            11 => serde_json::from_value(val).map(Self::Thumbnail),
            12 => serde_json::from_value(val).map(Self::MediaGallery),
            13 => serde_json::from_value(val).map(Self::File),
            14 => serde_json::from_value(val).map(Self::Separator),
            17 => serde_json::from_value(val).map(Self::Container),
            other => Err(serde::de::Error::custom(format!("unknown component type {other}"))),
        }
        .map_err(serde::de::Error::custom)
    }
}

impl From<ActionRow> for Component {
    fn from(a: ActionRow) -> Self {
        Self::ActionRow(a)
    }
}

impl From<Button> for Component {
    fn from(b: Button) -> Self {
        Self::Button(b)
    }
}

impl From<SelectMenu> for Component {
    fn from(s: SelectMenu) -> Self {
        Self::SelectMenu(s)
    }
}

impl From<TextInput> for Component {
    fn from(t: TextInput) -> Self {
        Self::TextInput(t)
    }
}

impl From<Section> for Component {
    fn from(s: Section) -> Self {
        Self::Section(s)
    }
}

impl From<TextDisplay> for Component {
    fn from(t: TextDisplay) -> Self {
        Self::TextDisplay(t)
    }
}

impl From<Thumbnail> for Component {
    fn from(t: Thumbnail) -> Self {
        Self::Thumbnail(t)
    }
}

impl From<MediaGallery> for Component {
    fn from(m: MediaGallery) -> Self {
        Self::MediaGallery(m)
    }
}

impl From<FileComponent> for Component {
    fn from(f: FileComponent) -> Self {
        Self::File(f)
    }
}

impl From<Separator> for Component {
    fn from(s: Separator) -> Self {
        Self::Separator(s)
    }
}

impl From<Container> for Component {
    fn from(c: Container) -> Self {
        Self::Container(c)
    }
}

/// Helper function to count the total components in a slice of components.
pub fn count_components(components: &[Component]) -> usize {
    components.iter().map(|c| c.count_total()).sum()
}

/// Validate a list of components when Components V2 is active (max 40 total components).
pub fn validate_components_v2(components: &[Component]) -> Result<(), ComponentError> {
    let total = count_components(components);
    if total > MAX_COMPONENTS_V2_TOTAL {
        return Err(ComponentError::TooManyComponents {
            count: total,
            max: MAX_COMPONENTS_V2_TOTAL,
        });
    }
    for c in components {
        c.validate()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_button_builders_and_validation() {
        let p = Button::primary("click", "Click Me").emoji(ComponentEmoji::unicode("👋"));
        assert!(p.validate().is_ok());
        let val = serde_json::to_value(&p).unwrap();
        assert_eq!(
            val,
            json!({
                "type": 2,
                "style": 1,
                "custom_id": "click",
                "label": "Click Me",
                "emoji": { "name": "👋" }
            })
        );

        let link = Button::link("https://example.com", "Visit");
        assert!(link.validate().is_ok());
        let val_link = serde_json::to_value(&link).unwrap();
        assert_eq!(
            val_link,
            json!({
                "type": 2,
                "style": 5,
                "label": "Visit",
                "url": "https://example.com"
            })
        );

        let prem = Button::premium(Snowflake::new(12345));
        assert!(prem.validate().is_ok());
        let val_prem = serde_json::to_value(&prem).unwrap();
        assert_eq!(
            val_prem,
            json!({
                "type": 2,
                "style": 6,
                "sku_id": "12345"
            })
        );

        // Validation failures
        let bad_custom_id = Button::primary("a".repeat(101), "Label");
        assert!(matches!(
            bad_custom_id.validate(),
            Err(ComponentError::InvalidCustomIdLength { len: 101, max: 100 })
        ));

        let bad_label = Button::primary("btn", "a".repeat(81));
        assert!(matches!(
            bad_label.validate(),
            Err(ComponentError::ButtonLabelTooLong { len: 81, max: 80 })
        ));

        let link_with_custom_id = Button::link("https://example.com", "Visit").custom_id("bad");
        assert_eq!(
            link_with_custom_id.validate(),
            Err(ComponentError::UnexpectedCustomIdOnLinkOrPremium)
        );

        let prem_with_label = Button::premium(Snowflake::new(123)).label("Buy Now");
        assert_eq!(
            prem_with_label.validate(),
            Err(ComponentError::LabelNotAllowedOnPremiumButton)
        );

        let no_label_or_emoji = Button::primary("btn", "");
        let empty_btn = Button::new(ButtonStyle::Primary).custom_id("btn");
        assert_eq!(
            empty_btn.validate(),
            Err(ComponentError::MissingButtonLabelOrEmoji)
        );
        let _ = no_label_or_emoji;
    }

    #[test]
    fn test_select_menu_builders_and_validation() {
        let opts = vec![
            SelectOption::new("Option 1", "opt1").description("First option"),
            SelectOption::new("Option 2", "opt2").default(true),
        ];
        let menu = SelectMenu::string("menu", opts)
            .placeholder("Select something...")
            .min_values(1)
            .max_values(2);
        assert!(menu.validate().is_ok());

        let val = serde_json::to_value(&menu).unwrap();
        assert_eq!(
            val,
            json!({
                "type": 3,
                "custom_id": "menu",
                "placeholder": "Select something...",
                "min_values": 1,
                "max_values": 2,
                "options": [
                    { "label": "Option 1", "value": "opt1", "description": "First option" },
                    { "label": "Option 2", "value": "opt2", "default": true }
                ]
            })
        );

        let user_menu = SelectMenu::user("user_select");
        assert!(user_menu.validate().is_ok());
        assert_eq!(user_menu.select_type(), SelectType::User);

        // Validation failures
        let bad_min_max = SelectMenu::string("menu", vec![SelectOption::new("a", "b")])
            .min_values(5)
            .max_values(2);
        assert_eq!(
            bad_min_max.validate(),
            Err(ComponentError::MinValuesGreaterThanMaxValues { min: 5, max: 2 })
        );

        let empty_opts = SelectMenu::string("menu", vec![]);
        assert_eq!(
            empty_opts.validate(),
            Err(ComponentError::InvalidOptionCount { count: 0, min: 1, max: 25 })
        );

        let too_many_opts = (0..26).map(|i| SelectOption::new(format!("o{i}"), format!("v{i}"))).collect::<Vec<_>>();
        let menu_many = SelectMenu::string("menu", too_many_opts);
        assert_eq!(
            menu_many.validate(),
            Err(ComponentError::InvalidOptionCount { count: 26, min: 1, max: 25 })
        );

        let opt_bad_label = SelectOption::new("a".repeat(101), "val");
        assert_eq!(
            opt_bad_label.validate(),
            Err(ComponentError::InvalidOptionLabelLength { len: 101, max: 100 })
        );
    }

    #[test]
    fn test_text_input_builders_and_validation() {
        let input = TextInput::paragraph("bio", "Biography")
            .placeholder("Tell us about yourself...")
            .min_length(10)
            .max_length(500)
            .required(true)
            .value("Existing bio text");
        assert!(input.validate().is_ok());

        let val = serde_json::to_value(&input).unwrap();
        assert_eq!(
            val,
            json!({
                "type": 4,
                "custom_id": "bio",
                "style": 2,
                "label": "Biography",
                "placeholder": "Tell us about yourself...",
                "min_length": 10,
                "max_length": 500,
                "required": true,
                "value": "Existing bio text"
            })
        );

        // Failures
        let label_too_long = TextInput::short("id", "a".repeat(46));
        assert_eq!(
            label_too_long.validate(),
            Err(ComponentError::InvalidTextInputLabelLength { len: 46, max: 45 })
        );

        let bounds_inverted = TextInput::short("id", "label").min_length(100).max_length(50);
        assert_eq!(
            bounds_inverted.validate(),
            Err(ComponentError::MinLengthGreaterThanMaxLength { min: 100, max: 50 })
        );

        let max_len_overflow = TextInput::short("id", "label").max_length(4001);
        assert_eq!(
            max_len_overflow.validate(),
            Err(ComponentError::InvalidMaxLength { value: 4001, max: 4000 })
        );
    }

    #[test]
    fn test_action_row_validation() {
        let b1 = Button::primary("1", "One");
        let b2 = Button::secondary("2", "Two");
        let b3 = Button::success("3", "Three");
        let row = ActionRow::buttons(vec![b1.clone(), b2.clone(), b3.clone()]);
        assert!(row.validate().is_ok());

        let val = serde_json::to_value(&row).unwrap();
        assert_eq!(val["type"], 1);
        assert_eq!(val["components"].as_array().unwrap().len(), 3);

        // Empty ActionRow fails
        let empty = ActionRow::buttons(vec![]);
        assert_eq!(empty.validate(), Err(ComponentError::EmptyActionRow));

        // More than 5 buttons fails
        let six = (0..6).map(|i| Button::primary(format!("b{i}"), format!("btn{i}"))).collect::<Vec<_>>();
        let row_six = ActionRow::buttons(six);
        assert_eq!(
            row_six.validate(),
            Err(ComponentError::TooManyComponentsInActionRow { count: 6, max: 5 })
        );

        // Mixing buttons with a select menu fails
        let mixed = ActionRow::new(vec![
            ActionRowChild::Button(b1),
            ActionRowChild::SelectMenu(SelectMenu::string("s", vec![SelectOption::new("a", "b")])),
        ]);
        assert_eq!(mixed.validate(), Err(ComponentError::MixedActionRowComponents));

        // Select menu row with 1 menu is ok, but 2 menus fail
        let two_menus = ActionRow::new(vec![
            ActionRowChild::SelectMenu(SelectMenu::string("s1", vec![SelectOption::new("a", "b")])),
            ActionRowChild::SelectMenu(SelectMenu::string("s2", vec![SelectOption::new("c", "d")])),
        ]);
        assert_eq!(
            two_menus.validate(),
            Err(ComponentError::TooManyComponentsInActionRow { count: 2, max: 1 })
        );
    }

    #[test]
    fn test_components_v2_types_and_tree_validation() {
        let text = TextDisplay::new("## Heading\nSome text content");
        assert!(text.validate().is_ok());

        let thumb = Thumbnail::new("https://example.com/image.png").description("Cool preview").spoiler(true);
        assert!(thumb.validate().is_ok());

        let section = Section::new(vec![text.clone()]).accessory(thumb.clone());
        assert!(section.validate().is_ok());

        let sep = Separator::new().divider(true).spacing(SeparatorSpacing::Small);
        assert!(sep.validate().is_ok());

        let media = MediaGallery::new(vec![
            MediaGalleryItem::new("https://example.com/1.png").description("Pic 1"),
            MediaGalleryItem::new("https://example.com/2.png").spoiler(true),
        ]);
        assert!(media.validate().is_ok());

        let file = FileComponent::new(Snowflake::new(99999)).spoiler(false);
        assert!(file.validate().is_ok());

        let container = Container::new(vec![
            Component::Section(section),
            Component::Separator(sep),
            Component::MediaGallery(media),
            Component::File(file),
        ])
        .accent_color(0x5865F2)
        .spoiler(false);
        assert!(container.validate().is_ok());

        let val = serde_json::to_value(&container).unwrap();
        assert_eq!(val["type"], 17);
        assert_eq!(val["accent_color"], 5793266);
        assert_eq!(val["components"].as_array().unwrap().len(), 4);

        // V2 total component count limit
        let mut components = Vec::new();
        for _ in 0..41 {
            components.push(Component::TextDisplay(TextDisplay::new("x")));
        }
        assert!(matches!(
            validate_components_v2(&components),
            Err(ComponentError::TooManyComponents { count: 41, max: 40 })
        ));
    }
}
