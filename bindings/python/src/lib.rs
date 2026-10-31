//! Python bindings for Rift Discord engine using PyO3.

pub mod cache;
pub mod client;
pub mod gateway;
pub mod intents;
pub mod limiter;
pub mod permissions;
pub mod snowflake;
pub mod verify;

pub use cache::PyCache;
pub use client::PyRiftClient;
pub use gateway::PyGatewaySession;
pub use limiter::{normalize_route, PyNormalizedRoute, PyRateLimiter};
pub use permissions::PyPermissions;
pub use snowflake::PySnowflake;
pub use verify::verify_interaction;

#[cfg(feature = "pyo3-binding")]
use pyo3::prelude::*;

#[cfg(feature = "pyo3-binding")]
#[pymodule]
fn _rift(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<snowflake::pyo3_impl::PySnowflakeClass>()?;
    m.add_class::<permissions::pyo3_impl::PyPermissionsClass>()?;
    m.add_class::<intents::pyo3_impl::PyIntentsClass>()?;
    m.add_class::<intents::pyo3_impl::PyCapabilitiesClass>()?;
    m.add_class::<limiter::pyo3_impl::PyNormalizedRouteClass>()?;
    m.add_class::<limiter::pyo3_impl::PyRateLimiterClass>()?;
    m.add_class::<cache::pyo3_impl::PyCacheClass>()?;
    m.add_class::<gateway::pyo3_impl::PyGatewaySessionClass>()?;
    m.add_class::<client::pyo3_impl::PyRiftClientClass>()?;
    m.add_function(wrap_pyfunction!(verify::pyo3_impl::py_verify_interaction, m)?)?;
    m.add_function(wrap_pyfunction!(limiter::pyo3_impl::py_normalize_route, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_py_modules_link() {
        let ts = PySnowflake::timestamp_ms(175_928_847_299_117_063);
        assert_eq!(ts, 1_462_015_105_796);

        assert!(PyPermissions::has_permission(permissions::ADMINISTRATOR, permissions::VIEW_CHANNEL));

        assert!(!verify_interaction("bad", "bad", "123", b"data"));

        let norm = normalize_route("GET", "/channels/1/messages");
        assert_eq!(norm.major, "channels/1");

        let cache = PyCache::default();
        assert_eq!(cache.stats().0, 0);

        let mut gw = PyGatewaySession::new();
        assert_eq!(gw.poll_heartbeat(0).0, "wait");

        let client = PyRiftClient::new("TOKEN".to_string(), None, None);
        assert_eq!(client.capabilities(), 0);
    }
}
