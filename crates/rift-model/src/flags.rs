//! Bitfield flags used in Discord Gateway objects.

use serde::{Deserialize, Serialize};
use std::ops::{BitOr, BitOrAssign};

/// Flags for a Discord channel.
///
/// See <https://discord.com/developers/docs/resources/channel#channel-object-channel-flags>
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChannelFlags(pub u32);

impl ChannelFlags {
    /// No flags set.
    pub const NONE: ChannelFlags = ChannelFlags(0);

    /// This thread is pinned to the top of its parent `GUILD_FORUM` or `GUILD_MEDIA` channel (`1 << 1`).
    pub const PINNED: ChannelFlags = ChannelFlags(1 << 1);

    /// Whether a tag is required to be specified when creating a thread in a `GUILD_FORUM` or `GUILD_MEDIA` channel (`1 << 4`).
    pub const REQUIRE_TAG: ChannelFlags = ChannelFlags(1 << 4);

    /// When set, hides the embedded media download options (available only for media channels) (`1 << 15`).
    pub const HIDE_MEDIA_DOWNLOAD_OPTIONS: ChannelFlags = ChannelFlags(1 << 15);

    /// This channel's metadata has been obfuscated because the current user/bot cannot view it (`1 << 17`).
    ///
    /// Only sent over the Gateway (never via HTTP). When this flag is set, channels must never
    /// be assumed to carry metadata (name, topic, bitrate, permissions, etc.).
    pub const CHANNEL_OBFUSCATED: ChannelFlags = ChannelFlags(1 << 17);

    /// This channel is a spoiler channel (`1 << 21`).
    pub const IS_SPOILER_CHANNEL: ChannelFlags = ChannelFlags(1 << 21);

    /// Wrap raw bits, keeping any unknown bits for forward compatibility.
    pub const fn from_bits_retain(bits: u32) -> Self {
        Self(bits)
    }

    /// Raw bits.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// True if every flag in `other` is set.
    pub const fn contains(self, other: ChannelFlags) -> bool {
        (self.0 & other.0) == other.0
    }

    /// True if any flag in `other` is set.
    pub const fn intersects(self, other: ChannelFlags) -> bool {
        (self.0 & other.0) != 0
    }

    /// True if no flags are set.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl BitOr for ChannelFlags {
    type Output = ChannelFlags;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for ChannelFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}
