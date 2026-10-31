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

#[cfg(feature = "serde")]
impl serde::Serialize for Snowflake {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(&self.0)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Snowflake {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SnowflakeVisitor;

        impl<'de> serde::de::Visitor<'de> for SnowflakeVisitor {
            type Value = Snowflake;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a snowflake as a string or u64 integer")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                value.parse::<u64>().map(Snowflake).map_err(E::custom)
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Snowflake(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if value < 0 {
                    Err(E::custom("snowflake cannot be negative"))
                } else {
                    Ok(Snowflake(value as u64))
                }
            }
        }

        deserializer.deserialize_any(SnowflakeVisitor)
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

    #[cfg(feature = "serde")]
    #[test]
    fn serde_string_and_integer_roundtrip() {
        let s = Snowflake::new(EXAMPLE);
        // String format
        let json_str = format!("\"{}\"", EXAMPLE);
        let deserialized: Snowflake = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized, s);

        // Integer format
        let json_int = format!("{}", EXAMPLE);
        let deserialized_int: Snowflake = serde_json::from_str(&json_int).unwrap();
        assert_eq!(deserialized_int, s);

        // Serialization format is string (Discord convention)
        let serialized = serde_json::to_string(&s).unwrap();
        assert_eq!(serialized, json_str);
    }
}
