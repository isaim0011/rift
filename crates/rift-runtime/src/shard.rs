//! The shard driver: glue between the socket and the sans-IO state machine.

use crate::config::ShardConfig;
use crate::error::Error;
use crate::transport::{self, Frame, Rx, Tx};
use crate::zstd_stream::StreamDecoder;
use rift_core::Opcode;
use rift_gateway::{
    BudgetError, Disconnect, HeartbeatAction, IdentifyBudget, ReconnectPlan, SendError,
    SendLimiter, Session,
};
use serde::Deserialize;
use serde_json::json;
use serde_json::value::RawValue;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

const MAX_PENDING_COMMANDS: usize = 64;

/// One dispatch event. `data` is raw JSON: parse only what you need.
#[derive(Debug)]
pub struct Event {
    /// Which shard delivered this event.
    pub shard_id: u32,
    /// Event name, e.g. `MESSAGE_CREATE`.
    pub name: String,
    /// Sequence number.
    pub seq: u64,
    /// Raw `d` payload, not parsed.
    pub data: Box<RawValue>,
}

/// Commands you can send to a running shard.
#[derive(Debug)]
pub enum Command {
    /// Send a raw gateway payload (JSON). Rate limited and size checked for you.
    Raw(String),
    /// Close cleanly (code 1000): the bot goes offline and the shard returns `Ok`.
    Shutdown,
}

/// An error returned when attempting to send an event into a closed channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventSendError<T>(pub T);

impl<T> std::fmt::Display for EventSendError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "event receiver has dropped")
    }
}

impl<T: std::fmt::Debug> std::error::Error for EventSendError<T> {}

struct State {
    queue: VecDeque<Event>,
    policy: crate::config::EventBufferPolicy,
    closed: bool,
}

struct ChannelInner {
    state: Mutex<State>,
    notify: tokio::sync::Notify,
}

#[derive(Clone)]
enum SenderInner {
    Queue(Arc<ChannelInner>),
    Unbounded(mpsc::UnboundedSender<Event>),
}

/// Sending side of the event channel with backpressure support.
#[derive(Clone)]
pub struct EventSender {
    inner: SenderInner,
}

impl EventSender {
    /// Send an event according to the channel's configured [`EventBufferPolicy`].
    pub fn send(&self, event: Event) -> Result<(), EventSendError<Event>> {
        match &self.inner {
            SenderInner::Queue(shared) => {
                let mut guard = shared.state.lock().unwrap_or_else(|e| e.into_inner());
                if guard.closed {
                    return Err(EventSendError(event));
                }
                match guard.policy {
                    crate::config::EventBufferPolicy::Unbounded => {
                        guard.queue.push_back(event);
                        shared.notify.notify_one();
                    }
                    crate::config::EventBufferPolicy::Bounded { capacity, drop_oldest } => {
                        if guard.queue.len() < capacity {
                            guard.queue.push_back(event);
                            shared.notify.notify_one();
                        } else if drop_oldest {
                            guard.queue.pop_front();
                            guard.queue.push_back(event);
                            shared.notify.notify_one();
                        } else {
                            // Drop incoming event to protect memory bounds without stalling.
                        }
                    }
                }
                Ok(())
            }
            SenderInner::Unbounded(tx) => tx.send(event).map_err(|e| EventSendError(e.0)),
        }
    }
}

impl From<mpsc::UnboundedSender<Event>> for EventSender {
    fn from(tx: mpsc::UnboundedSender<Event>) -> Self {
        Self { inner: SenderInner::Unbounded(tx) }
    }
}

impl Drop for EventSender {
    fn drop(&mut self) {
        if let SenderInner::Queue(ref shared) = self.inner {
            if Arc::strong_count(shared) <= 2 {
                let guard = shared.state.lock().unwrap_or_else(|e| e.into_inner());
                if guard.closed || guard.queue.is_empty() {
                    shared.notify.notify_waiters();
                }
            }
        }
    }
}

/// Receiving side of the event channel with backpressure support.
pub struct EventReceiver {
    inner: Arc<ChannelInner>,
}

impl EventReceiver {
    /// Asynchronously receive the next event, waiting if the buffer is empty.
    ///
    /// Returns `None` once all senders have disconnected and the queue is drained.
    pub async fn recv(&mut self) -> Option<Event> {
        loop {
            let notified = self.inner.notify.notified();
            {
                let mut guard = self.inner.state.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(ev) = guard.queue.pop_front() {
                    return Some(ev);
                }
                if guard.closed || Arc::strong_count(&self.inner) <= 1 {
                    guard.closed = true;
                    return None;
                }
            }
            notified.await;
        }
    }
}

impl Drop for EventReceiver {
    fn drop(&mut self) {
        let mut guard = self.inner.state.lock().unwrap_or_else(|e| e.into_inner());
        guard.closed = true;
    }
}

/// Creates a pair of [`EventSender`] and [`EventReceiver`] governed by `policy`.
pub fn event_channel(policy: crate::config::EventBufferPolicy) -> (EventSender, EventReceiver) {
    let shared = Arc::new(ChannelInner {
        state: Mutex::new(State {
            queue: VecDeque::new(),
            policy,
            closed: false,
        }),
        notify: tokio::sync::Notify::new(),
    });
    (
        EventSender { inner: SenderInner::Queue(shared.clone()) },
        EventReceiver { inner: shared },
    )
}

/// Identify budget shared by all shards of one bot.
pub type SharedBudget = Arc<Mutex<IdentifyBudget>>;

/// Milliseconds on a process-wide monotonic clock. Seed budgets with this.
pub fn now_ms() -> u64 {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    EPOCH.get_or_init(Instant::now).elapsed().as_millis() as u64
}

#[derive(Deserialize)]
struct Envelope<'a> {
    op: u8,
    #[serde(borrow)]
    d: Option<&'a RawValue>,
    s: Option<u64>,
    #[serde(borrow)]
    t: Option<&'a str>,
}

#[derive(Deserialize)]
struct Hello {
    heartbeat_interval: u64,
}

#[derive(Deserialize)]
struct Ready {
    session_id: String,
    resume_gateway_url: String,
}

enum Outcome {
    Disconnected(Disconnect),
    Shutdown,
}

struct Ctx<'a> {
    config: &'a ShardConfig,
    budget: &'a SharedBudget,
    events: &'a EventSender,
}

fn ws_url(base: &str) -> String {
    format!("{}/?v=10&encoding=json&compress=zstd-stream", base.trim_end_matches('/'))
}

fn backoff(failures: u32) -> Duration {
    Duration::from_millis((250u64 << failures.min(7)).min(30_000))
}

fn jitter() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| f64::from(d.subsec_nanos() % 1000) / 1000.0)
        .unwrap_or(0.5)
}

fn acquire(budget: &SharedBudget, now: u64) -> Result<(), BudgetError> {
    budget.lock().unwrap_or_else(|e| e.into_inner()).try_acquire(now)
}

fn identify_payload(c: &ShardConfig) -> String {
    let mut d = json!({
        "token": c.token,
        "intents": c.intents.bits(),
        "properties": { "os": std::env::consts::OS, "browser": "rift", "device": "rift" },
        "shard": [c.shard_id, c.num_shards],
    });
    // Docs: `capabilities` is optional and defaults to 0, so only send it when set.
    if !c.capabilities.is_empty() {
        d["capabilities"] = json!(c.capabilities.bits());
    }
    json!({ "op": 2, "d": d }).to_string()
}

fn lost() -> Result<Outcome, Error> {
    Ok(Outcome::Disconnected(Disconnect::Close(None)))
}

/// Send a heartbeat/identify/resume. Returns `false` if the socket is dead.
async fn send_priority(tx: &mut Tx, limiter: &mut SendLimiter, now: u64, text: String) -> bool {
    if let Err(e) = limiter.try_send(now, text.len(), true) {
        tracing::warn!(error = %e, "priority send refused by send limiter");
        return true;
    }
    tx.send_text(text).await.is_ok()
}

/// Run one shard until a fatal error or shutdown. Reconnects, resumes and
/// re-identifies automatically per the Gateway docs.
///
/// Events go to `events` through an [`EventSender`] (or unbounded channel)
/// governed by [`EventBufferPolicy`](crate::EventBufferPolicy).
pub async fn run_shard(
    config: ShardConfig,
    budget: SharedBudget,
    events: impl Into<EventSender>,
    mut commands: mpsc::Receiver<Command>,
) -> Result<(), Error> {
    let events = events.into();
    let mut session = Session::default();
    let mut plan = ReconnectPlan::Identify;
    let mut failures: u32 = 0;

    loop {
        let (base_url, resume) = match &plan {
            ReconnectPlan::Fatal(code) => return Err(Error::Fatal(*code)),
            ReconnectPlan::Identify => (config.gateway_url.clone(), None),
            ReconnectPlan::Resume { url, session_id, seq } => {
                (url.clone(), Some((session_id.clone(), *seq)))
            }
        };

        let (mut tx, mut rx) = match transport::connect(&ws_url(&base_url)).await {
            Ok(pair) => pair,
            Err(e) => {
                let wait = backoff(failures + 1);
                tracing::warn!(
                    shard_id = config.shard_id,
                    error = %e,
                    backoff_ms = wait.as_millis() as u64,
                    failures = failures + 1,
                    "gateway shard connection failed; reconnecting"
                );
                failures += 1;
                if resume.is_some() && failures >= 3 {
                    plan = session.plan(Disconnect::InvalidSession { resumable: false });
                }
                tokio::time::sleep(wait).await;
                continue;
            }
        };

        let mut ready_seen = false;
        let ctx = Ctx { config: &config, budget: &budget, events: &events };
        let outcome =
            run_connection(&mut tx, &mut rx, &ctx, &mut session, resume, &mut ready_seen, &mut commands)
                .await?;

        match outcome {
            Outcome::Shutdown => {
                tracing::info!(shard_id = config.shard_id, "gateway shard shutting down cleanly");
                return Ok(());
            }
            Outcome::Disconnected(why) => {
                failures = if ready_seen { 0 } else { failures + 1 };
                plan = session.plan(why);
                if matches!(plan, ReconnectPlan::Fatal(_)) {
                    tracing::error!(
                        shard_id = config.shard_id,
                        reason = ?why,
                        "gateway shard encountered fatal disconnect; terminating"
                    );
                    continue;
                }
                if failures > 0 {
                    let wait = backoff(failures);
                    tracing::warn!(
                        shard_id = config.shard_id,
                        reason = ?why,
                        failures,
                        backoff_ms = wait.as_millis() as u64,
                        "gateway shard disconnected; reconnecting with backoff"
                    );
                    tokio::time::sleep(wait).await;
                } else {
                    tracing::info!(
                        shard_id = config.shard_id,
                        reason = ?why,
                        "gateway shard disconnected; reconnecting immediately"
                    );
                }
            }
        }
    }
}

async fn run_connection(
    tx: &mut Tx,
    rx: &mut Rx,
    ctx: &Ctx<'_>,
    session: &mut Session,
    resume: Option<(String, u64)>,
    ready_seen: &mut bool,
    commands: &mut mpsc::Receiver<Command>,
) -> Result<Outcome, Error> {
    let mut decoder = StreamDecoder::new().map_err(|e| Error::Transport(e.to_string()))?;
    let mut limiter = SendLimiter::default();
    let mut pending: VecDeque<String> = VecDeque::new();

    loop {
        let now = now_ms();

        match session.heartbeat.poll(now) {
            HeartbeatAction::Wait => {}
            HeartbeatAction::Zombie => return Ok(Outcome::Disconnected(Disconnect::Zombie)),
            HeartbeatAction::Send { seq } => {
                let text = json!({ "op": 1, "d": seq }).to_string();
                if !send_priority(tx, &mut limiter, now, text).await {
                    return lost();
                }
            }
        }

        // Flush queued user commands through the limiter.
        let mut retry_ms: Option<u64> = None;
        while let Some(len) = pending.front().map(|t| t.len()) {
            match limiter.try_send(now, len, false) {
                Ok(()) => {
                    let text = pending.pop_front().unwrap_or_default();
                    if tx.send_text(text).await.is_err() {
                        return lost();
                    }
                }
                Err(SendError::TooLarge { len }) => {
                    tracing::warn!(
                        shard_id = ctx.config.shard_id,
                        len,
                        "dropped oversized gateway payload"
                    );
                    pending.pop_front();
                }
                Err(SendError::RateLimited { retry_after_ms }) => {
                    retry_ms = Some(retry_after_ms);
                    break;
                }
            }
        }

        let hb_wait = session.heartbeat.next_due_ms().map(|d| d.saturating_sub(now));
        let wait = [hb_wait, retry_ms].into_iter().flatten().min().unwrap_or(1_000);

        tokio::select! {
            frame = rx.recv() => {
                let frame = match frame {
                    Some(Ok(f)) => f,
                    _ => return lost(),
                };
                let now = now_ms();
                let bytes: &[u8] = match &frame {
                    Frame::Binary(b) => match decoder.decode(b) {
                        Ok(out) => out,
                        Err(e) => {
                            tracing::error!(
                                shard_id = ctx.config.shard_id,
                                error = %e,
                                "gateway frame zstd decompression error"
                            );
                            return lost();
                        }
                    },
                    Frame::Text(t) => t.as_bytes(),
                    Frame::Close(code) => {
                        return Ok(Outcome::Disconnected(Disconnect::Close(*code)));
                    }
                    Frame::Ignore => continue,
                };

                let env: Envelope = match serde_json::from_slice(bytes) {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::warn!(
                            shard_id = ctx.config.shard_id,
                            error = %e,
                            "undecodable gateway frame"
                        );
                        continue;
                    }
                };

                match Opcode::from_u8(env.op) {
                    Some(Opcode::Hello) => {
                        let hello = match env.d.map(|d| serde_json::from_str::<Hello>(d.get())) {
                            Some(Ok(h)) => h,
                            _ => return lost(),
                        };
                        session.heartbeat.on_hello(now, hello.heartbeat_interval, jitter());
                        tracing::debug!(
                            shard_id = ctx.config.shard_id,
                            heartbeat_interval_ms = hello.heartbeat_interval,
                            resuming = resume.is_some(),
                            "received HELLO from gateway"
                        );
                        let handshake = match &resume {
                            Some((id, seq)) => json!({
                                "op": 6,
                                "d": { "token": ctx.config.token, "session_id": id, "seq": seq }
                            })
                            .to_string(),
                            None => {
                                acquire(ctx.budget, now)?;
                                identify_payload(ctx.config)
                            }
                        };
                        if !send_priority(tx, &mut limiter, now, handshake).await {
                            return lost();
                        }
                    }
                    Some(Opcode::Heartbeat) => {
                        if let HeartbeatAction::Send { seq } = session.heartbeat.on_server_request(now) {
                            let text = json!({ "op": 1, "d": seq }).to_string();
                            if !send_priority(tx, &mut limiter, now, text).await {
                                return lost();
                            }
                        }
                    }
                    Some(Opcode::HeartbeatAck) => {
                        session.heartbeat.on_ack();
                        tracing::debug!(
                            shard_id = ctx.config.shard_id,
                            "heartbeat acknowledged by gateway"
                        );
                    }
                    Some(Opcode::Reconnect) => {
                        return Ok(Outcome::Disconnected(Disconnect::ReconnectRequested));
                    }
                    Some(Opcode::InvalidSession) => {
                        let resumable = env
                            .d
                            .and_then(|d| serde_json::from_str::<bool>(d.get()).ok())
                            .unwrap_or(false);
                        return Ok(Outcome::Disconnected(Disconnect::InvalidSession { resumable }));
                    }
                    Some(Opcode::Dispatch) => {
                        if let Some(s) = env.s {
                            session.heartbeat.record_seq(s);
                        }
                        let name = env.t.unwrap_or("");
                        if name == "READY" {
                            if let Some(Ok(r)) =
                                env.d.map(|d| serde_json::from_str::<Ready>(d.get()))
                            {
                                session.on_ready(r.session_id.clone(), r.resume_gateway_url.clone());
                                *ready_seen = true;
                                tracing::info!(
                                    shard_id = ctx.config.shard_id,
                                    session_id = %r.session_id,
                                    resume_gateway_url = %r.resume_gateway_url,
                                    "shard received READY from gateway"
                                );
                            }
                        } else if name == "RESUMED" {
                            *ready_seen = true;
                            tracing::info!(
                                shard_id = ctx.config.shard_id,
                                "shard session successfully RESUMED"
                            );
                        }
                        let keep = match &ctx.config.event_filter {
                            None => true,
                            Some(f) => name == "READY" || name == "RESUMED" || f.contains(name),
                        };
                        if keep {
                            if let (Some(d), Some(seq)) = (env.d, env.s) {
                                let ev = Event {
                                    shard_id: ctx.config.shard_id,
                                    name: name.to_owned(),
                                    seq,
                                    data: d.to_owned(),
                                };
                                if ctx.events.send(ev).is_err() {
                                    // Consumer is gone: shut down cleanly.
                                    tx.close().await;
                                    return Ok(Outcome::Shutdown);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            cmd = commands.recv(), if pending.len() < MAX_PENDING_COMMANDS => {
                match cmd {
                    Some(Command::Raw(text)) => pending.push_back(text),
                    Some(Command::Shutdown) | None => {
                        tx.close().await;
                        return Ok(Outcome::Shutdown);
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(wait)) => {}
        }
    }
}

/// Parse just the envelope of a raw gateway frame: `(op, s)`.
///
/// Exposed for benchmarks only; not a stable API.
#[doc(hidden)]
pub fn envelope_probe(bytes: &[u8]) -> Option<(u8, Option<u64>)> {
    let env: Envelope = serde_json::from_slice(bytes).ok()?;
    Some((env.op, env.s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EventBufferPolicy;

    #[test]
    fn url_has_version_encoding_and_zstd() {
        assert_eq!(
            ws_url("wss://gateway.discord.gg/"),
            "wss://gateway.discord.gg/?v=10&encoding=json&compress=zstd-stream"
        );
        assert_eq!(
            ws_url("wss://resume.example"),
            "wss://resume.example/?v=10&encoding=json&compress=zstd-stream"
        );
    }

    #[test]
    fn envelope_probe_reads_op_and_sequence_without_parsing_the_payload() {
        assert_eq!(envelope_probe(br#"{"op":0,"s":7,"t":"X","d":{"a":[1,2,3]}}"#), Some((0, Some(7))));
        assert_eq!(envelope_probe(br#"{"op":11,"d":null}"#), Some((11, None)));
        assert_eq!(envelope_probe(b"not json"), None);
    }

    #[test]
    fn backoff_grows_and_caps() {
        assert_eq!(backoff(1), Duration::from_millis(500));
        assert_eq!(backoff(20), Duration::from_millis(30_000));
    }

    #[test]
    fn identify_payload_shape() {
        let c = ShardConfig::new("tok", rift_core::Intents::GUILDS | rift_core::Intents::GUILD_MESSAGES);
        let v: serde_json::Value = serde_json::from_str(&identify_payload(&c)).unwrap();
        assert_eq!(v["op"], 2);
        assert_eq!(v["d"]["intents"], 513);
        assert_eq!(v["d"]["shard"], json!([0, 1]));
        assert_eq!(v["d"]["token"], "tok");
    }

    #[test]
    fn capabilities_are_omitted_by_default_and_sent_when_enabled() {
        let c = ShardConfig::new("tok", rift_core::Intents::GUILDS);
        let v: serde_json::Value = serde_json::from_str(&identify_payload(&c)).unwrap();
        assert!(v["d"].get("capabilities").is_none());

        let c = c.with_capabilities(rift_core::Capabilities::CHANNEL_OBFUSCATION);
        let v: serde_json::Value = serde_json::from_str(&identify_payload(&c)).unwrap();
        assert_eq!(v["d"]["capabilities"], 32_768);
        // Intents are untouched by the capability bit.
        assert_eq!(v["d"]["intents"], 1);
    }

    #[test]
    fn envelope_borrows_without_parsing_d() {
        let raw = br#"{"op":0,"d":{"id":"1","content":"hi"},"s":7,"t":"MESSAGE_CREATE"}"#;
        let env: Envelope = serde_json::from_slice(raw).unwrap();
        assert_eq!((env.op, env.s, env.t), (0, Some(7), Some("MESSAGE_CREATE")));
        assert_eq!(env.d.unwrap().get(), r#"{"id":"1","content":"hi"}"#);
    }

    #[tokio::test]
    async fn event_channel_unbounded_delivers_all_events() {
        let (tx, mut rx) = event_channel(EventBufferPolicy::Unbounded);
        for i in 0..10 {
            tx.send(Event {
                shard_id: 0,
                name: "TEST".to_owned(),
                seq: i,
                data: RawValue::from_string(format!(r#"{{"i":{i}}}"#)).unwrap(),
            })
            .unwrap();
        }

        for i in 0..10 {
            let ev = rx.recv().await.expect("received event");
            assert_eq!(ev.seq, i);
        }
    }

    #[tokio::test]
    async fn event_channel_bounded_drop_oldest() {
        let (tx, mut rx) = event_channel(EventBufferPolicy::Bounded { capacity: 3, drop_oldest: true });
        for i in 0..5 {
            tx.send(Event {
                shard_id: 0,
                name: "TEST".to_owned(),
                seq: i,
                data: RawValue::from_string(format!(r#"{{"i":{i}}}"#)).unwrap(),
            })
            .unwrap();
        }

        // Only the newest 3 (seq 2, 3, 4) should remain
        let e1 = rx.recv().await.unwrap();
        assert_eq!(e1.seq, 2);
        let e2 = rx.recv().await.unwrap();
        assert_eq!(e2.seq, 3);
        let e3 = rx.recv().await.unwrap();
        assert_eq!(e3.seq, 4);
    }

    #[tokio::test]
    async fn event_channel_bounded_drop_newest() {
        let (tx, mut rx) = event_channel(EventBufferPolicy::Bounded { capacity: 3, drop_oldest: false });
        for i in 0..5 {
            tx.send(Event {
                shard_id: 0,
                name: "TEST".to_owned(),
                seq: i,
                data: RawValue::from_string(format!(r#"{{"i":{i}}}"#)).unwrap(),
            })
            .unwrap();
        }

        // Only the first 3 (seq 0, 1, 2) should remain
        let e1 = rx.recv().await.unwrap();
        assert_eq!(e1.seq, 0);
        let e2 = rx.recv().await.unwrap();
        assert_eq!(e2.seq, 1);
        let e3 = rx.recv().await.unwrap();
        assert_eq!(e3.seq, 2);
    }

    #[tokio::test]
    async fn event_channel_sender_closed_when_receiver_dropped() {
        let (tx, rx) = event_channel(EventBufferPolicy::Unbounded);
        drop(rx);
        let res = tx.send(Event {
            shard_id: 0,
            name: "TEST".to_owned(),
            seq: 1,
            data: RawValue::from_string(r#"{"i":1}"#.to_owned()).unwrap(),
        });
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn event_channel_receiver_drains_then_returns_none_after_senders_drop() {
        let (tx, mut rx) = event_channel(EventBufferPolicy::Unbounded);
        tx.send(Event {
            shard_id: 0,
            name: "TEST".to_owned(),
            seq: 1,
            data: RawValue::from_string(r#"{"i":1}"#.to_owned()).unwrap(),
        })
        .unwrap();
        drop(tx);

        let ev = rx.recv().await.unwrap();
        assert_eq!(ev.seq, 1);
        assert!(rx.recv().await.is_none());
    }
}
