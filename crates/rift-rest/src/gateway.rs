use rift_gateway::IdentifyBudget;
use serde::Deserialize;
use std::num::NonZeroU32;

/// `session_start_limit` from Get Gateway Bot.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct SessionStartLimit {
    /// Total session starts allowed per window.
    pub total: u32,
    /// Session starts remaining.
    pub remaining: u32,
    /// Milliseconds until the limit resets.
    pub reset_after: u64,
    /// Identify requests allowed per 5 seconds.
    pub max_concurrency: u32,
}

/// Response of `GET /gateway/bot`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct GatewayBot {
    /// WSS URL to connect to.
    pub url: String,
    /// Recommended shard count.
    pub shards: u32,
    /// Current session-start limits.
    pub session_start_limit: SessionStartLimit,
}

impl GatewayBot {
    /// Build an [`IdentifyBudget`] from the real limits, keeping `reserve`
    /// Identifies untouched. `now_ms` must come from the same clock the shards
    /// use (`rift_runtime::now_ms()`).
    pub fn identify_budget(&self, now_ms: u64, reserve: u32) -> IdentifyBudget {
        let l = &self.session_start_limit;
        IdentifyBudget::from_session_start_limit(l.total, l.remaining, l.reset_after, now_ms, reserve)
    }

    /// Shards grouped into start waves per `max_concurrency` (see the docs'
    /// "Max Concurrency" rules). Start each wave in order.
    pub fn start_waves(&self) -> Vec<Vec<u32>> {
        let c = NonZeroU32::new(self.session_start_limit.max_concurrency).unwrap_or(NonZeroU32::MIN);
        rift_core::shard::start_waves(self.shards, c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The example response from the Gateway docs.
    const DOCS_EXAMPLE: &str = r#"{
        "url": "wss://gateway.discord.gg/",
        "shards": 9,
        "session_start_limit": {
            "total": 1000, "remaining": 999, "reset_after": 14400000, "max_concurrency": 1
        }
    }"#;

    #[test]
    fn parses_documented_example() {
        let g: GatewayBot = serde_json::from_str(DOCS_EXAMPLE).unwrap();
        assert_eq!(g.url, "wss://gateway.discord.gg/");
        assert_eq!(g.shards, 9);
        assert_eq!(g.session_start_limit.remaining, 999);
        assert_eq!(g.session_start_limit.max_concurrency, 1);
    }

    #[test]
    fn nine_shards_with_concurrency_one_start_one_at_a_time() {
        let g: GatewayBot = serde_json::from_str(DOCS_EXAMPLE).unwrap();
        let waves = g.start_waves();
        assert_eq!(waves.len(), 9);
        assert!(waves.iter().all(|w| w.len() == 1));
    }

    #[test]
    fn budget_uses_the_real_remaining_count() {
        let g: GatewayBot = serde_json::from_str(DOCS_EXAMPLE).unwrap();
        let mut b = g.identify_budget(0, 10);
        assert_eq!(b.remaining(), 999);
        assert!(b.try_acquire(0).is_ok());
    }

    #[test]
    fn ignores_unknown_fields() {
        let g: Result<GatewayBot, _> = serde_json::from_str(
            r#"{"url":"wss://x/","shards":1,"future_field":true,
                "session_start_limit":{"total":1,"remaining":1,"reset_after":1,"max_concurrency":0}}"#,
        );
        let g = g.unwrap();
        // max_concurrency of 0 must not panic.
        assert_eq!(g.start_waves(), vec![vec![0]]);
    }
}
