use rift_core::Snowflake;

/// Snowflake ID calculation and parsing for Python.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PySnowflake;

impl PySnowflake {
    /// Milliseconds since Unix epoch for the snowflake creation time.
    pub fn timestamp_ms(id: u64) -> u64 {
        Snowflake::new(id).timestamp_ms()
    }

    /// Internal Discord worker id (5 bits).
    pub fn worker_id(id: u64) -> u8 {
        Snowflake::new(id).worker_id()
    }

    /// Internal Discord process id (5 bits).
    pub fn process_id(id: u64) -> u8 {
        Snowflake::new(id).process_id()
    }

    /// Sequence increment (12 bits).
    pub fn increment(id: u64) -> u16 {
        Snowflake::new(id).increment()
    }

    /// Constructs a snowflake from its constituent components.
    pub fn from_parts(timestamp_ms: u64, worker_id: u8, process_id: u8, increment: u16) -> u64 {
        const DISCORD_EPOCH_MS: u64 = rift_core::snowflake::DISCORD_EPOCH_MS;
        let ts_offset = timestamp_ms.saturating_sub(DISCORD_EPOCH_MS);
        (ts_offset << 22)
            | (((worker_id as u64) & 0x1F) << 17)
            | (((process_id as u64) & 0x1F) << 12)
            | ((increment as u64) & 0xFFF)
    }

    /// Deconstructs a snowflake into a tuple of (timestamp_ms, worker_id, process_id, increment).
    pub fn deconstruct(id: u64) -> (u64, u8, u8, u16) {
        (
            Self::timestamp_ms(id),
            Self::worker_id(id),
            Self::process_id(id),
            Self::increment(id),
        )
    }
}

#[cfg(feature = "pyo3-binding")]
pub mod pyo3_impl {
    use super::*;
    use pyo3::prelude::*;

    #[pyclass(name = "Snowflake")]
    #[derive(Clone, Debug, Default)]
    pub struct PySnowflakeClass;

    #[pymethods]
    impl PySnowflakeClass {
        #[staticmethod]
        pub fn timestamp_ms(id: u64) -> u64 {
            PySnowflake::timestamp_ms(id)
        }

        #[staticmethod]
        pub fn worker_id(id: u64) -> u8 {
            PySnowflake::worker_id(id)
        }

        #[staticmethod]
        pub fn process_id(id: u64) -> u8 {
            PySnowflake::process_id(id)
        }

        #[staticmethod]
        pub fn increment(id: u64) -> u16 {
            PySnowflake::increment(id)
        }

        #[staticmethod]
        pub fn from_parts(timestamp_ms: u64, worker_id: u8, process_id: u8, increment: u16) -> u64 {
            PySnowflake::from_parts(timestamp_ms, worker_id, process_id, increment)
        }

        #[staticmethod]
        pub fn deconstruct(id: u64) -> (u64, u8, u8, u16) {
            PySnowflake::deconstruct(id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: u64 = 175_928_847_299_117_063;

    #[test]
    fn test_py_snowflake_parts() {
        assert_eq!(PySnowflake::timestamp_ms(EXAMPLE), 1_462_015_105_796);
        assert_eq!(PySnowflake::worker_id(EXAMPLE), 1);
        assert_eq!(PySnowflake::process_id(EXAMPLE), 0);
        assert_eq!(PySnowflake::increment(EXAMPLE), 7);
    }

    #[test]
    fn test_py_snowflake_from_parts() {
        let reconstructed = PySnowflake::from_parts(1_462_015_105_796, 1, 0, 7);
        assert_eq!(reconstructed, EXAMPLE);
    }

    #[test]
    fn test_py_snowflake_deconstruct() {
        let parts = PySnowflake::deconstruct(EXAMPLE);
        assert_eq!(parts, (1_462_015_105_796, 1, 0, 7));
    }
}
