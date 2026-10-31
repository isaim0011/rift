//! Sharding math, straight from the Gateway docs.

use crate::snowflake::Snowflake;
use std::num::NonZeroU32;

/// Hard limit: a single shard supports at most this many guilds.
pub const MAX_GUILDS_PER_SHARD: u32 = 2500;

/// `shard_id = (guild_id >> 22) % num_shards`
pub fn shard_for_guild(guild_id: Snowflake, num_shards: NonZeroU32) -> u32 {
    ((guild_id.get() >> 22) % u64::from(num_shards.get())) as u32
}

/// Events with no `guild_id` (DMs, entitlements, subscriptions) are only
/// delivered to shard 0.
pub fn shard_for_event(guild_id: Option<Snowflake>, num_shards: NonZeroU32) -> u32 {
    guild_id.map_or(0, |g| shard_for_guild(g, num_shards))
}

/// Identify rate-limit key: `shard_id % max_concurrency`.
pub fn rate_limit_key(shard_id: u32, max_concurrency: NonZeroU32) -> u32 {
    shard_id % max_concurrency.get()
}

/// Split `0..num_shards` into start "waves". Shards inside a wave have
/// distinct rate-limit keys and may identify concurrently; waves must run in
/// order, one every 5 seconds.
pub fn start_waves(num_shards: u32, max_concurrency: NonZeroU32) -> Vec<Vec<u32>> {
    let c = max_concurrency.get();
    let mut waves: Vec<Vec<u32>> = Vec::new();
    for id in 0..num_shards {
        let w = (id / c) as usize;
        if waves.len() <= w {
            waves.push(Vec::with_capacity(c as usize));
        }
        waves[w].push(id);
    }
    waves
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nz(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).unwrap()
    }

    #[test]
    fn dm_events_go_to_shard_zero() {
        assert_eq!(shard_for_event(None, nz(7)), 0);
    }

    #[test]
    fn guild_routing_uses_timestamp_bits() {
        let g = Snowflake::new(175_928_847_299_117_063);
        assert_eq!(shard_for_guild(g, nz(3)), ((g.get() >> 22) % 3) as u32);
        assert_eq!(shard_for_guild(g, nz(1)), 0);
    }

    #[test]
    fn thirty_two_shards_two_waves() {
        let waves = start_waves(32, nz(16));
        assert_eq!(waves.len(), 2);
        assert_eq!(waves[0], (0..16).collect::<Vec<_>>());
        assert_eq!(waves[1], (16..32).collect::<Vec<_>>());
        // Keys inside a wave are unique.
        for w in &waves {
            let mut keys: Vec<_> = w.iter().map(|&s| rate_limit_key(s, nz(16))).collect();
            keys.sort_unstable();
            keys.dedup();
            assert_eq!(keys.len(), w.len());
        }
    }

    #[test]
    fn partial_last_wave() {
        let waves = start_waves(5, nz(2));
        assert_eq!(waves, vec![vec![0, 1], vec![2, 3], vec![4]]);
    }
}
