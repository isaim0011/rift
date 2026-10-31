//! Gateway intents, as listed in the Gateway docs.

use std::ops::{BitOr, BitOrAssign};

/// A set of gateway intents (bitflags).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Intents(u32);

macro_rules! intents {
    ($($name:ident = $bit:expr),* $(,)?) => {
        impl Intents {
            $(#[allow(missing_docs)] pub const $name: Intents = Intents(1 << $bit);)*
            /// Every intent Discord currently documents.
            pub const ALL_KNOWN: u32 = 0 $(| (1 << $bit))*;
        }
    };
}

intents! {
    GUILDS = 0,
    GUILD_MEMBERS = 1,            // privileged
    GUILD_MODERATION = 2,
    GUILD_EXPRESSIONS = 3,
    GUILD_INTEGRATIONS = 4,
    GUILD_WEBHOOKS = 5,
    GUILD_INVITES = 6,
    GUILD_VOICE_STATES = 7,
    GUILD_PRESENCES = 8,          // privileged
    GUILD_MESSAGES = 9,
    GUILD_MESSAGE_REACTIONS = 10,
    GUILD_MESSAGE_TYPING = 11,
    DIRECT_MESSAGES = 12,
    DIRECT_MESSAGE_REACTIONS = 13,
    DIRECT_MESSAGE_TYPING = 14,
    MESSAGE_CONTENT = 15,         // privileged
    GUILD_SCHEDULED_EVENTS = 16,
    AUTO_MODERATION_CONFIGURATION = 20,
    AUTO_MODERATION_EXECUTION = 21,
    GUILD_MESSAGE_POLLS = 24,
    DIRECT_MESSAGE_POLLS = 25,
}

impl Intents {
    /// No intents.
    pub const NONE: Intents = Intents(0);

    /// Intents that need a Developer Portal toggle (and review past 10k users).
    pub const PRIVILEGED: Intents =
        Intents(Self::GUILD_MEMBERS.0 | Self::GUILD_PRESENCES.0 | Self::MESSAGE_CONTENT.0);

    /// Raw bits, ready for the Identify payload.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Build from raw bits, rejecting bits Discord doesn't define
    /// (an invalid intent closes the connection with code 4013).
    pub const fn from_bits(bits: u32) -> Option<Intents> {
        if bits & !Self::ALL_KNOWN == 0 {
            Some(Intents(bits))
        } else {
            None
        }
    }

    /// True if every intent in `other` is set.
    pub const fn contains(self, other: Intents) -> bool {
        self.0 & other.0 == other.0
    }

    /// The privileged intents in this set. Non-empty means the app must have
    /// them enabled in the portal, or Discord closes with 4014.
    pub const fn privileged_in(self) -> Intents {
        Intents(self.0 & Self::PRIVILEGED.0)
    }

    /// True if no intents are set.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl BitOr for Intents {
    type Output = Intents;
    fn bitor(self, rhs: Intents) -> Intents {
        Intents(self.0 | rhs.0)
    }
}

impl BitOrAssign for Intents {
    fn bitor_assign(&mut self, rhs: Intents) {
        self.0 |= rhs.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_documented_example() {
        // The docs' minimal Identify uses 513 = GUILDS | GUILD_MESSAGES.
        assert_eq!((Intents::GUILDS | Intents::GUILD_MESSAGES).bits(), 513);
    }

    #[test]
    fn rejects_unknown_bits() {
        assert!(Intents::from_bits(1 << 17).is_none());
        assert!(Intents::from_bits(513).is_some());
    }

    #[test]
    fn detects_privileged() {
        let i = Intents::GUILDS | Intents::MESSAGE_CONTENT;
        assert_eq!(i.privileged_in(), Intents::MESSAGE_CONTENT);
        assert!(Intents::GUILDS.privileged_in().is_empty());
        assert!(i.contains(Intents::GUILDS));
    }
}
