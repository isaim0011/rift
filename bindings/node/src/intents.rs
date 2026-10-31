use rift_core::{Capabilities, Intents};

// Gateway Intents
pub const INTENT_GUILDS: u32 = Intents::GUILDS.bits();
pub const INTENT_GUILD_MEMBERS: u32 = Intents::GUILD_MEMBERS.bits();
pub const INTENT_GUILD_MODERATION: u32 = Intents::GUILD_MODERATION.bits();
pub const INTENT_GUILD_EXPRESSIONS: u32 = Intents::GUILD_EXPRESSIONS.bits();
pub const INTENT_GUILD_INTEGRATIONS: u32 = Intents::GUILD_INTEGRATIONS.bits();
pub const INTENT_GUILD_WEBHOOKS: u32 = Intents::GUILD_WEBHOOKS.bits();
pub const INTENT_GUILD_INVITES: u32 = Intents::GUILD_INVITES.bits();
pub const INTENT_GUILD_VOICE_STATES: u32 = Intents::GUILD_VOICE_STATES.bits();
pub const INTENT_GUILD_PRESENCES: u32 = Intents::GUILD_PRESENCES.bits();
pub const INTENT_GUILD_MESSAGES: u32 = Intents::GUILD_MESSAGES.bits();
pub const INTENT_GUILD_MESSAGE_REACTIONS: u32 = Intents::GUILD_MESSAGE_REACTIONS.bits();
pub const INTENT_GUILD_MESSAGE_TYPING: u32 = Intents::GUILD_MESSAGE_TYPING.bits();
pub const INTENT_DIRECT_MESSAGES: u32 = Intents::DIRECT_MESSAGES.bits();
pub const INTENT_DIRECT_MESSAGE_REACTIONS: u32 = Intents::DIRECT_MESSAGE_REACTIONS.bits();
pub const INTENT_DIRECT_MESSAGE_TYPING: u32 = Intents::DIRECT_MESSAGE_TYPING.bits();
pub const INTENT_MESSAGE_CONTENT: u32 = Intents::MESSAGE_CONTENT.bits();
pub const INTENT_GUILD_SCHEDULED_EVENTS: u32 = Intents::GUILD_SCHEDULED_EVENTS.bits();
pub const INTENT_AUTO_MODERATION_CONFIGURATION: u32 = Intents::AUTO_MODERATION_CONFIGURATION.bits();
pub const INTENT_AUTO_MODERATION_EXECUTION: u32 = Intents::AUTO_MODERATION_EXECUTION.bits();
pub const INTENT_GUILD_MESSAGE_POLLS: u32 = Intents::GUILD_MESSAGE_POLLS.bits();
pub const INTENT_DIRECT_MESSAGE_POLLS: u32 = Intents::DIRECT_MESSAGE_POLLS.bits();
pub const INTENT_PRIVILEGED: u32 = Intents::PRIVILEGED.bits();
pub const INTENT_ALL_KNOWN: u32 = Intents::ALL_KNOWN;

// Gateway Capabilities
pub const CAPABILITY_NONE: u32 = Capabilities::NONE.bits();
pub const CAPABILITY_CHANNEL_OBFUSCATION: u32 = Capabilities::CHANNEL_OBFUSCATION.bits();
pub const CAPABILITY_IS_COMPONENTS_V2: u32 = 1 << 16;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_bits() {
        assert_eq!(INTENT_GUILDS, 1 << 0);
        assert_eq!(INTENT_GUILD_MEMBERS, 1 << 1);
        assert_eq!(INTENT_MESSAGE_CONTENT, 1 << 15);
        assert_eq!(INTENT_PRIVILEGED, (1 << 1) | (1 << 8) | (1 << 15));
    }

    #[test]
    fn test_capability_bits() {
        assert_eq!(CAPABILITY_NONE, 0);
        assert_eq!(CAPABILITY_CHANNEL_OBFUSCATION, 1 << 15);
        assert_eq!(CAPABILITY_IS_COMPONENTS_V2, 1 << 16);
    }
}
