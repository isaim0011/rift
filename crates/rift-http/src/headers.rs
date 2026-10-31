/// `X-RateLimit-Scope` on 429 responses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Per bot/user limit.
    User,
    /// Global limit.
    Global,
    /// Per-resource limit shared between users (does not count as an invalid request).
    Shared,
}

/// Parsed rate-limit headers from one response.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RateLimitHeaders {
    /// `X-RateLimit-Limit`
    pub limit: Option<u32>,
    /// `X-RateLimit-Remaining`
    pub remaining: Option<u32>,
    /// `X-RateLimit-Reset-After`, in milliseconds.
    pub reset_after_ms: Option<u64>,
    /// `X-RateLimit-Bucket`
    pub bucket: Option<String>,
    /// `X-RateLimit-Global` present on a 429.
    pub global: bool,
    /// `X-RateLimit-Scope`
    pub scope: Option<Scope>,
    /// `Retry-After` in milliseconds. If the header is missing, fill this from
    /// the 429 JSON body's `retry_after`.
    pub retry_after_ms: Option<u64>,
}

fn secs_to_ms(s: &str) -> Option<u64> {
    let v: f64 = s.trim().parse().ok()?;
    if v.is_finite() {
        Some((v.max(0.0) * 1000.0).round() as u64)
    } else {
        None
    }
}

impl RateLimitHeaders {
    /// Parse from any header lookup. `get` must be case-insensitive.
    pub fn parse<'a>(get: impl Fn(&str) -> Option<&'a str>) -> Self {
        Self {
            limit: get("x-ratelimit-limit").and_then(|v| v.trim().parse().ok()),
            remaining: get("x-ratelimit-remaining").and_then(|v| v.trim().parse().ok()),
            reset_after_ms: get("x-ratelimit-reset-after").and_then(secs_to_ms),
            bucket: get("x-ratelimit-bucket").map(str::to_owned),
            global: get("x-ratelimit-global").is_some_and(|v| v.trim().eq_ignore_ascii_case("true")),
            scope: get("x-ratelimit-scope").and_then(|v| match v.trim() {
                "user" => Some(Scope::User),
                "global" => Some(Scope::Global),
                "shared" => Some(Scope::Shared),
                _ => None,
            }),
            retry_after_ms: get("retry-after").and_then(secs_to_ms),
        }
    }

    pub(crate) fn is_global(&self) -> bool {
        self.global || self.scope == Some(Scope::Global)
    }

    pub(crate) fn has_bucket_info(&self) -> bool {
        self.limit.is_some() || self.remaining.is_some() || self.reset_after_ms.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<&'a str> {
        move |name| pairs.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| *v)
    }

    #[test]
    fn parses_documented_example() {
        let pairs = [
            ("X-RateLimit-Limit", "5"),
            ("X-RateLimit-Remaining", "0"),
            ("X-RateLimit-Reset-After", "1"),
            ("X-RateLimit-Bucket", "abcd1234"),
        ];
        let h = RateLimitHeaders::parse(lookup(&pairs));
        assert_eq!(h.limit, Some(5));
        assert_eq!(h.remaining, Some(0));
        assert_eq!(h.reset_after_ms, Some(1000));
        assert_eq!(h.bucket.as_deref(), Some("abcd1234"));
        assert!(!h.is_global());
    }

    #[test]
    fn parses_fractional_seconds_and_429_fields() {
        let pairs = [
            ("Retry-After", "64.57"),
            ("X-RateLimit-Global", "true"),
            ("X-RateLimit-Scope", "global"),
        ];
        let h = RateLimitHeaders::parse(lookup(&pairs));
        assert_eq!(h.retry_after_ms, Some(64_570));
        // 0.1s must be 100ms, not 101ms from float error.
        assert_eq!(secs_to_ms("0.1"), Some(100));
        assert!(h.is_global());
        assert_eq!(h.scope, Some(Scope::Global));
    }

    #[test]
    fn missing_headers_are_none() {
        let h = RateLimitHeaders::parse(|_| None);
        assert_eq!(h, RateLimitHeaders::default());
        assert!(!h.has_bucket_info());
    }
}
