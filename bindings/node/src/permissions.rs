use rift_core::permissions::apply_implicit_text;
use rift_core::Permissions;
use serde::{Deserialize, Serialize};

/// Channel permission overwrite input representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverwriteInput {
    /// ID of role or member target.
    pub id: u64,
    /// 0 for Role, 1 for Member.
    pub kind: u8,
    /// Allowed permission bits.
    pub allow: u64,
    /// Denied permission bits.
    pub deny: u64,
}

// Re-export standard permission constants as u64
pub const CREATE_INSTANT_INVITE: u64 = Permissions::CREATE_INSTANT_INVITE.bits();
pub const KICK_MEMBERS: u64 = Permissions::KICK_MEMBERS.bits();
pub const BAN_MEMBERS: u64 = Permissions::BAN_MEMBERS.bits();
pub const ADMINISTRATOR: u64 = Permissions::ADMINISTRATOR.bits();
pub const MANAGE_CHANNELS: u64 = Permissions::MANAGE_CHANNELS.bits();
pub const MANAGE_GUILD: u64 = Permissions::MANAGE_GUILD.bits();
pub const ADD_REACTIONS: u64 = Permissions::ADD_REACTIONS.bits();
pub const VIEW_AUDIT_LOG: u64 = Permissions::VIEW_AUDIT_LOG.bits();
pub const PRIORITY_SPEAKER: u64 = Permissions::PRIORITY_SPEAKER.bits();
pub const STREAM: u64 = Permissions::STREAM.bits();
pub const VIEW_CHANNEL: u64 = Permissions::VIEW_CHANNEL.bits();
pub const SEND_MESSAGES: u64 = Permissions::SEND_MESSAGES.bits();
pub const SEND_TTS_MESSAGES: u64 = Permissions::SEND_TTS_MESSAGES.bits();
pub const MANAGE_MESSAGES: u64 = Permissions::MANAGE_MESSAGES.bits();
pub const EMBED_LINKS: u64 = Permissions::EMBED_LINKS.bits();
pub const ATTACH_FILES: u64 = Permissions::ATTACH_FILES.bits();
pub const READ_MESSAGE_HISTORY: u64 = Permissions::READ_MESSAGE_HISTORY.bits();
pub const MENTION_EVERYONE: u64 = Permissions::MENTION_EVERYONE.bits();
pub const USE_EXTERNAL_EMOJIS: u64 = Permissions::USE_EXTERNAL_EMOJIS.bits();
pub const VIEW_GUILD_INSIGHTS: u64 = Permissions::VIEW_GUILD_INSIGHTS.bits();
pub const CONNECT: u64 = Permissions::CONNECT.bits();
pub const SPEAK: u64 = Permissions::SPEAK.bits();
pub const MUTE_MEMBERS: u64 = Permissions::MUTE_MEMBERS.bits();
pub const DEAFEN_MEMBERS: u64 = Permissions::DEAFEN_MEMBERS.bits();
pub const MOVE_MEMBERS: u64 = Permissions::MOVE_MEMBERS.bits();
pub const USE_VAD: u64 = Permissions::USE_VAD.bits();
pub const CHANGE_NICKNAME: u64 = Permissions::CHANGE_NICKNAME.bits();
pub const MANAGE_NICKNAMES: u64 = Permissions::MANAGE_NICKNAMES.bits();
pub const MANAGE_ROLES: u64 = Permissions::MANAGE_ROLES.bits();
pub const MANAGE_WEBHOOKS: u64 = Permissions::MANAGE_WEBHOOKS.bits();
pub const MANAGE_GUILD_EXPRESSIONS: u64 = Permissions::MANAGE_GUILD_EXPRESSIONS.bits();
pub const USE_APPLICATION_COMMANDS: u64 = Permissions::USE_APPLICATION_COMMANDS.bits();
pub const REQUEST_TO_SPEAK: u64 = Permissions::REQUEST_TO_SPEAK.bits();
pub const MANAGE_EVENTS: u64 = Permissions::MANAGE_EVENTS.bits();
pub const MANAGE_THREADS: u64 = Permissions::MANAGE_THREADS.bits();
pub const CREATE_PUBLIC_THREADS: u64 = Permissions::CREATE_PUBLIC_THREADS.bits();
pub const CREATE_PRIVATE_THREADS: u64 = Permissions::CREATE_PRIVATE_THREADS.bits();
pub const USE_EXTERNAL_STICKERS: u64 = Permissions::USE_EXTERNAL_STICKERS.bits();
pub const SEND_MESSAGES_IN_THREADS: u64 = Permissions::SEND_MESSAGES_IN_THREADS.bits();
pub const USE_EMBEDDED_ACTIVITIES: u64 = Permissions::USE_EMBEDDED_ACTIVITIES.bits();
pub const MODERATE_MEMBERS: u64 = Permissions::MODERATE_MEMBERS.bits();
pub const VIEW_CREATOR_MONETIZATION_ANALYTICS: u64 = Permissions::VIEW_CREATOR_MONETIZATION_ANALYTICS.bits();
pub const USE_SOUNDBOARD: u64 = Permissions::USE_SOUNDBOARD.bits();
pub const CREATE_GUILD_EXPRESSIONS: u64 = Permissions::CREATE_GUILD_EXPRESSIONS.bits();
pub const CREATE_EVENTS: u64 = Permissions::CREATE_EVENTS.bits();
pub const USE_EXTERNAL_SOUNDS: u64 = Permissions::USE_EXTERNAL_SOUNDS.bits();
pub const SEND_VOICE_MESSAGES: u64 = Permissions::SEND_VOICE_MESSAGES.bits();
pub const SET_VOICE_CHANNEL_STATUS: u64 = Permissions::SET_VOICE_CHANNEL_STATUS.bits();
pub const SEND_POLLS: u64 = Permissions::SEND_POLLS.bits();
pub const USE_EXTERNAL_APPS: u64 = Permissions::USE_EXTERNAL_APPS.bits();
pub const PIN_MESSAGES: u64 = Permissions::PIN_MESSAGES.bits();
pub const BYPASS_SLOWMODE: u64 = Permissions::BYPASS_SLOWMODE.bits();
pub const ALL: u64 = Permissions::ALL.bits();

/// Checks if the given permissions set contains the required permissions.
/// Returns `true` if `ADMINISTRATOR` is present or all `required` bits are set.
pub fn has_permission(permissions: u64, required: u64) -> bool {
    let p = Permissions::from_bits_retain(permissions);
    let r = Permissions::from_bits_retain(required);
    p.has(r)
}

/// Computes final channel-level effective permissions given base permissions,
/// a list of overwrites, the target member ID, and the member's assigned role IDs.
pub fn compute_channel_permissions(
    base: u64,
    overwrites: &[OverwriteInput],
    guild_id: u64,
    member_id: u64,
    member_roles: &[u64],
) -> u64 {
    let base_p = Permissions::from_bits_retain(base);
    if base_p.contains(Permissions::ADMINISTRATOR) {
        return Permissions::ALL.bits();
    }

    let mut p = base_p;

    // 1. @everyone overwrite (kind == 0 && id == guild_id)
    if let Some(everyone_ow) = overwrites.iter().find(|o| o.kind == 0 && o.id == guild_id) {
        let allow = Permissions::from_bits_retain(everyone_ow.allow);
        let deny = Permissions::from_bits_retain(everyone_ow.deny);
        p = (p & !deny) | allow;
    }

    // 2. Role overwrites (aggregate denies then allows for matching roles)
    let mut role_allow = Permissions::NONE;
    let mut role_deny = Permissions::NONE;
    for ow in overwrites.iter().filter(|o| o.kind == 0 && member_roles.contains(&o.id)) {
        role_allow |= Permissions::from_bits_retain(ow.allow);
        role_deny |= Permissions::from_bits_retain(ow.deny);
    }
    p = (p & !role_deny) | role_allow;

    // 3. Member overwrite (specific to this user)
    if let Some(member_ow) = overwrites.iter().find(|o| o.kind == 1 && o.id == member_id) {
        let allow = Permissions::from_bits_retain(member_ow.allow);
        let deny = Permissions::from_bits_retain(member_ow.deny);
        p = (p & !deny) | allow;
    }

    // 4. Implicit text permissions
    apply_implicit_text(p).bits()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_permission() {
        assert!(has_permission(ADMINISTRATOR, SEND_MESSAGES));
        assert!(has_permission(SEND_MESSAGES | VIEW_CHANNEL, SEND_MESSAGES));
        assert!(!has_permission(VIEW_CHANNEL, SEND_MESSAGES));
    }

    #[test]
    fn test_compute_channel_permissions_admin_bypass() {
        let overwrites = vec![OverwriteInput {
            id: 123,
            kind: 0,
            allow: 0,
            deny: ALL,
        }];
        let computed = compute_channel_permissions(ADMINISTRATOR, &overwrites, 123, 456, &[123]);
        assert_eq!(computed, ALL);
    }

    #[test]
    fn test_compute_channel_permissions_role_overwrites() {
        let guild_id = 1000;
        let member_id = 2000;
        let role_a = 3001;
        let role_b = 3002;

        let base = VIEW_CHANNEL;
        let overwrites = vec![
            // Role A denies SEND_MESSAGES
            OverwriteInput {
                id: role_a,
                kind: 0,
                allow: 0,
                deny: SEND_MESSAGES,
            },
            // Role B allows SEND_MESSAGES (allow beats deny across roles)
            OverwriteInput {
                id: role_b,
                kind: 0,
                allow: SEND_MESSAGES,
                deny: 0,
            },
        ];

        let result = compute_channel_permissions(base, &overwrites, guild_id, member_id, &[role_a, role_b]);
        assert_ne!(result & SEND_MESSAGES, 0);
    }

    #[test]
    fn test_implicit_view_channel_revocation() {
        let guild_id = 1000;
        let member_id = 2000;

        let base = SEND_MESSAGES | VIEW_CHANNEL;
        let overwrites = vec![
            // Member overwrite denies VIEW_CHANNEL
            OverwriteInput {
                id: member_id,
                kind: 1,
                allow: 0,
                deny: VIEW_CHANNEL,
            },
        ];

        let result = compute_channel_permissions(base, &overwrites, guild_id, member_id, &[]);
        assert_eq!(result, 0);
    }
}
