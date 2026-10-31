use rift_core::Snowflake;
use serde::{Deserialize, Serialize};

/// Decoded snowflake parts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnowflakeParts {
    /// Milliseconds since Unix epoch.
    pub timestamp_ms: u64,
    /// Internal Discord worker id (5 bits).
    pub worker_id: u8,
    /// Internal Discord process id (5 bits).
    pub process_id: u8,
    /// Increment counter (12 bits).
    pub increment: u16,
}

/// Returns the creation time of a snowflake as Unix milliseconds.
pub fn timestamp_ms(id: u64) -> u64 {
    Snowflake::new(id).timestamp_ms()
}

/// Returns the internal worker ID (5 bits).
pub fn worker_id(id: u64) -> u8 {
    Snowflake::new(id).worker_id()
}

/// Returns the internal process ID (5 bits).
pub fn process_id(id: u64) -> u8 {
    Snowflake::new(id).process_id()
}

/// Returns the sequence increment (12 bits).
pub fn increment(id: u64) -> u16 {
    Snowflake::new(id).increment()
}

/// Constructs a 64-bit snowflake ID from its constituent parts.
pub fn from_parts(timestamp_ms: u64, worker_id: u8, process_id: u8, increment: u16) -> u64 {
    const DISCORD_EPOCH_MS: u64 = rift_core::snowflake::DISCORD_EPOCH_MS;
    let ts_offset = timestamp_ms.saturating_sub(DISCORD_EPOCH_MS);
    (ts_offset << 22)
        | (((worker_id as u64) & 0x1F) << 17)
        | (((process_id as u64) & 0x1F) << 12)
        | ((increment as u64) & 0xFFF)
}

/// Parses a decimal snowflake string into a `u64`.
pub fn parse_snowflake(s: &str) -> Result<u64, String> {
    s.parse::<u64>()
        .map_err(|e| format!("Invalid snowflake: {e}"))
}

/// Formats a `u64` snowflake into a decimal string.
pub fn format_snowflake(id: u64) -> String {
    id.to_string()
}

/// Deconstructs a snowflake into all constituent parts.
pub fn deconstruct(id: u64) -> SnowflakeParts {
    SnowflakeParts {
        timestamp_ms: timestamp_ms(id),
        worker_id: worker_id(id),
        process_id: process_id(id),
        increment: increment(id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: u64 = 175_928_847_299_117_063;

    #[test]
    fn test_documented_example_parts() {
        assert_eq!(timestamp_ms(EXAMPLE), 1_462_015_105_796);
        assert_eq!(worker_id(EXAMPLE), 1);
        assert_eq!(process_id(EXAMPLE), 0);
        assert_eq!(increment(EXAMPLE), 7);
    }

    #[test]
    fn test_from_parts_roundtrip() {
        let reconstructed = from_parts(1_462_015_105_796, 1, 0, 7);
        assert_eq!(reconstructed, EXAMPLE);
    }

    #[test]
    fn test_parse_and_format() {
        assert_eq!(parse_snowflake("175928847299117063").unwrap(), EXAMPLE);
        assert_eq!(format_snowflake(EXAMPLE), "175928847299117063");
        assert!(parse_snowflake("invalid").is_err());
    }

    #[test]
    fn test_deconstruct() {
        let parts = deconstruct(EXAMPLE);
        assert_eq!(parts.timestamp_ms, 1_462_015_105_796);
        assert_eq!(parts.worker_id, 1);
        assert_eq!(parts.process_id, 0);
        assert_eq!(parts.increment, 7);
    }
}
