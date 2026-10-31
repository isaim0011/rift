//! Permissions: bit flags, overwrite resolution, implicit rules.
//!
//! Every constant and rule here comes from the Permissions page of the docs.
//! Permissions are serialized by Discord as decimal *strings*; use [`FromStr`].

use std::fmt;
use std::num::ParseIntError;
use std::ops::{BitAnd, BitOr, BitOrAssign, Not};
use std::str::FromStr;

/// A permission bit set. Unknown (future) bits are preserved, never dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Permissions(u64);

macro_rules! permissions {
    ($($name:ident = $bit:expr),* $(,)?) => {
        impl Permissions {
            $(#[allow(missing_docs)] pub const $name: Permissions = Permissions(1u64 << $bit);)*
            /// Every permission currently documented.
            pub const ALL: Permissions = Permissions(0 $(| (1u64 << $bit))*);
        }
    };
}

permissions! {
    CREATE_INSTANT_INVITE = 0,
    KICK_MEMBERS = 1,
    BAN_MEMBERS = 2,
    ADMINISTRATOR = 3,
    MANAGE_CHANNELS = 4,
    MANAGE_GUILD = 5,
    ADD_REACTIONS = 6,
    VIEW_AUDIT_LOG = 7,
    PRIORITY_SPEAKER = 8,
    STREAM = 9,
    VIEW_CHANNEL = 10,
    SEND_MESSAGES = 11,
    SEND_TTS_MESSAGES = 12,
    MANAGE_MESSAGES = 13,
    EMBED_LINKS = 14,
    ATTACH_FILES = 15,
    READ_MESSAGE_HISTORY = 16,
    MENTION_EVERYONE = 17,
    USE_EXTERNAL_EMOJIS = 18,
    VIEW_GUILD_INSIGHTS = 19,
    CONNECT = 20,
    SPEAK = 21,
    MUTE_MEMBERS = 22,
    DEAFEN_MEMBERS = 23,
    MOVE_MEMBERS = 24,
    USE_VAD = 25,
    CHANGE_NICKNAME = 26,
    MANAGE_NICKNAMES = 27,
    MANAGE_ROLES = 28,
    MANAGE_WEBHOOKS = 29,
    MANAGE_GUILD_EXPRESSIONS = 30,
    USE_APPLICATION_COMMANDS = 31,
    REQUEST_TO_SPEAK = 32,
    MANAGE_EVENTS = 33,
    MANAGE_THREADS = 34,
    CREATE_PUBLIC_THREADS = 35,
    CREATE_PRIVATE_THREADS = 36,
    USE_EXTERNAL_STICKERS = 37,
    SEND_MESSAGES_IN_THREADS = 38,
    USE_EMBEDDED_ACTIVITIES = 39,
    MODERATE_MEMBERS = 40,
    VIEW_CREATOR_MONETIZATION_ANALYTICS = 41,
    USE_SOUNDBOARD = 42,
    CREATE_GUILD_EXPRESSIONS = 43,
    CREATE_EVENTS = 44,
    USE_EXTERNAL_SOUNDS = 45,
    SEND_VOICE_MESSAGES = 46,
    SET_VOICE_CHANNEL_STATUS = 48,
    SEND_POLLS = 49,
    USE_EXTERNAL_APPS = 50,
    PIN_MESSAGES = 51,
    BYPASS_SLOWMODE = 52,
}

impl Permissions {
    /// No permissions.
    pub const NONE: Permissions = Permissions(0);

    /// Permissions the docs mark with `*`: they require the owner to have 2FA
    /// when the guild has server-wide 2FA enabled.
    pub const TWO_FACTOR_REQUIRED: Permissions = Permissions(
        Self::KICK_MEMBERS.0
            | Self::BAN_MEMBERS.0
            | Self::ADMINISTRATOR.0
            | Self::MANAGE_CHANNELS.0
            | Self::MANAGE_GUILD.0
            | Self::MANAGE_MESSAGES.0
            | Self::MANAGE_ROLES.0
            | Self::MANAGE_WEBHOOKS.0
            | Self::MANAGE_GUILD_EXPRESSIONS.0
            | Self::MANAGE_THREADS.0
            | Self::VIEW_CREATOR_MONETIZATION_ANALYTICS.0,
    );

    /// Wrap raw bits, keeping any bits this version doesn't know about.
    pub const fn from_bits_retain(bits: u64) -> Self {
        Self(bits)
    }

    /// Raw bits.
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Bits set that this version doesn't know about (newer Discord permissions).
    pub const fn unknown_bits(self) -> u64 {
        self.0 & !Self::ALL.0
    }

    /// True if every permission in `other` is set.
    pub const fn contains(self, other: Permissions) -> bool {
        self.0 & other.0 == other.0
    }

    /// True if any permission in `other` is set.
    pub const fn intersects(self, other: Permissions) -> bool {
        self.0 & other.0 != 0
    }

    /// True if nothing is set.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Like [`contains`](Self::contains), but `ADMINISTRATOR` grants everything.
    pub const fn has(self, other: Permissions) -> bool {
        self.contains(Self::ADMINISTRATOR) || self.contains(other)
    }
}

impl BitOr for Permissions {
    type Output = Permissions;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Permissions {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for Permissions {
    type Output = Permissions;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl Not for Permissions {
    type Output = Permissions;
    fn not(self) -> Self {
        Self(!self.0)
    }
}

impl FromStr for Permissions {
    type Err = ParseIntError;
    /// Parse Discord's decimal-string form.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u64>().map(Self)
    }
}

impl fmt::Display for Permissions {
    /// Discord's decimal-string form.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A channel permission overwrite (`allow` / `deny`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Overwrite {
    /// Explicitly allowed.
    pub allow: Permissions,
    /// Explicitly denied.
    pub deny: Permissions,
}

/// Guild-level permissions. Owners and `ADMINISTRATOR` get everything.
pub fn compute_base(
    is_owner: bool,
    everyone: Permissions,
    member_roles: impl IntoIterator<Item = Permissions>,
) -> Permissions {
    if is_owner {
        return Permissions::ALL;
    }
    let mut p = everyone;
    for r in member_roles {
        p |= r;
    }
    if p.contains(Permissions::ADMINISTRATOR) {
        Permissions::ALL
    } else {
        p
    }
}

/// Channel-level permissions, in the docs' order: @everyone overwrite, then all
/// role overwrites (denies first, then allows, aggregated), then the member
/// overwrite (deny, then allow). `ADMINISTRATOR` bypasses everything.
pub fn compute_overwrites(
    base: Permissions,
    everyone: Option<Overwrite>,
    role_overwrites: impl IntoIterator<Item = Overwrite>,
    member: Option<Overwrite>,
) -> Permissions {
    if base.contains(Permissions::ADMINISTRATOR) {
        return Permissions::ALL;
    }
    let mut p = base;
    if let Some(o) = everyone {
        p = (p & !o.deny) | o.allow;
    }
    let mut allow = Permissions::NONE;
    let mut deny = Permissions::NONE;
    for o in role_overwrites {
        allow |= o.allow;
        deny |= o.deny;
    }
    p = (p & !deny) | allow;
    if let Some(o) = member {
        p = (p & !o.deny) | o.allow;
    }
    p
}

/// Apply the docs' implicit rules for text channels: without `VIEW_CHANNEL`
/// nothing else matters; without `SEND_MESSAGES`, `MENTION_EVERYONE`,
/// `SEND_TTS_MESSAGES`, `ATTACH_FILES` and `EMBED_LINKS` are ignored.
/// Administrators are untouched.
pub fn apply_implicit_text(p: Permissions) -> Permissions {
    if p.contains(Permissions::ADMINISTRATOR) {
        return p;
    }
    if !p.contains(Permissions::VIEW_CHANNEL) {
        return Permissions::NONE;
    }
    if !p.contains(Permissions::SEND_MESSAGES) {
        let dependent = Permissions::MENTION_EVERYONE
            | Permissions::SEND_TTS_MESSAGES
            | Permissions::ATTACH_FILES
            | Permissions::EMBED_LINKS;
        return p & !dependent;
    }
    p
}

/// Timed-out members keep only `VIEW_CHANNEL` and `READ_MESSAGE_HISTORY`.
/// Owners and administrators are exempt.
pub fn apply_timeout(p: Permissions, is_owner: bool) -> Permissions {
    if is_owner || p.contains(Permissions::ADMINISTRATOR) {
        p
    } else {
        p & (Permissions::VIEW_CHANNEL | Permissions::READ_MESSAGE_HISTORY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: fn(Permissions) -> Permissions = |p| p;

    #[test]
    fn documented_example() {
        let p = Permissions::ADD_REACTIONS | Permissions::SEND_MESSAGES;
        assert_eq!(p.bits(), 2112);
        assert_eq!(p.to_string(), "2112");
        assert!(!p.contains(Permissions::KICK_MEMBERS));
    }

    #[test]
    fn flag_table_is_complete() {
        // 0..=46 and 48..=52 are documented (47 is not).
        assert_eq!(Permissions::ALL.bits().count_ones(), 52);
        assert!(!Permissions::ALL.contains(Permissions::from_bits_retain(1 << 47)));
        assert_eq!(Permissions::BYPASS_SLOWMODE.bits(), 1 << 52);
    }

    #[test]
    fn parses_role_string_and_keeps_unknown_bits() {
        let p: Permissions = "66321471".parse().unwrap();
        assert_eq!(p.bits(), 66_321_471);
        assert_eq!(p.unknown_bits(), 0);
        let future = Permissions::from_bits_retain(1 << 60);
        assert_eq!(future.unknown_bits(), 1 << 60);
    }

    #[test]
    fn owner_and_admin_get_everything() {
        assert_eq!(compute_base(true, Permissions::NONE, []), Permissions::ALL);
        let r = Permissions::ADMINISTRATOR;
        assert_eq!(compute_base(false, Permissions::NONE, [r]), Permissions::ALL);
        // Admin ignores overwrites entirely.
        let deny_all = Overwrite { allow: Permissions::NONE, deny: Permissions::ALL };
        assert_eq!(
            compute_overwrites(Permissions::ALL, Some(deny_all), [deny_all], Some(deny_all)),
            Permissions::ALL
        );
    }

    #[test]
    fn everyone_deny_applies() {
        let base = Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES;
        let ow = Overwrite { allow: Permissions::NONE, deny: Permissions::SEND_MESSAGES };
        let p = compute_overwrites(base, Some(ow), [], None);
        assert_eq!(P(p), Permissions::VIEW_CHANNEL);
    }

    #[test]
    fn docs_view_channel_example_allow_beats_deny_across_roles() {
        // Role A denies VIEW_CHANNEL, role B allows it: user can view.
        let a = Overwrite { allow: Permissions::NONE, deny: Permissions::VIEW_CHANNEL };
        let b = Overwrite { allow: Permissions::VIEW_CHANNEL, deny: Permissions::NONE };
        let p = compute_overwrites(Permissions::NONE, None, [a, b], None);
        assert!(p.contains(Permissions::VIEW_CHANNEL));
    }

    #[test]
    fn member_overwrite_beats_role_overwrite() {
        let role = Overwrite { allow: Permissions::NONE, deny: Permissions::SEND_MESSAGES };
        let member = Overwrite { allow: Permissions::SEND_MESSAGES, deny: Permissions::NONE };
        let base = Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES;
        let p = compute_overwrites(base, None, [role], Some(member));
        assert!(p.contains(Permissions::SEND_MESSAGES));
    }

    #[test]
    fn implicit_rules() {
        let all_text = Permissions::VIEW_CHANNEL
            | Permissions::SEND_MESSAGES
            | Permissions::EMBED_LINKS
            | Permissions::ATTACH_FILES;
        assert_eq!(apply_implicit_text(all_text), all_text);

        let no_view = Permissions::SEND_MESSAGES | Permissions::EMBED_LINKS;
        assert_eq!(apply_implicit_text(no_view), Permissions::NONE);

        let no_send = Permissions::VIEW_CHANNEL | Permissions::EMBED_LINKS | Permissions::ADD_REACTIONS;
        assert_eq!(
            apply_implicit_text(no_send),
            Permissions::VIEW_CHANNEL | Permissions::ADD_REACTIONS
        );
    }

    #[test]
    fn timeout_keeps_only_view_and_history() {
        let p = Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY;
        assert_eq!(
            apply_timeout(p, false),
            Permissions::VIEW_CHANNEL | Permissions::READ_MESSAGE_HISTORY
        );
        assert_eq!(apply_timeout(p, true), p);
        assert_eq!(apply_timeout(Permissions::ALL, false), Permissions::ALL);
    }

    #[test]
    fn has_respects_administrator() {
        assert!(Permissions::ADMINISTRATOR.has(Permissions::BAN_MEMBERS));
        assert!(!Permissions::KICK_MEMBERS.has(Permissions::BAN_MEMBERS));
    }
}
