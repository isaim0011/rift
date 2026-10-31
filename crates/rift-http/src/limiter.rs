use crate::headers::{RateLimitHeaders, Scope};
use crate::invalid::InvalidTracker;
use crate::route::Route;
use std::collections::{HashMap, HashSet, VecDeque};

/// Tunables. Defaults follow the docs.
#[derive(Clone, Debug)]
pub struct Config {
    /// Global requests per second (docs: 50).
    pub global_per_second: u32,
    /// Hard invalid-request limit per 10 minutes (docs: 10,000).
    pub invalid_limit: u32,
    /// Stop sending when the invalid count reaches this (safety margin below the limit).
    pub invalid_trip_at: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self { global_per_second: 50, invalid_limit: 10_000, invalid_trip_at: 9_000 }
    }
}

/// Why a request must not be sent at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockReason {
    /// Too many 401/403/429s recently; sending more risks a Cloudflare ban.
    CircuitOpen {
        /// Milliseconds until the count drops below the trip point.
        retry_after_ms: u64,
    },
    /// This webhook returned 404; Discord restricts clients that keep using it.
    DeadWebhook,
}

/// Decision for one request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Send now.
    Go,
    /// Wait this long, then call `acquire` again.
    Wait {
        /// Milliseconds to wait.
        retry_after_ms: u64,
    },
    /// Do not send.
    Blocked(BlockReason),
}

#[derive(Clone, Debug)]
struct Bucket {
    limit: u32,
    remaining: u32,
    /// For a probe placeholder: when it is considered abandoned.
    reset_at_ms: u64,
    /// First request on an unknown route is sent alone so we learn its limits.
    probing: bool,
}

const PROBE_WAIT_MS: u64 = 50;
/// A probe with no response after this long (e.g. the request future was
/// cancelled) is abandoned so the route can't wedge.
const PROBE_TIMEOUT_MS: u64 = 30_000;
/// Run the idle-bucket sweep every this many `acquire` calls...
const PRUNE_EVERY: u32 = 1_024;
/// ...but only when at least this many buckets exist.
const PRUNE_MIN_BUCKETS: usize = 1_024;
/// A bucket idle this long past its reset is dropped.
const PRUNE_IDLE_MS: u64 = 300_000;
/// Dead webhooks remembered (oldest forgotten first), so per-token entries
/// from interactions can't grow without bound.
const MAX_DEAD_WEBHOOKS: usize = 10_000;

/// The REST rate limiter.
#[derive(Debug)]
pub struct RateLimiter {
    cfg: Config,
    route_bucket: HashMap<String, String>,
    buckets: HashMap<String, Bucket>,
    global_window: VecDeque<u64>,
    global_blocked_until: u64,
    invalid: InvalidTracker,
    dead_webhooks: HashSet<String>,
    dead_order: VecDeque<String>,
    acquires: u32,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(Config::default())
    }
}

impl RateLimiter {
    /// New limiter.
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            route_bucket: HashMap::new(),
            buckets: HashMap::new(),
            global_window: VecDeque::new(),
            global_blocked_until: 0,
            invalid: InvalidTracker::default(),
            dead_webhooks: HashSet::new(),
            dead_order: VecDeque::new(),
            acquires: 0,
        }
    }

    fn key(&self, route: &Route) -> String {
        let base = self.route_bucket.get(&route.id).map_or(route.id.as_str(), String::as_str);
        format!("{base}|{}", route.major)
    }

    /// Invalid requests (401/403/429 non-shared) currently in the 10-minute window.
    pub fn invalid_requests(&self, now_ms: u64) -> u32 {
        self.invalid.count(now_ms)
    }

    /// Drop buckets that are idle (and abandoned probes). Called automatically.
    pub(crate) fn prune(&mut self, now_ms: u64) {
        self.buckets.retain(|_, b| {
            let grace = if b.probing { 0 } else { PRUNE_IDLE_MS };
            now_ms < b.reset_at_ms.saturating_add(grace)
        });
    }

    fn mark_dead(&mut self, webhook: &str) {
        if self.dead_webhooks.insert(webhook.to_owned()) {
            self.dead_order.push_back(webhook.to_owned());
            if self.dead_order.len() > MAX_DEAD_WEBHOOKS {
                if let Some(old) = self.dead_order.pop_front() {
                    self.dead_webhooks.remove(&old);
                }
            }
        }
    }

    /// Ask permission to send. On [`Verdict::Go`] a slot is reserved.
    pub fn acquire(&mut self, route: &Route, now_ms: u64) -> Verdict {
        self.acquires += 1;
        if self.acquires >= PRUNE_EVERY {
            self.acquires = 0;
            if self.buckets.len() >= PRUNE_MIN_BUCKETS {
                self.prune(now_ms);
            }
        }

        if route.webhook.as_ref().is_some_and(|w| self.dead_webhooks.contains(w)) {
            return Verdict::Blocked(BlockReason::DeadWebhook);
        }
        if let Some(ms) = self.invalid.retry_after_if_at_least(now_ms, self.cfg.invalid_trip_at) {
            return Verdict::Blocked(BlockReason::CircuitOpen { retry_after_ms: ms });
        }

        if !route.global_exempt {
            if now_ms < self.global_blocked_until {
                return Verdict::Wait { retry_after_ms: self.global_blocked_until - now_ms };
            }
            while let Some(&t) = self.global_window.front() {
                if now_ms.saturating_sub(t) >= 1_000 {
                    self.global_window.pop_front();
                } else {
                    break;
                }
            }
            if self.global_window.len() >= self.cfg.global_per_second as usize {
                let oldest = self.global_window.front().copied().unwrap_or(now_ms);
                return Verdict::Wait { retry_after_ms: (oldest + 1_000).saturating_sub(now_ms) };
            }
        }

        if !route.untracked {
            let key = self.key(route);
            match self.buckets.get_mut(&key) {
                None => {
                    self.buckets.insert(
                        key,
                        Bucket {
                            limit: 1,
                            remaining: 0,
                            reset_at_ms: now_ms + PROBE_TIMEOUT_MS,
                            probing: true,
                        },
                    );
                }
                Some(b) if b.probing => {
                    if now_ms >= b.reset_at_ms {
                        // The earlier probe never reported back; take over.
                        b.reset_at_ms = now_ms + PROBE_TIMEOUT_MS;
                    } else {
                        return Verdict::Wait { retry_after_ms: PROBE_WAIT_MS };
                    }
                }
                Some(b) => {
                    if now_ms >= b.reset_at_ms {
                        b.remaining = b.limit;
                    }
                    if b.remaining == 0 {
                        return Verdict::Wait {
                            retry_after_ms: b.reset_at_ms.saturating_sub(now_ms).max(1),
                        };
                    }
                    b.remaining -= 1;
                }
            }
        }

        if !route.global_exempt {
            self.global_window.push_back(now_ms);
        }
        Verdict::Go
    }

    /// Report a response. Call for every response, including errors
    /// (use status `0` when no response arrived).
    pub fn on_response(&mut self, route: &Route, status: u16, h: &RateLimitHeaders, now_ms: u64) {
        let shared = h.scope == Some(Scope::Shared);
        if status == 401 || status == 403 || (status == 429 && !shared) {
            self.invalid.record(now_ms);
        }
        if status == 404 {
            if let Some(w) = &route.webhook {
                self.mark_dead(w);
            }
        }

        let is_global_429 = status == 429 && h.is_global();
        if is_global_429 {
            let wait = h.retry_after_ms.or(h.reset_after_ms).unwrap_or(1_000);
            self.global_blocked_until = self.global_blocked_until.max(now_ms + wait);
        }

        if route.untracked {
            return;
        }

        let old_key = self.key(route);
        if let Some(id) = &h.bucket {
            self.route_bucket.insert(route.id.clone(), id.clone());
        }
        let new_key = self.key(route);

        let mut b = self.buckets.remove(&old_key).unwrap_or(Bucket {
            limit: 1,
            remaining: 0,
            reset_at_ms: 0,
            probing: true,
        });

        let has_info = h.has_bucket_info() || (status == 429 && !is_global_429);
        if !has_info {
            // Nothing learned. Keep real state, drop the probe placeholder.
            if !b.probing {
                self.buckets.insert(old_key, b);
            }
            return;
        }

        let was_probing = b.probing;
        if let Some(l) = h.limit {
            b.limit = l;
        }
        if let Some(r) = h.remaining {
            b.remaining = r;
        }
        if let Some(ra) = h.reset_after_ms {
            b.reset_at_ms = now_ms + ra;
        } else if was_probing {
            b.reset_at_ms = now_ms;
        }
        if status == 429 && !is_global_429 {
            let wait = h.retry_after_ms.or(h.reset_after_ms).unwrap_or(1_000);
            b.remaining = 0;
            b.reset_at_ms = b.reset_at_ms.max(now_ms + wait);
        }
        b.probing = false;
        self.buckets.insert(new_key, b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(limit: u32, remaining: u32, reset_after_ms: u64, bucket: &str) -> RateLimitHeaders {
        RateLimitHeaders {
            limit: Some(limit),
            remaining: Some(remaining),
            reset_after_ms: Some(reset_after_ms),
            bucket: Some(bucket.to_owned()),
            ..Default::default()
        }
    }

    #[test]
    fn first_request_probes_then_bucket_is_enforced() {
        let mut rl = RateLimiter::default();
        let r = Route::new("POST", "/channels/1/messages");
        assert_eq!(rl.acquire(&r, 0), Verdict::Go);
        // A second request while the first is in flight waits for its headers.
        assert_eq!(rl.acquire(&r, 0), Verdict::Wait { retry_after_ms: PROBE_WAIT_MS });

        rl.on_response(&r, 200, &hdr(5, 0, 1_500, "abcd"), 0);
        assert_eq!(rl.acquire(&r, 100), Verdict::Wait { retry_after_ms: 1_400 });
        assert_eq!(rl.acquire(&r, 1_500), Verdict::Go);
    }

    #[test]
    fn buckets_are_per_major_resource() {
        let mut rl = RateLimiter::default();
        let a = Route::new("POST", "/channels/1/messages");
        let b = Route::new("POST", "/channels/2/messages");
        assert_eq!(rl.acquire(&a, 0), Verdict::Go);
        rl.on_response(&a, 200, &hdr(5, 0, 1_000, "abcd"), 0);
        assert!(matches!(rl.acquire(&a, 10), Verdict::Wait { .. }));
        // Channel 2 is a different top-level resource: not limited by channel 1.
        assert_eq!(rl.acquire(&b, 10), Verdict::Go);
    }

    #[test]
    fn remaining_counts_down_locally() {
        let mut rl = RateLimiter::default();
        let r = Route::new("GET", "/guilds/9/members/5");
        assert_eq!(rl.acquire(&r, 0), Verdict::Go);
        rl.on_response(&r, 200, &hdr(3, 2, 10_000, "g"), 0);
        assert_eq!(rl.acquire(&r, 1), Verdict::Go);
        assert_eq!(rl.acquire(&r, 2), Verdict::Go);
        assert!(matches!(rl.acquire(&r, 3), Verdict::Wait { .. }));
    }

    #[test]
    fn global_limit_is_50_per_second_and_interactions_are_exempt() {
        let mut rl = RateLimiter::default();
        let ping = Route::new("GET", "/users/@me");
        assert_eq!(rl.acquire(&ping, 0), Verdict::Go);
        rl.on_response(&ping, 200, &hdr(1000, 999, 1_000, "u"), 0);
        for _ in 0..49 {
            assert_eq!(rl.acquire(&ping, 0), Verdict::Go);
        }
        assert_eq!(rl.acquire(&ping, 500), Verdict::Wait { retry_after_ms: 500 });
        assert_eq!(rl.acquire(&ping, 1_000), Verdict::Go);

        let cb = Route::new("POST", "/interactions/1/tok/callback");
        let mut rl2 = RateLimiter::default();
        assert_eq!(rl2.acquire(&cb, 0), Verdict::Go);
        rl2.on_response(&cb, 204, &RateLimitHeaders::default(), 0);
        for _ in 0..200 {
            assert_eq!(rl2.acquire(&cb, 0), Verdict::Go);
            rl2.on_response(&cb, 204, &RateLimitHeaders::default(), 0);
        }
    }

    #[test]
    fn global_429_blocks_every_route() {
        let mut rl = RateLimiter::default();
        let a = Route::new("GET", "/users/@me");
        let b = Route::new("GET", "/gateway/bot");
        assert_eq!(rl.acquire(&a, 0), Verdict::Go);
        let h = RateLimitHeaders {
            global: true,
            scope: Some(Scope::Global),
            retry_after_ms: Some(5_000),
            ..Default::default()
        };
        rl.on_response(&a, 429, &h, 0);
        assert_eq!(rl.acquire(&b, 1_000), Verdict::Wait { retry_after_ms: 4_000 });
        assert_eq!(rl.acquire(&b, 5_000), Verdict::Go);
    }

    #[test]
    fn route_429_pauses_only_that_bucket() {
        let mut rl = RateLimiter::default();
        let r = Route::new("POST", "/channels/1/messages");
        assert_eq!(rl.acquire(&r, 0), Verdict::Go);
        let h = RateLimitHeaders {
            scope: Some(Scope::User),
            retry_after_ms: Some(3_000),
            bucket: Some("b".into()),
            ..Default::default()
        };
        rl.on_response(&r, 429, &h, 0);
        assert_eq!(rl.acquire(&r, 1_000), Verdict::Wait { retry_after_ms: 2_000 });
    }

    #[test]
    fn circuit_breaker_trips_and_shared_429_does_not_count() {
        let mut rl = RateLimiter::new(Config { invalid_trip_at: 5, ..Config::default() });
        let r = Route::new("GET", "/channels/1/messages");
        let shared = RateLimitHeaders { scope: Some(Scope::Shared), retry_after_ms: Some(1), ..Default::default() };
        for _ in 0..10 {
            rl.on_response(&r, 429, &shared, 0);
        }
        assert_eq!(rl.invalid_requests(0), 0);

        for _ in 0..5 {
            rl.on_response(&r, 403, &RateLimitHeaders::default(), 0);
        }
        assert_eq!(rl.invalid_requests(0), 5);
        assert_eq!(
            rl.acquire(&r, 1_000),
            Verdict::Blocked(BlockReason::CircuitOpen { retry_after_ms: 609_000 })
        );
        assert_ne!(rl.acquire(&r, 610_000), Verdict::Blocked(BlockReason::CircuitOpen { retry_after_ms: 0 }));
        assert_eq!(rl.invalid_requests(610_000), 0);
    }

    #[test]
    fn dead_webhook_is_never_retried() {
        let mut rl = RateLimiter::default();
        let w = Route::new("POST", "/webhooks/10/tok");
        assert_eq!(rl.acquire(&w, 0), Verdict::Go);
        rl.on_response(&w, 404, &RateLimitHeaders::default(), 0);
        assert_eq!(rl.acquire(&w, 1), Verdict::Blocked(BlockReason::DeadWebhook));
        // A different webhook is unaffected.
        let other = Route::new("POST", "/webhooks/11/tok2");
        assert_eq!(rl.acquire(&other, 1), Verdict::Go);
    }

    #[test]
    fn route_without_headers_does_not_get_stuck_probing() {
        let mut rl = RateLimiter::default();
        let r = Route::new("GET", "/gateway");
        assert_eq!(rl.acquire(&r, 0), Verdict::Go);
        rl.on_response(&r, 200, &RateLimitHeaders::default(), 0);
        assert_eq!(rl.acquire(&r, 1), Verdict::Go);
    }

    // ----- added with the interactions work -----

    #[test]
    fn interaction_callbacks_are_never_probed_or_bucketed() {
        let mut rl = RateLimiter::default();
        let cb = Route::new("POST", "/interactions/1/tok/callback");
        // Two callbacks in flight at once: neither waits for the other.
        assert_eq!(rl.acquire(&cb, 0), Verdict::Go);
        assert_eq!(rl.acquire(&cb, 0), Verdict::Go);
        // Even a response carrying bucket headers must not create shared state.
        rl.on_response(&cb, 204, &hdr(5, 0, 60_000, "cb"), 0);
        assert_eq!(rl.acquire(&cb, 1), Verdict::Go);
        assert!(rl.buckets.is_empty());
    }

    #[test]
    fn interaction_callbacks_still_count_toward_the_invalid_limit() {
        let mut rl = RateLimiter::new(Config { invalid_trip_at: 2, ..Config::default() });
        let cb = Route::new("POST", "/interactions/1/tok/callback");
        rl.on_response(&cb, 403, &RateLimitHeaders::default(), 0);
        rl.on_response(&cb, 403, &RateLimitHeaders::default(), 0);
        assert!(matches!(rl.acquire(&cb, 1), Verdict::Blocked(BlockReason::CircuitOpen { .. })));
    }

    #[test]
    fn an_abandoned_probe_is_taken_over() {
        let mut rl = RateLimiter::default();
        let r = Route::new("GET", "/channels/1/messages");
        assert_eq!(rl.acquire(&r, 0), Verdict::Go); // probe; the caller is then cancelled
        assert_eq!(rl.acquire(&r, 1_000), Verdict::Wait { retry_after_ms: PROBE_WAIT_MS });
        assert_eq!(rl.acquire(&r, PROBE_TIMEOUT_MS), Verdict::Go);
        // The new probe is itself protected again.
        assert!(matches!(rl.acquire(&r, PROBE_TIMEOUT_MS + 1), Verdict::Wait { .. }));
    }

    #[test]
    fn the_probe_timeout_never_leaks_into_the_learned_reset_time() {
        // Regression: a probe placeholder carries a 30 s abandonment timeout in
        // `reset_at_ms`. A response with limits but no reset-after must not
        // inherit it as the bucket's reset time.
        let mut rl = RateLimiter::default();
        let r = Route::new("GET", "/channels/1/messages");
        assert_eq!(rl.acquire(&r, 0), Verdict::Go);
        let h = RateLimitHeaders {
            limit: Some(5),
            remaining: Some(0),
            bucket: Some("b".into()),
            ..Default::default()
        };
        rl.on_response(&r, 200, &h, 0);
        // With no reset time to go on, we can't justify waiting (the broken
        // behavior here was a 29,999 ms wait).
        assert_eq!(rl.acquire(&r, 1), Verdict::Go);
    }

    #[test]
    fn prune_drops_idle_buckets_and_keeps_active_ones() {
        let mut rl = RateLimiter::default();
        let idle = Route::new("GET", "/channels/1/messages");
        let active = Route::new("GET", "/channels/2/messages");
        rl.acquire(&idle, 0);
        rl.on_response(&idle, 200, &hdr(5, 4, 1_000, "a"), 0);
        rl.acquire(&active, 0);
        rl.on_response(&active, 200, &hdr(5, 4, 400_000, "a"), 0);
        assert_eq!(rl.buckets.len(), 2);

        rl.prune(310_000);
        assert_eq!(rl.buckets.len(), 1);
        assert!(rl.buckets.keys().any(|k| k.ends_with("channels/2")));
    }

    #[test]
    fn acquire_sweeps_idle_buckets_automatically() {
        let mut rl = RateLimiter::default();
        // 40 requests/second stays under the global limit.
        for i in 0..1_100u32 {
            let now = u64::from(i) * 25;
            let r = Route::new("GET", &format!("/channels/{i}/messages"));
            assert_eq!(rl.acquire(&r, now), Verdict::Go);
            rl.on_response(&r, 200, &hdr(5, 4, 1_000, "b"), now);
        }
        assert!(rl.buckets.len() >= 1_100);

        // Long after they all went idle, ordinary traffic cleans them up.
        let r = Route::new("GET", "/users/@me");
        for _ in 0..(PRUNE_EVERY + 1) {
            rl.acquire(&r, 400_000);
        }
        assert!(rl.buckets.len() < 10, "left {}", rl.buckets.len());
    }

    #[test]
    fn dead_webhook_memory_is_capped_oldest_first() {
        let mut rl = RateLimiter::default();
        let first = Route::new("POST", "/webhooks/0/tok");
        for i in 0..=MAX_DEAD_WEBHOOKS {
            let r = Route::new("POST", &format!("/webhooks/{i}/tok"));
            rl.acquire(&r, 0);
            rl.on_response(&r, 404, &RateLimitHeaders::default(), 0);
        }
        assert_eq!(rl.dead_webhooks.len(), MAX_DEAD_WEBHOOKS);
        // The oldest entry was forgotten; the newest is still dead.
        // (Asked later, once the global window has cleared, so Go is the clean answer.)
        assert_eq!(rl.acquire(&first, 5_000), Verdict::Go);
        let newest = Route::new("POST", &format!("/webhooks/{MAX_DEAD_WEBHOOKS}/tok"));
        assert_eq!(rl.acquire(&newest, 5_000), Verdict::Blocked(BlockReason::DeadWebhook));
    }
}
