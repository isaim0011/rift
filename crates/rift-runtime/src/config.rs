use rift_core::{Capabilities, Intents};
use std::collections::HashSet;
use std::sync::Arc;

/// Backpressure policy for buffering gateway dispatch events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EventBufferPolicy {
    /// Unbounded channel (default): events buffer indefinitely without dropping.
    #[default]
    Unbounded,
    /// Bounded buffer with maximum capacity.
    Bounded {
        /// Maximum number of queued events.
        capacity: usize,
        /// If `true`, the oldest event is dropped when full to make room for new events.
        /// If `false`, incoming events are dropped when full.
        drop_oldest: bool,
    },
}

/// Everything one shard needs to connect.
#[derive(Clone, Debug)]
pub struct ShardConfig {
    /// Bot token (never logged).
    pub token: String,
    /// Gateway intents.
    pub intents: Intents,
    /// Gateway capabilities (Identify `capabilities`). Off by default; see
    /// [`Capabilities::CHANNEL_OBFUSCATION`] for the November 2026 change.
    pub capabilities: Capabilities,
    /// This shard's id.
    pub shard_id: u32,
    /// Total number of shards.
    pub num_shards: u32,
    /// Base gateway URL (from Get Gateway Bot; cache it).
    pub gateway_url: String,
    /// If set, only these dispatch events are forwarded (plus `READY` and
    /// `RESUMED`, which always are). Others are dropped before any allocation.
    pub event_filter: Option<Arc<HashSet<String>>>,
    /// Backpressure policy for the event channel.
    pub event_buffer_policy: EventBufferPolicy,
}

impl ShardConfig {
    /// Single-shard config pointing at the public gateway.
    pub fn new(token: impl Into<String>, intents: Intents) -> Self {
        Self {
            token: token.into(),
            intents,
            capabilities: Capabilities::NONE,
            shard_id: 0,
            num_shards: 1,
            gateway_url: "wss://gateway.discord.gg".to_owned(),
            event_filter: None,
            event_buffer_policy: EventBufferPolicy::Unbounded,
        }
    }

    /// Forward only these dispatch events (plus `READY`/`RESUMED`).
    pub fn only_events<I, S>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.event_filter = Some(Arc::new(names.into_iter().map(Into::into).collect()));
        self
    }

    /// Opt in to gateway capabilities.
    pub fn with_capabilities(mut self, capabilities: Capabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Configure the event backpressure policy.
    pub fn with_event_buffer_policy(mut self, policy: EventBufferPolicy) -> Self {
        self.event_buffer_policy = policy;
        self
    }

    /// Configure the event backpressure policy.
    pub fn with_buffer_policy(mut self, policy: EventBufferPolicy) -> Self {
        self.event_buffer_policy = policy;
        self
    }

    /// Returns the configured event backpressure policy.
    pub fn buffer_policy(&self) -> EventBufferPolicy {
        self.event_buffer_policy
    }
}
