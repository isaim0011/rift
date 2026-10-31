//! Gateway capabilities (Identify `capabilities` field).
//!
//! A separate bitfield from [`Intents`](crate::Intents): intents choose which
//! events you receive, capabilities change how existing gateway behavior works.
//! They are distinct types here so one can't be passed where the other is
//! expected (the bit values can collide: `MESSAGE_CONTENT` and
//! `CHANNEL_OBFUSCATION` are both `1 << 15`).

use std::ops::{BitOr, BitOrAssign};

/// A set of gateway capabilities. Unknown bits are preserved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Capabilities(u32);

impl Capabilities {
    /// No capabilities (the Identify default of `0`).
    pub const NONE: Capabilities = Capabilities(0);

    /// Opt in to receiving obfuscated channel metadata for channels the bot
    /// can't view.
    ///
    /// Discord documents this as a **temporary, testing-only** opt-in whose
    /// mechanism will change before general availability; obfuscation is then
    /// planned to apply to all bots automatically.
    pub const CHANNEL_OBFUSCATION: Capabilities = Capabilities(1 << 15);

    /// Wrap raw bits, keeping any this version doesn't know about.
    pub const fn from_bits_retain(bits: u32) -> Self {
        Self(bits)
    }

    /// Raw bits for the Identify payload.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// True if every capability in `other` is set.
    pub const fn contains(self, other: Capabilities) -> bool {
        self.0 & other.0 == other.0
    }

    /// True if nothing is set.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl BitOr for Capabilities {
    type Output = Capabilities;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Capabilities {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Intents;

    #[test]
    fn channel_obfuscation_is_bit_15() {
        assert_eq!(Capabilities::CHANNEL_OBFUSCATION.bits(), 32_768);
    }

    #[test]
    fn same_bit_as_message_content_but_a_different_type() {
        // The trap documented in the module docs: the numbers collide, so the
        // type system has to keep them apart.
        assert_eq!(Capabilities::CHANNEL_OBFUSCATION.bits(), Intents::MESSAGE_CONTENT.bits());
    }

    #[test]
    fn default_is_none_and_flags_combine() {
        assert!(Capabilities::default().is_empty());
        let mut c = Capabilities::NONE;
        c |= Capabilities::CHANNEL_OBFUSCATION;
        assert!(c.contains(Capabilities::CHANNEL_OBFUSCATION));
        assert_eq!(Capabilities::from_bits_retain(1 << 3).bits(), 8);
    }
}
