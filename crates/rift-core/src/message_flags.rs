//! Message flags (Message Resource, "Message Flags").
//!
//! A separate type from [`Intents`](crate::Intents) and
//! [`Capabilities`](crate::Capabilities): `IS_COMPONENTS_V2`, `MESSAGE_CONTENT`
//! and `CHANNEL_OBFUSCATION` are all `1 << 15` in different bitfields.

use std::ops::{BitOr, BitOrAssign};

/// A set of message flags. Unknown bits are preserved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct MessageFlags(u32);

macro_rules! message_flags {
    ($($name:ident = $bit:expr),* $(,)?) => {
        impl MessageFlags {
            $(#[allow(missing_docs)] pub const $name: MessageFlags = MessageFlags(1 << $bit);)*
        }
    };
}

message_flags! {
    CROSSPOSTED = 0,
    IS_CROSSPOST = 1,
    SUPPRESS_EMBEDS = 2,
    SOURCE_MESSAGE_DELETED = 3,
    URGENT = 4,
    HAS_THREAD = 5,
    EPHEMERAL = 6,
    LOADING = 7,
    FAILED_TO_MENTION_SOME_ROLES_IN_THREAD = 8,
    SUPPRESS_NOTIFICATIONS = 12,
    IS_VOICE_MESSAGE = 13,
    HAS_SNAPSHOT = 14,
    IS_COMPONENTS_V2 = 15,
}

impl MessageFlags {
    /// No flags.
    pub const NONE: MessageFlags = MessageFlags(0);

    /// The only flags Create Message accepts: `SUPPRESS_EMBEDS`,
    /// `SUPPRESS_NOTIFICATIONS`, `IS_VOICE_MESSAGE` and `IS_COMPONENTS_V2`.
    pub const SETTABLE_ON_CREATE: MessageFlags = MessageFlags(
        Self::SUPPRESS_EMBEDS.0
            | Self::SUPPRESS_NOTIFICATIONS.0
            | Self::IS_VOICE_MESSAGE.0
            | Self::IS_COMPONENTS_V2.0,
    );

    /// The only flags an interaction response may carry: `SUPPRESS_EMBEDS`,
    /// `EPHEMERAL`, `IS_COMPONENTS_V2`, `IS_VOICE_MESSAGE`, `SUPPRESS_NOTIFICATIONS`.
    pub const SETTABLE_IN_INTERACTION_RESPONSE: MessageFlags = MessageFlags(
        Self::SUPPRESS_EMBEDS.0
            | Self::EPHEMERAL.0
            | Self::IS_COMPONENTS_V2.0
            | Self::IS_VOICE_MESSAGE.0
            | Self::SUPPRESS_NOTIFICATIONS.0,
    );

    /// Wrap raw bits, keeping any this version doesn't know about.
    pub const fn from_bits_retain(bits: u32) -> Self {
        Self(bits)
    }

    /// Raw bits.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// True if every flag in `other` is set.
    pub const fn contains(self, other: MessageFlags) -> bool {
        self.0 & other.0 == other.0
    }

    /// True if nothing is set.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Bits set that Create Message would reject (empty if the set is valid).
    pub const fn unsettable_on_create(self) -> u32 {
        self.0 & !Self::SETTABLE_ON_CREATE.0
    }
}

impl MessageFlags {
    /// Bits set that an interaction response would reject (empty if valid).
    pub const fn unsettable_in_interaction_response(self) -> u32 {
        self.0 & !Self::SETTABLE_IN_INTERACTION_RESPONSE.0
    }
}

impl BitOr for MessageFlags {
    type Output = MessageFlags;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for MessageFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_match_the_docs() {
        assert_eq!(MessageFlags::SUPPRESS_EMBEDS.bits(), 4);
        assert_eq!(MessageFlags::EPHEMERAL.bits(), 64);
        assert_eq!(MessageFlags::SUPPRESS_NOTIFICATIONS.bits(), 4096);
        assert_eq!(MessageFlags::IS_VOICE_MESSAGE.bits(), 8192);
        assert_eq!(MessageFlags::IS_COMPONENTS_V2.bits(), 32_768);
    }

    #[test]
    fn interaction_responses_allow_ephemeral_but_not_crossposted() {
        let ok = MessageFlags::EPHEMERAL | MessageFlags::SUPPRESS_EMBEDS;
        assert_eq!(ok.unsettable_in_interaction_response(), 0);
        assert_eq!(MessageFlags::CROSSPOSTED.unsettable_in_interaction_response(), 1);
        assert_eq!(MessageFlags::URGENT.unsettable_in_interaction_response(), 16);
    }

    #[test]
    fn only_four_flags_are_settable_on_create() {
        let ok = MessageFlags::SUPPRESS_EMBEDS | MessageFlags::SUPPRESS_NOTIFICATIONS;
        assert_eq!(ok.unsettable_on_create(), 0);
        assert_eq!(MessageFlags::IS_COMPONENTS_V2.unsettable_on_create(), 0);
        // EPHEMERAL is for interaction responses, not Create Message.
        assert_eq!(MessageFlags::EPHEMERAL.unsettable_on_create(), 64);
        assert_eq!(MessageFlags::NONE.unsettable_on_create(), 0);
    }
}
