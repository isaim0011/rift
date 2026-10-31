use rift_http::{BlockReason, Config, RateLimitHeaders, RateLimiter, Route, Scope, Verdict};
use serde::{Deserialize, Serialize};

/// Normalized route descriptor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedRoute {
    /// Full normalized route ID, e.g. "GET /channels/:major/messages/:id".
    pub id: String,
    /// Path template, e.g. "/channels/:major/messages/:id".
    pub template: String,
    /// Major parameter key, e.g. "channels/12345".
    pub major: String,
}

/// Normalizes an HTTP method and path for Discord REST rate-limit bucket grouping.
pub fn normalize_route(method: &str, path: &str) -> NormalizedRoute {
    let r = Route::new(method, path);
    NormalizedRoute {
        id: format!("{} {}", method.to_ascii_uppercase(), r.template()),
        template: r.template().to_string(),
        major: r.major().to_string(),
    }
}

/// Result of acquiring a rate-limit slot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcquireVerdict {
    /// "go", "wait", or "blocked".
    pub verdict: String,
    /// Milliseconds to wait before retrying (if "wait").
    pub retry_after_ms: Option<u64>,
    /// Reason if blocked ("circuit_open", "dead_webhook").
    pub reason: Option<String>,
}

/// Rate limit response headers input.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ResponseHeadersInput {
    pub limit: Option<u32>,
    pub remaining: Option<u32>,
    pub reset_after_ms: Option<u64>,
    pub bucket: Option<String>,
    pub global: Option<bool>,
    pub scope: Option<String>,
    pub retry_after_ms: Option<u64>,
}

/// In-memory REST bucket rate limiter wrapping `rift_http::RateLimiter`.
pub struct NodeRateLimiter {
    inner: RateLimiter,
}

impl NodeRateLimiter {
    /// Creates a new rate limiter with optional tunables.
    pub fn new(
        global_per_second: Option<u32>,
        invalid_limit: Option<u32>,
        invalid_trip_at: Option<u32>,
    ) -> Self {
        let mut cfg = Config::default();
        if let Some(gps) = global_per_second {
            cfg.global_per_second = gps;
        }
        if let Some(il) = invalid_limit {
            cfg.invalid_limit = il;
        }
        if let Some(ita) = invalid_trip_at {
            cfg.invalid_trip_at = ita;
        }
        Self {
            inner: RateLimiter::new(cfg),
        }
    }

    /// Acquires permission to send a request.
    pub fn acquire(&mut self, method: &str, path: &str, now_ms: u64) -> AcquireVerdict {
        let route = Route::new(method, path);
        match self.inner.acquire(&route, now_ms) {
            Verdict::Go => AcquireVerdict {
                verdict: "go".to_string(),
                retry_after_ms: None,
                reason: None,
            },
            Verdict::Wait { retry_after_ms } => AcquireVerdict {
                verdict: "wait".to_string(),
                retry_after_ms: Some(retry_after_ms),
                reason: None,
            },
            Verdict::Blocked(reason) => {
                let r_str = match reason {
                    BlockReason::CircuitOpen { retry_after_ms } => {
                        return AcquireVerdict {
                            verdict: "blocked".to_string(),
                            retry_after_ms: Some(retry_after_ms),
                            reason: Some("circuit_open".to_string()),
                        };
                    }
                    BlockReason::DeadWebhook => "dead_webhook",
                };
                AcquireVerdict {
                    verdict: "blocked".to_string(),
                    retry_after_ms: None,
                    reason: Some(r_str.to_string()),
                }
            }
        }
    }

    /// Feeds response status and rate-limit headers back into the bucket tracker.
    pub fn on_response(
        &mut self,
        method: &str,
        path: &str,
        status: u16,
        headers: ResponseHeadersInput,
        now_ms: u64,
    ) {
        let route = Route::new(method, path);
        let scope = headers.scope.as_deref().and_then(|s| match s {
            "user" => Some(Scope::User),
            "global" => Some(Scope::Global),
            "shared" => Some(Scope::Shared),
            _ => None,
        });

        let r_headers = RateLimitHeaders {
            limit: headers.limit,
            remaining: headers.remaining,
            reset_after_ms: headers.reset_after_ms,
            bucket: headers.bucket,
            global: headers.global.unwrap_or(false),
            scope,
            retry_after_ms: headers.retry_after_ms,
        };

        self.inner.on_response(&route, status, &r_headers, now_ms);
    }

    /// Returns the number of invalid requests tracked in the rolling 10-minute window.
    pub fn invalid_requests(&self, now_ms: u64) -> u32 {
        self.inner.invalid_requests(now_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_route() {
        let norm = normalize_route("GET", "/channels/12345/messages/67890?limit=50");
        assert_eq!(norm.template, "/channels/:major/messages/:id");
        assert_eq!(norm.major, "channels/12345");
        assert_eq!(norm.id, "GET /channels/:major/messages/:id");
    }

    #[test]
    fn test_limiter_acquire_and_probe() {
        let mut limiter = NodeRateLimiter::new(None, None, None);
        // First request is a probe: Go
        let v1 = limiter.acquire("GET", "/channels/123/messages", 1000);
        assert_eq!(v1.verdict, "go");

        // Second request before response must wait for probe
        let v2 = limiter.acquire("GET", "/channels/123/messages", 1005);
        assert_eq!(v2.verdict, "wait");

        // Feed back response with remaining = 5
        limiter.on_response(
            "GET",
            "/channels/123/messages",
            200,
            ResponseHeadersInput {
                limit: Some(10),
                remaining: Some(5),
                reset_after_ms: Some(1000),
                bucket: Some("b1".to_string()),
                ..Default::default()
            },
            1010,
        );

        // Next acquire succeeds
        let v3 = limiter.acquire("GET", "/channels/123/messages", 1015);
        assert_eq!(v3.verdict, "go");
    }
}
