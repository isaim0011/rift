use rift_core::permissions::apply_implicit_text;
use rift_core::Permissions;

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

/// Overwrite representation: (id, kind: 0=role 1=member, allow, deny)
pub type OverwriteTuple = (u64, u8, u64, u64);

/// Permissions operations for Python.
#[derive(Clone, Debug, Default)]
pub struct PyPermissions;

impl PyPermissions {
    /// True if `ADMINISTRATOR` is set or all `required` bits are set.
    pub fn has_permission(permissions: u64, required: u64) -> bool {
        let p = Permissions::from_bits_retain(permissions);
        let r = Permissions::from_bits_retain(required);
        p.has(r)
    }

    /// Computes channel effective permissions following Discord's priority order.
    pub fn compute_channel_permissions(
        base: u64,
        overwrites: &[OverwriteTuple],
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
        if let Some((_, _, allow, deny)) = overwrites.iter().find(|(id, kind, _, _)| *kind == 0 && *id == guild_id) {
            p = (p & !Permissions::from_bits_retain(*deny)) | Permissions::from_bits_retain(*allow);
        }

        // 2. Role overwrites (allow beats deny across roles)
        let mut role_allow = Permissions::NONE;
        let mut role_deny = Permissions::NONE;
        for (_id, _kind, allow, deny) in overwrites.iter().filter(|(id, kind, _, _)| *kind == 0 && member_roles.contains(id)) {
            role_allow |= Permissions::from_bits_retain(*allow);
            role_deny |= Permissions::from_bits_retain(*deny);
        }
        p = (p & !role_deny) | role_allow;

        // 3. Member overwrite
        if let Some((_, _, allow, deny)) = overwrites.iter().find(|(id, kind, _, _)| *kind == 1 && *id == member_id) {
            p = (p & !Permissions::from_bits_retain(*deny)) | Permissions::from_bits_retain(*allow);
        }

        // 4. Implicit text permissions
        apply_implicit_text(p).bits()
    }
}

#[cfg(feature = "pyo3-binding")]
pub mod pyo3_impl {
    use super::*;
    use pyo3::prelude::*;

    #[pyclass(name = "Permissions")]
    #[derive(Clone, Debug, Default)]
    pub struct PyPermissionsClass;

    #[pymethods]
    impl PyPermissionsClass {
        #[classattr]
        pub const ADMINISTRATOR: u64 = super::ADMINISTRATOR;
        #[classattr]
        pub const VIEW_CHANNEL: u64 = super::VIEW_CHANNEL;
        #[classattr]
        pub const SEND_MESSAGES: u64 = super::SEND_MESSAGES;
        #[classattr]
        pub const ALL: u64 = super::ALL;

        #[staticmethod]
        pub fn has_permission(permissions: u64, required: u64) -> bool {
            PyPermissions::has_permission(permissions, required)
        }

        #[staticmethod]
        pub fn compute_channel_permissions(
            base: u64,
            overwrites: Vec<(u64, u8, u64, u64)>,
            guild_id: u64,
            member_id: u64,
            member_roles: Vec<u64>,
        ) -> u64 {
            PyPermissions::compute_channel_permissions(base, &overwrites, guild_id, member_id, &member_roles)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_py_permissions_has() {
        assert!(PyPermissions::has_permission(ADMINISTRATOR, SEND_MESSAGES));
        assert!(PyPermissions::has_permission(SEND_MESSAGES | VIEW_CHANNEL, SEND_MESSAGES));
        assert!(!PyPermissions::has_permission(VIEW_CHANNEL, SEND_MESSAGES));
    }

    #[test]
    fn test_py_permissions_compute() {
        let overwrites = vec![
            (3001, 0, 0, SEND_MESSAGES),
            (3002, 0, SEND_MESSAGES, 0),
        ];
        let result = PyPermissions::compute_channel_permissions(
            VIEW_CHANNEL,
            &overwrites,
            1000,
            2000,
            &[3001, 3002],
        );
        assert_ne!(result & SEND_MESSAGES, 0);
    }
}
