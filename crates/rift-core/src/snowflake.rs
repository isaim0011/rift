//! Discord snowflake IDs.

use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

/// Milliseconds between the Unix epoch and the Discord epoch (2015-01-01).
pub const DISCORD_EPOCH_MS: u64 = 1_420_070_400_000;

/// A Discord snowflake. `Copy`, 8 bytes, hashable.
///
/// Discord sends these as JSON strings; parse with [`FromStr`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Snowflake(u64);

impl Snowflake {
    /// Wrap a raw id.
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// The raw id.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Creation time as Unix milliseconds.
    pub const fn timestamp_ms(self) -> u64 {
        (self.0 >> 22) + DISCORD_EPOCH_MS
    }

    /// Internal worker id (5 bits).
    pub const fn worker_id(self) -> u8 {
        ((self.0 & 0x3E_0000) >> 17) as u8
    }

    /// Internal process id (5 bits).
    pub const fn process_id(self) -> u8 {
        ((self.0 & 0x1_F000) >> 12) as u8
    }

    /// Per-process increment (12 bits).
    pub const fn increment(self) -> u16 {
        (self.0 & 0xFFF) as u16
    }
}

impl FromStr for Snowflake {
    type Err = ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u64>().map(Self)
    }
}

impl fmt::Display for Snowflake {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Example id from Discord's documentation.
    const EXAMPLE: u64 = 175_928_847_299_117_063;

    #[test]
    fn decodes_documented_example() {
        let s = Snowflake::new(EXAMPLE);
        assert_eq!(s.timestamp_ms(), 1_462_015_105_796);
        assert_eq!(s.worker_id(), 1);
        assert_eq!(s.process_id(), 0);
        assert_eq!(s.increment(), 7);
    }

    #[test]
    fn parses_and_displays() {
        let s: Snowflake = "175928847299117063".parse().unwrap();
        assert_eq!(s.get(), EXAMPLE);
        assert_eq!(s.to_string(), "175928847299117063");
        assert!("nope".parse::<Snowflake>().is_err());
    }
}
