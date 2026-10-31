use crate::error::Error;
use crate::supervisor::{supervise, ShardResult};
use rift_core::{Capabilities, Intents, Snowflake};
use rift_rest::{ReqwestTransport, RestClient};
use rift_runtime::{
    now_ms, run_shard, Command, Event, EventBufferPolicy, EventReceiver, RequestGuildMembers,
    RequestSoundboardSounds, ShardConfig, SharedBudget, UpdatePresence, UpdateVoiceState,
};
use std::collections::HashSet;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::{JoinHandle, JoinSet};

/// Discord limits Identifies to `max_concurrency` per 5 seconds; a small margin
/// keeps us safely under it.
const DEFAULT_WAVE_GAP: Duration = Duration::from_millis(5_250);

/// Identifies kept untouched so a crash loop can never reach the 1000/day
/// limit that resets the bot token.
const DEFAULT_IDENTIFY_RESERVE: u32 = 10;

/// Builds a [`Client`].
pub struct ClientBuilder {
    token: String,
    intents: Intents,
    capabilities: Capabilities,
    shard_count: Option<u32>,
    identify_reserve: u32,
    contact_url: String,
    version: String,
    wave_gap: Duration,
    event_filter: Option<Arc<HashSet<String>>>,
    event_buffer_policy: EventBufferPolicy,
    #[cfg(feature = "cache")]
    cache: Option<Arc<std::sync::RwLock<rift_cache::Cache>>>,
}

impl ClientBuilder {
    /// Opt in to gateway capabilities (off by default).
    pub fn capabilities(mut self, capabilities: Capabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Override the shard count (default: Discord's recommendation).
    pub fn shard_count(mut self, n: u32) -> Self {
        self.shard_count = Some(n.max(1));
        self
    }

    /// Identifies to keep unused (default 10).
    pub fn identify_reserve(mut self, n: u32) -> Self {
        self.identify_reserve = n;
        self
    }

    /// URL for the `User-Agent` (`DiscordBot ($url, $version)`). Set this to your project's page.
    pub fn contact_url(mut self, url: impl Into<String>) -> Self {
        self.contact_url = url.into();
        self
    }

    /// Version for the `User-Agent` (default: this library's version).
    pub fn version(mut self, v: impl Into<String>) -> Self {
        self.version = v.into();
        self
    }

    /// Deliver only these gateway events (`READY` and `RESUMED` always are).
    ///
    /// Everything else is dropped inside the driver, before any allocation or
    /// channel send, so events you don't handle cost almost nothing.
    pub fn only_events<I, S>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.event_filter = Some(Arc::new(names.into_iter().map(Into::into).collect()));
        self
    }

    /// Configure the event backpressure policy (default: `Unbounded`).
    pub fn event_buffer_policy(mut self, policy: EventBufferPolicy) -> Self {
        self.event_buffer_policy = policy;
        self
    }

    /// Configure the event backpressure policy (default: `Unbounded`).
    pub fn buffer_policy(mut self, policy: EventBufferPolicy) -> Self {
        self.event_buffer_policy = policy;
        self
    }

    /// Attach an in-memory state cache to the client.
    ///
    /// The cache will automatically be updated with incoming events when using
    /// [`Client::next_gateway_event`] or [`Client::next_typed_event`].
    #[cfg(feature = "cache")]
    pub fn cache(mut self, cache: rift_cache::Cache) -> Self {
        self.cache = Some(Arc::new(std::sync::RwLock::new(cache)));
        self
    }

    /// Attach an in-memory state cache configured with the given [`rift_cache::CacheConfig`].
    #[cfg(feature = "cache")]
    pub fn with_cache(mut self, config: rift_cache::CacheConfig) -> Self {
        self.cache = Some(Arc::new(std::sync::RwLock::new(rift_cache::Cache::new(config))));
        self
    }

    /// Fetch `/gateway/bot`, then start launching shards in the background.
    ///
    /// Returns as soon as the REST call succeeds and the launch is scheduled;
    /// shards come online over the next seconds (one wave per ~5s).
    pub async fn build(self) -> Result<Client, Error> {
        let rest = Arc::new(RestClient::new(&self.token, &self.contact_url, &self.version)?);
        let gb = rest.gateway_bot().await?;

        let num_shards = self.shard_count.unwrap_or(gb.shards).max(1);
        check_identify_capacity(num_shards, gb.session_start_limit.remaining, self.identify_reserve)?;

        let max_concurrency =
            NonZeroU32::new(gb.session_start_limit.max_concurrency).unwrap_or(NonZeroU32::MIN);
        let waves = rift_core::shard::start_waves(num_shards, max_concurrency);
        let budget: SharedBudget = Arc::new(Mutex::new(gb.identify_budget(now_ms(), self.identify_reserve)));

        let (events_tx, events_rx) = rift_runtime::event_channel(self.event_buffer_policy);
        let mut senders = Vec::with_capacity(num_shards as usize);
        let mut receivers: Vec<Option<mpsc::Receiver<Command>>> = Vec::with_capacity(num_shards as usize);
        for _ in 0..num_shards {
            let (tx, rx) = mpsc::channel(64);
            senders.push(tx);
            receivers.push(Some(rx));
        }
        let (stop_tx, stop_rx) = mpsc::unbounded_channel::<()>();

        let event_filter = self.event_filter.clone();
        let token = self.token.clone();
        let intents = self.intents;
        let capabilities = self.capabilities;
        let url = gb.url.clone();
        let event_buffer_policy = self.event_buffer_policy;
        let start = move |id: u32, set: &mut JoinSet<ShardResult>| {
            let Some(rx) = receivers.get_mut(id as usize).and_then(Option::take) else {
                return;
            };
            let config = ShardConfig {
                token: token.clone(),
                intents,
                capabilities,
                shard_id: id,
                num_shards,
                gateway_url: url.clone(),
                event_filter: event_filter.clone(),
                event_buffer_policy,
            };
            let (budget, events) = (budget.clone(), events_tx.clone());
            set.spawn(async move { (id, run_shard(config, budget, events, rx).await) });
        };

        let stop_senders = senders.clone();
        let stop_all = move || {
            for s in &stop_senders {
                let _ = s.try_send(Command::Shutdown);
            }
        };

        let supervisor = tokio::spawn(supervise(waves, self.wave_gap, start, stop_all, stop_rx));

        #[cfg(feature = "cache")]
        let cache = self.cache;

        Ok(Client {
            rest,
            senders,
            events: events_rx,
            stop: stop_tx,
            supervisor,
            num_shards,
            #[cfg(feature = "cache")]
            cache,
        })
    }
}

/// A running bot: REST client plus all gateway shards.
pub struct Client {
    pub(crate) rest: Arc<RestClient<ReqwestTransport>>,
    pub(crate) senders: Vec<mpsc::Sender<Command>>,
    pub(crate) events: EventReceiver,
    pub(crate) stop: mpsc::UnboundedSender<()>,
    pub(crate) supervisor: JoinHandle<Result<(), rift_runtime::Error>>,
    pub(crate) num_shards: u32,
    #[cfg(feature = "cache")]
    pub(crate) cache: Option<Arc<std::sync::RwLock<rift_cache::Cache>>>,
}

impl Client {
    /// Start configuring a client.
    pub fn builder(token: impl Into<String>, intents: Intents) -> ClientBuilder {
        ClientBuilder {
            token: token.into(),
            intents,
            capabilities: Capabilities::NONE,
            shard_count: None,
            identify_reserve: DEFAULT_IDENTIFY_RESERVE,
            contact_url: "https://github.com/rift-rs/rift".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            wave_gap: DEFAULT_WAVE_GAP,
            event_filter: None,
            event_buffer_policy: EventBufferPolicy::Unbounded,
            #[cfg(feature = "cache")]
            cache: None,
        }
    }

    /// Next event from any shard. `None` once every shard has stopped.
    pub async fn next_event(&mut self) -> Option<Event> {
        self.events.recv().await
    }

    /// Next typed gateway event from any shard.
    ///
    /// Parses the raw runtime event into a [`rift_model::GatewayEvent`] and automatically
    /// updates the attached cache if one is configured.
    ///
    /// Returns `None` once every shard has stopped.
    pub async fn next_gateway_event(&mut self) -> Option<Result<rift_model::GatewayEvent, serde_json::Error>> {
        let raw = self.next_event().await?;
        let parsed = rift_model::GatewayEvent::parse(&raw.name, &raw.data);
        #[cfg(feature = "cache")]
        if let Ok(ref model) = parsed {
            self.update_cache(model);
        }
        Some(parsed)
    }

    /// Next typed event, silently skipping any deserialization errors.
    ///
    /// Automatically updates the attached cache if one is configured.
    /// Returns `None` once every shard has stopped.
    pub async fn next_typed_event(&mut self) -> Option<rift_model::GatewayEvent> {
        while let Some(res) = self.next_gateway_event().await {
            if let Ok(model) = res {
                return Some(model);
            }
        }
        None
    }

    /// Returns a reference to the attached in-memory cache, if configured.
    #[cfg(feature = "cache")]
    pub fn cache(&self) -> Option<&Arc<std::sync::RwLock<rift_cache::Cache>>> {
        self.cache.as_ref()
    }

    /// Updates the attached cache with a typed gateway event.
    ///
    /// No-op if no cache was attached to the client.
    #[cfg(feature = "cache")]
    pub fn update_cache(&self, event: &rift_model::GatewayEvent) {
        if let Some(ref cache) = self.cache {
            if let Ok(mut guard) = cache.write() {
                guard.update(event);
            }
        }
    }

    /// The shared, rate-limited REST client.
    pub fn rest(&self) -> &Arc<RestClient<ReqwestTransport>> {
        &self.rest
    }

    /// Number of shards in this cluster.
    pub fn num_shards(&self) -> u32 {
        self.num_shards
    }

    /// Which shard receives events for `guild_id`.
    pub fn shard_for_guild(&self, guild_id: Snowflake) -> u32 {
        let n = NonZeroU32::new(self.num_shards).unwrap_or(NonZeroU32::MIN);
        rift_core::shard::shard_for_guild(guild_id, n)
    }

    /// Send a raw gateway payload (JSON) on one shard. Rate limited per connection.
    pub async fn send_raw(&self, shard_id: u32, payload: String) -> Result<(), Error> {
        let tx = self.senders.get(shard_id as usize).ok_or(Error::UnknownShard(shard_id))?;
        tx.send(Command::Raw(payload)).await.map_err(|_| Error::ShardGone(shard_id))
    }

    /// Update presence on the specified shard.
    pub async fn update_presence(&self, shard_id: u32, presence: &UpdatePresence) -> Result<(), Error> {
        let payload = presence.to_payload().map_err(rift_runtime::CommandError::from)?;
        self.send_raw(shard_id, payload).await
    }

    /// Update voice state on the specified shard.
    pub async fn update_voice_state(&self, shard_id: u32, voice: &UpdateVoiceState) -> Result<(), Error> {
        let payload = voice.to_payload().map_err(rift_runtime::CommandError::from)?;
        self.send_raw(shard_id, payload).await
    }

    /// Request guild members on the specified shard.
    pub async fn request_guild_members(&self, shard_id: u32, req: &RequestGuildMembers) -> Result<(), Error> {
        let payload = req.to_payload()?;
        self.send_raw(shard_id, payload).await
    }

    /// Request soundboard sounds on the specified shard.
    pub async fn request_soundboard_sounds(&self, shard_id: u32, req: &RequestSoundboardSounds) -> Result<(), Error> {
        let payload = req.to_payload().map_err(rift_runtime::CommandError::from)?;
        self.send_raw(shard_id, payload).await
    }

    /// Close every shard cleanly (the bot goes offline) and wait for them.
    /// Returns the first error any shard hit, if one did.
    pub async fn shutdown(self) -> Result<(), Error> {
        let _ = self.stop.send(());
        match self.supervisor.await {
            Ok(r) => r.map_err(Error::from),
            Err(e) => Err(Error::Shard(rift_runtime::Error::Transport(format!(
                "supervisor failed: {e}"
            )))),
        }
    }
}

/// Fail early, with a clear message, if starting would eat the identify reserve.
fn check_identify_capacity(num_shards: u32, remaining: u32, reserve: u32) -> Result<(), Error> {
    let usable = remaining.saturating_sub(reserve);
    if usable < num_shards {
        Err(Error::NotEnoughIdentifies { needed: num_shards, usable, remaining })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plenty_of_identifies_is_fine() {
        assert!(check_identify_capacity(1, 998, 10).is_ok());
        assert!(check_identify_capacity(16, 26, 10).is_ok());
    }

    #[test]
    fn refuses_to_start_if_it_would_eat_the_reserve() {
        let e = check_identify_capacity(16, 20, 10).unwrap_err();
        assert!(matches!(e, Error::NotEnoughIdentifies { needed: 16, usable: 10, remaining: 20 }));
        assert!(check_identify_capacity(1, 10, 10).is_err());
    }

    #[test]
    fn error_message_explains_the_token_reset_risk() {
        let e = check_identify_capacity(5, 3, 10).unwrap_err();
        assert!(e.to_string().contains("token reset"));
    }

    #[test]
    fn only_events_builds_a_filter() {
        let b = Client::builder("tok", Intents::GUILDS).only_events(["MESSAGE_CREATE", "INTERACTION_CREATE"]);
        let f = b.event_filter.expect("filter set");
        assert!(f.contains("MESSAGE_CREATE") && f.contains("INTERACTION_CREATE") && f.len() == 2);
        assert!(Client::builder("tok", Intents::GUILDS).event_filter.is_none());
    }

    #[test]
    fn builder_defaults_are_conservative() {
        let b = Client::builder("tok", Intents::GUILDS);
        assert_eq!(b.identify_reserve, 10);
        assert_eq!(b.shard_count, None);
        assert!(b.capabilities.is_empty());
        assert!(b.wave_gap > Duration::from_secs(5));
        // Shard count can never be zero.
        assert_eq!(Client::builder("tok", Intents::GUILDS).shard_count(0).shard_count, Some(1));
    }

    #[test]
    fn builder_event_buffer_policy_config() {
        let b = Client::builder("tok", Intents::GUILDS)
            .event_buffer_policy(EventBufferPolicy::Bounded { capacity: 100, drop_oldest: true });
        assert_eq!(
            b.event_buffer_policy,
            EventBufferPolicy::Bounded { capacity: 100, drop_oldest: true }
        );
    }

    #[tokio::test]
    async fn client_commands_send_payloads_to_shard() {
        let (cmd_tx, mut cmd_rx) = mpsc::channel(16);
        let (_events_tx, events_rx) = rift_runtime::event_channel(EventBufferPolicy::Unbounded);
        let (stop_tx, _stop_rx) = mpsc::unbounded_channel();
        let supervisor = tokio::spawn(async { Ok(()) });

        let client = Client {
            rest: Arc::new(RestClient::new("token", "https://example.com", "1.0").unwrap()),
            senders: vec![cmd_tx],
            events: events_rx,
            stop: stop_tx,
            supervisor,
            num_shards: 1,
            #[cfg(feature = "cache")]
            cache: None,
        };

        // 1. update_presence
        let presence = UpdatePresence::dnd().with_activity(rift_runtime::Activity::playing("Rift"));
        client.update_presence(0, &presence).await.unwrap();
        match cmd_rx.recv().await.unwrap() {
            Command::Raw(json) => {
                let v: serde_json::Value = serde_json::from_str(&json).unwrap();
                assert_eq!(v["op"], 3);
                assert_eq!(v["d"]["status"], "dnd");
            }
            _ => panic!("expected raw command"),
        }

        // 2. update_voice_state
        let voice = UpdateVoiceState::join(Snowflake::new(100), Snowflake::new(200), false, true);
        client.update_voice_state(0, &voice).await.unwrap();
        match cmd_rx.recv().await.unwrap() {
            Command::Raw(json) => {
                let v: serde_json::Value = serde_json::from_str(&json).unwrap();
                assert_eq!(v["op"], 4);
                assert_eq!(v["d"]["guild_id"], "100");
                assert_eq!(v["d"]["channel_id"], "200");
            }
            _ => panic!("expected raw command"),
        }

        // 3. request_guild_members
        let members = RequestGuildMembers::by_query(Snowflake::new(100), "bob", 5);
        client.request_guild_members(0, &members).await.unwrap();
        match cmd_rx.recv().await.unwrap() {
            Command::Raw(json) => {
                let v: serde_json::Value = serde_json::from_str(&json).unwrap();
                assert_eq!(v["op"], 8);
                assert_eq!(v["d"]["query"], "bob");
            }
            _ => panic!("expected raw command"),
        }

        // 4. request_soundboard_sounds
        let soundboard = RequestSoundboardSounds::new(vec![Snowflake::new(300)]);
        client.request_soundboard_sounds(0, &soundboard).await.unwrap();
        match cmd_rx.recv().await.unwrap() {
            Command::Raw(json) => {
                let v: serde_json::Value = serde_json::from_str(&json).unwrap();
                assert_eq!(v["op"], 31);
                assert_eq!(v["d"]["guild_ids"], serde_json::json!(["300"]));
            }
            _ => panic!("expected raw command"),
        }

        // 5. Unknown shard error
        let err = client.update_presence(99, &presence).await.unwrap_err();
        assert!(matches!(err, Error::UnknownShard(99)));
    }

    #[test]
    #[cfg(feature = "cache")]
    fn builder_cache_configuration() {
        let b = Client::builder("tok", Intents::GUILDS).with_cache(rift_cache::CacheConfig::default());
        assert!(b.cache.is_some());
    }

    #[tokio::test]
    #[cfg(feature = "cache")]
    async fn client_next_gateway_event_updates_cache() {
        let (cmd_tx, _cmd_rx) = mpsc::channel(16);
        let (events_tx, events_rx) = rift_runtime::event_channel(EventBufferPolicy::Unbounded);
        let (stop_tx, _stop_rx) = mpsc::unbounded_channel();
        let supervisor = tokio::spawn(async { Ok(()) });

        let cache = Arc::new(std::sync::RwLock::new(rift_cache::Cache::default()));
        let mut client = Client {
            rest: Arc::new(RestClient::new("token", "https://example.com", "1.0").unwrap()),
            senders: vec![cmd_tx],
            events: events_rx,
            stop: stop_tx,
            supervisor,
            num_shards: 1,
            cache: Some(cache.clone()),
        };

        // Send a READY event into the events channel
        let ready_json = serde_json::json!({
            "v": 10,
            "user": {
                "id": "12345",
                "username": "RiftBot",
                "discriminator": "0001",
                "avatar": null,
                "bot": true
            },
            "guilds": [],
            "session_id": "sess_abc",
            "resume_gateway_url": "wss://gateway.discord.gg",
            "application": {
                "id": "2000"
            }
        });
        let raw = serde_json::value::RawValue::from_string(ready_json.to_string()).unwrap();
        let event = Event {
            shard_id: 0,
            name: "READY".to_string(),
            seq: 1,
            data: raw,
        };
        events_tx.send(event).unwrap();

        let model = client.next_gateway_event().await.unwrap().unwrap();
        assert!(matches!(model, rift_model::GatewayEvent::Ready(_)));

        // Verify cache was automatically updated
        let guard = cache.read().unwrap();
        let user = guard.current_user().expect("current user should be cached");
        assert_eq!(user.id, Snowflake::new(12345));
        assert_eq!(user.username, "RiftBot");
    }
}
