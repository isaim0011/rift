use rift_core::{Capabilities, Intents};

pub const INTENT_GUILDS: u32 = Intents::GUILDS.bits();
pub const INTENT_GUILD_MEMBERS: u32 = Intents::GUILD_MEMBERS.bits();
pub const INTENT_MESSAGE_CONTENT: u32 = Intents::MESSAGE_CONTENT.bits();
pub const INTENT_PRIVILEGED: u32 = Intents::PRIVILEGED.bits();
pub const INTENT_ALL_KNOWN: u32 = Intents::ALL_KNOWN;

pub const CAPABILITY_NONE: u32 = Capabilities::NONE.bits();
pub const CAPABILITY_CHANNEL_OBFUSCATION: u32 = Capabilities::CHANNEL_OBFUSCATION.bits();
pub const CAPABILITY_IS_COMPONENTS_V2: u32 = 1 << 16;

#[cfg(feature = "pyo3-binding")]
pub mod pyo3_impl {
    use super::*;
    use pyo3::prelude::*;

    #[pyclass(name = "Intents")]
    #[derive(Clone, Debug, Default)]
    pub struct PyIntentsClass;

    #[pymethods]
    impl PyIntentsClass {
        #[classattr]
        pub const GUILDS: u32 = INTENT_GUILDS;
        #[classattr]
        pub const GUILD_MEMBERS: u32 = INTENT_GUILD_MEMBERS;
        #[classattr]
        pub const MESSAGE_CONTENT: u32 = INTENT_MESSAGE_CONTENT;
        #[classattr]
        pub const PRIVILEGED: u32 = INTENT_PRIVILEGED;
        #[classattr]
        pub const ALL_KNOWN: u32 = INTENT_ALL_KNOWN;
    }

    #[pyclass(name = "Capabilities")]
    #[derive(Clone, Debug, Default)]
    pub struct PyCapabilitiesClass;

    #[pymethods]
    impl PyCapabilitiesClass {
        #[classattr]
        pub const NONE: u32 = CAPABILITY_NONE;
        #[classattr]
        pub const CHANNEL_OBFUSCATION: u32 = CAPABILITY_CHANNEL_OBFUSCATION;
        #[classattr]
        pub const IS_COMPONENTS_V2: u32 = CAPABILITY_IS_COMPONENTS_V2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_py_intents_and_capabilities() {
        assert_eq!(INTENT_GUILDS, 1);
        assert_eq!(INTENT_MESSAGE_CONTENT, 1 << 15);
        assert_eq!(CAPABILITY_CHANNEL_OBFUSCATION, 1 << 15);
    }
}
