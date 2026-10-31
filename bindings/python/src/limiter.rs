use rift_http::{BlockReason, Config, RateLimitHeaders, RateLimiter, Route, Scope, Verdict};

/// Normalized route descriptor for Python.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PyNormalizedRoute {
    pub id: String,
    pub template: String,
    pub major: String,
}

/// Normalizes HTTP method and path for rate-limit bucket grouping.
pub fn normalize_route(method: &str, path: &str) -> PyNormalizedRoute {
    let r = Route::new(method, path);
    PyNormalizedRoute {
        id: format!("{} {}", method.to_ascii_uppercase(), r.template()),
        template: r.template().to_string(),
        major: r.major().to_string(),
    }
}

/// Discord REST bucket rate limiter for Python.
pub struct PyRateLimiter {
    inner: RateLimiter,
}

impl Default for PyRateLimiter {
    fn default() -> Self {
        Self::new(None, None, None)
    }
}

impl PyRateLimiter {
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

    /// Returns (verdict: "go"|"wait"|"blocked", retry_after_ms: Option<u64>, reason: Option<String>)
    pub fn acquire(
        &mut self,
        method: &str,
        path: &str,
        now_ms: u64,
    ) -> (String, Option<u64>, Option<String>) {
        let route = Route::new(method, path);
        match self.inner.acquire(&route, now_ms) {
            Verdict::Go => ("go".to_string(), None, None),
            Verdict::Wait { retry_after_ms } => {
                ("wait".to_string(), Some(retry_after_ms), None)
            }
            Verdict::Blocked(reason) => match reason {
                BlockReason::CircuitOpen { retry_after_ms } => (
                    "blocked".to_string(),
                    Some(retry_after_ms),
                    Some("circuit_open".to_string()),
                ),
                BlockReason::DeadWebhook => (
                    "blocked".to_string(),
                    None,
                    Some("dead_webhook".to_string()),
                ),
            },
        }
    }

    pub fn on_response(
        &mut self,
        method: &str,
        path: &str,
        status: u16,
        limit: Option<u32>,
        remaining: Option<u32>,
        reset_after_ms: Option<u64>,
        bucket: Option<String>,
        scope: Option<String>,
        now_ms: u64,
    ) {
        let route = Route::new(method, path);
        let sc = scope.as_deref().and_then(|s| match s {
            "user" => Some(Scope::User),
            "global" => Some(Scope::Global),
            "shared" => Some(Scope::Shared),
            _ => None,
        });

        let headers = RateLimitHeaders {
            limit,
            remaining,
            reset_after_ms,
            bucket,
            global: false,
            scope: sc,
            retry_after_ms: None,
        };

        self.inner.on_response(&route, status, &headers, now_ms);
    }

    pub fn invalid_requests(&self, now_ms: u64) -> u32 {
        self.inner.invalid_requests(now_ms)
    }
}

#[cfg(feature = "pyo3-binding")]
pub mod pyo3_impl {
    use super::*;
    use pyo3::prelude::*;

    #[pyclass(name = "NormalizedRoute")]
    #[derive(Clone, Debug)]
    pub struct PyNormalizedRouteClass {
        #[pyo3(get)]
        pub id: String,
        #[pyo3(get)]
        pub template: String,
        #[pyo3(get)]
        pub major: String,
    }

    #[pyfunction(name = "normalize_route")]
    pub fn py_normalize_route(method: &str, path: &str) -> PyNormalizedRouteClass {
        let r = normalize_route(method, path);
        PyNormalizedRouteClass {
            id: r.id,
            template: r.template,
            major: r.major,
        }
    }

    #[pyclass(name = "RateLimiter")]
    pub struct PyRateLimiterClass {
        inner: PyRateLimiter,
    }

    #[pymethods]
    impl PyRateLimiterClass {
        #[new]
        #[pyo3(signature = (global_per_second=None, invalid_limit=None, invalid_trip_at=None))]
        pub fn new(
            global_per_second: Option<u32>,
            invalid_limit: Option<u32>,
            invalid_trip_at: Option<u32>,
        ) -> Self {
            Self {
                inner: PyRateLimiter::new(global_per_second, invalid_limit, invalid_trip_at),
            }
        }

        pub fn acquire(
            &mut self,
            method: &str,
            path: &str,
            now_ms: u64,
        ) -> (String, Option<u64>, Option<String>) {
            self.inner.acquire(method, path, now_ms)
        }

        #[pyo3(signature = (method, path, status, limit=None, remaining=None, reset_after_ms=None, bucket=None, scope=None, now_ms=0))]
        pub fn on_response(
            &mut self,
            method: &str,
            path: &str,
            status: u16,
            limit: Option<u32>,
            remaining: Option<u32>,
            reset_after_ms: Option<u64>,
            bucket: Option<String>,
            scope: Option<String>,
            now_ms: u64,
        ) {
            self.inner.on_response(
                method,
                path,
                status,
                limit,
                remaining,
                reset_after_ms,
                bucket,
                scope,
                now_ms,
            );
        }

        pub fn invalid_requests(&self, now_ms: u64) -> u32 {
            self.inner.invalid_requests(now_ms)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_py_limiter_acquire() {
        let mut limiter = PyRateLimiter::default();
        let (verdict, retry, _) = limiter.acquire("GET", "/channels/123/messages", 1000);
        assert_eq!(verdict, "go");
        assert_eq!(retry, None);

        let (verdict2, retry2, _) = limiter.acquire("GET", "/channels/123/messages", 1005);
        assert_eq!(verdict2, "wait");
        assert_eq!(retry2, Some(50));
    }
}
