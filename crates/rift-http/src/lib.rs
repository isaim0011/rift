//! # rift-http
//!
//! Sans-IO REST rate limiting. Nothing here sends a request or reads a clock:
//! the HTTP client (next layer) calls [`RateLimiter::acquire`] before sending
//! and [`RateLimiter::on_response`] after, passing `now_ms`.
//!
//! Rules implemented, all from the Rate Limits docs:
//! - limits are learned from response headers, never hardcoded
//! - per-route buckets, grouped by `X-RateLimit-Bucket`, keyed per top-level
//!   resource (channel, guild, webhook)
//! - global limit of 50 requests/second (interaction endpoints exempt)
//! - 429 handling for `user`, `global` and `shared` scopes
//! - invalid-request circuit breaker (10,000 per 10 minutes; 401/403/429, but
//!   `shared` 429s don't count)
//! - dead webhooks (404) are never retried

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod headers;
mod invalid;
mod limiter;
mod route;
pub mod verify;

pub use headers::{RateLimitHeaders, Scope};
pub use limiter::{BlockReason, Config, RateLimiter, Verdict};
pub use route::Route;
pub use verify::{
    verify_interaction, verify_interaction_custom, verify_interaction_raw, verify_interaction_with_timestamp,
    Ed25519Verifier, Verifier, VerifyError, HEADER_SIGNATURE, HEADER_TIMESTAMP,
};
