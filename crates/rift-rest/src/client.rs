use crate::error::Error;
use crate::gateway::GatewayBot;
use crate::transport::{Multipart, RawResponse, ReqwestTransport, Request, Transport};
use rift_http::{Config, RateLimitHeaders, RateLimiter, Route, Verdict};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::fmt;
use std::sync::Mutex;
use std::time::Duration;
use tokio::time::Instant;

/// Base URL including the API version.
pub const API_BASE: &str = "https://discord.com/api/v10";

/// Give up after this many 429s on one request.
const MAX_ATTEMPTS: u32 = 5;

/// Build the required `User-Agent`: `DiscordBot ($url, $versionNumber)`.
pub fn user_agent(url: &str, version: &str) -> String {
    format!("DiscordBot ({url}, {version})")
}

/// A completed API response.
#[derive(Clone, Debug)]
pub struct Response {
    /// HTTP status.
    pub status: u16,
    /// Response headers.
    pub headers: Vec<(String, String)>,
    /// Raw body.
    pub body: Vec<u8>,
}

impl Response {
    /// True for 2xx.
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Body as lossy UTF-8.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    /// Turn a non-2xx response into an error, using Discord's `{"code","message"}`
    /// body when present.
    pub fn expect_success(self) -> Result<Response, Error> {
        if self.is_success() {
            return Ok(self);
        }
        #[derive(Deserialize)]
        struct Api {
            code: Option<u64>,
            message: Option<String>,
        }
        match serde_json::from_slice::<Api>(&self.body) {
            Ok(Api { code, message: Some(message) }) => {
                Err(Error::Api { status: self.status, code, message })
            }
            _ => Err(Error::Status { status: self.status, body: self.text() }),
        }
    }

    /// Decode the body as JSON.
    pub fn json<T: DeserializeOwned>(&self) -> Result<T, Error> {
        serde_json::from_slice(&self.body).map_err(|e| Error::Decode(e.to_string()))
    }
}

/// Rate-limited, retrying REST client. Share it across tasks with `Arc`.
pub struct RestClient<T: Transport> {
    transport: T,
    limiter: Mutex<RateLimiter>,
    auth: String,
    user_agent: String,
    base: String,
    epoch: Instant,
}

// Manual Debug so the token can never leak through `{:?}`.
impl<T: Transport> fmt::Debug for RestClient<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RestClient").field("base", &self.base).finish_non_exhaustive()
    }
}

impl RestClient<ReqwestTransport> {
    /// Client using reqwest. `contact_url` and `version` go in the User-Agent
    /// (the docs require the `DiscordBot ($url, $versionNumber)` format).
    pub fn new(token: &str, contact_url: &str, version: &str) -> Result<Self, Error> {
        let transport = ReqwestTransport::new().map_err(Error::Transport)?;
        Ok(Self::with_transport(transport, token, user_agent(contact_url, version), Config::default()))
    }
}

impl<T: Transport> RestClient<T> {
    /// Client with a custom transport and limiter config.
    pub fn with_transport(transport: T, token: &str, user_agent: String, cfg: Config) -> Self {
        Self {
            transport,
            limiter: Mutex::new(RateLimiter::new(cfg)),
            auth: format!("Bot {token}"),
            user_agent,
            base: API_BASE.to_owned(),
            epoch: Instant::now(),
        }
    }

    /// The underlying transport.
    pub fn transport(&self) -> &T {
        &self.transport
    }

    fn now_ms(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
    }

    fn report(&self, route: &Route, status: u16, h: &RateLimitHeaders) {
        let now = self.now_ms();
        self.limiter.lock().unwrap_or_else(|e| e.into_inner()).on_response(route, status, h, now);
    }

    fn headers(&self, has_body: bool) -> Vec<(String, String)> {
        let mut h = vec![
            ("Authorization".to_owned(), self.auth.clone()),
            ("User-Agent".to_owned(), self.user_agent.clone()),
        ];
        if has_body {
            h.push(("Content-Type".to_owned(), "application/json".to_owned()));
        }
        h
    }

    /// Send a request with optional custom headers, waiting for rate limits and retrying 429s.
    pub async fn request_with_headers(
        &self,
        method: &str,
        path: &str,
        body: Option<Vec<u8>>,
        extra_headers: Vec<(String, String)>,
    ) -> Result<Response, Error> {
        let route = Route::new(method, path);
        let mut attempts = 0u32;

        loop {
            // 1. Wait until the limiter lets us through.
            loop {
                let now = self.now_ms();
                let verdict = self.limiter.lock().unwrap_or_else(|e| e.into_inner()).acquire(&route, now);
                match verdict {
                    Verdict::Go => break,
                    Verdict::Wait { retry_after_ms } => {
                        tokio::time::sleep(Duration::from_millis(retry_after_ms)).await;
                    }
                    Verdict::Blocked(reason) => return Err(Error::Blocked(reason)),
                }
            }

            // 2. Send.
            let mut headers = self.headers(body.is_some());
            headers.extend(extra_headers.clone());
            let req = Request {
                method: method.to_ascii_uppercase(),
                url: format!("{}{}", self.base, path),
                headers,
                body: body.clone(),
                multipart: None,
            };
            let raw: RawResponse = match self.transport.send(req).await {
                Ok(r) => r,
                Err(e) => {
                    // No response: release any probe slot so the route can't wedge.
                    self.report(&route, 0, &RateLimitHeaders::default());
                    return Err(Error::Transport(e));
                }
            };

            // 3. Learn from the response.
            let mut h = RateLimitHeaders::parse(|name| lookup(&raw.headers, name));
            if raw.status == 429 && h.retry_after_ms.is_none() {
                h.retry_after_ms = retry_after_from_body(&raw.body);
            }
            self.report(&route, raw.status, &h);

            // 4. Retry 429s (the limiter now knows how long to wait).
            if raw.status == 429 {
                attempts += 1;
                if attempts >= MAX_ATTEMPTS {
                    return Err(Error::RateLimited);
                }
                continue;
            }

            return Ok(Response { status: raw.status, headers: raw.headers, body: raw.body });
        }
    }

    /// Send a request, waiting for rate limits and retrying 429s.
    ///
    /// `path` starts with `/` and is relative to the API base,
    /// e.g. `/channels/123/messages`. Non-429 error statuses are returned as a
    /// normal [`Response`]; check [`Response::is_success`].
    pub async fn request(&self, method: &str, path: &str, body: Option<Vec<u8>>) -> Result<Response, Error> {
        self.request_with_headers(method, path, body, Vec::new()).await
    }

    /// Send a multipart request, waiting for rate limits and retrying 429s.
    pub async fn request_multipart(
        &self,
        method: &str,
        path: &str,
        multipart: Multipart,
    ) -> Result<Response, Error> {
        self.request_multipart_with_headers(method, path, multipart, Vec::new()).await
    }

    /// Send a multipart request with optional custom headers, waiting for rate limits and retrying 429s.
    pub async fn request_multipart_with_headers(
        &self,
        method: &str,
        path: &str,
        multipart: Multipart,
        extra_headers: Vec<(String, String)>,
    ) -> Result<Response, Error> {
        let route = Route::new(method, path);
        let mut attempts = 0u32;

        loop {
            // 1. Wait until the limiter lets us through.
            loop {
                let now = self.now_ms();
                let verdict = self.limiter.lock().unwrap_or_else(|e| e.into_inner()).acquire(&route, now);
                match verdict {
                    Verdict::Go => break,
                    Verdict::Wait { retry_after_ms } => {
                        tokio::time::sleep(Duration::from_millis(retry_after_ms)).await;
                    }
                    Verdict::Blocked(reason) => return Err(Error::Blocked(reason)),
                }
            }

            // 2. Send.
            let mut headers = vec![
                ("Authorization".to_owned(), self.auth.clone()),
                ("User-Agent".to_owned(), self.user_agent.clone()),
                (
                    "Content-Type".to_owned(),
                    format!("multipart/form-data; boundary={}", multipart.boundary),
                ),
            ];
            headers.extend(extra_headers.clone());
            let body = multipart.encode();
            let req = Request {
                method: method.to_ascii_uppercase(),
                url: format!("{}{}", self.base, path),
                headers,
                body: Some(body),
                multipart: Some(multipart.clone()),
            };
            let raw: RawResponse = match self.transport.send(req).await {
                Ok(r) => r,
                Err(e) => {
                    // No response: release any probe slot so the route can't wedge.
                    self.report(&route, 0, &RateLimitHeaders::default());
                    return Err(Error::Transport(e));
                }
            };

            // 3. Learn from the response.
            let mut h = RateLimitHeaders::parse(|name| lookup(&raw.headers, name));
            if raw.status == 429 && h.retry_after_ms.is_none() {
                h.retry_after_ms = retry_after_from_body(&raw.body);
            }
            self.report(&route, raw.status, &h);

            // 4. Retry 429s (the limiter now knows how long to wait).
            if raw.status == 429 {
                attempts += 1;
                if attempts >= MAX_ATTEMPTS {
                    return Err(Error::RateLimited);
                }
                continue;
            }

            return Ok(Response { status: raw.status, headers: raw.headers, body: raw.body });
        }
    }

    /// `GET /gateway/bot`: shard count, URL and session-start limits.
    pub async fn gateway_bot(&self) -> Result<GatewayBot, Error> {
        self.request("GET", "/gateway/bot", None).await?.expect_success()?.json()
    }
}

fn lookup<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
}

/// The 429 JSON body carries `retry_after` in seconds.
fn retry_after_from_body(body: &[u8]) -> Option<u64> {
    #[derive(Deserialize)]
    struct Body {
        retry_after: f64,
    }
    let b: Body = serde_json::from_slice(body).ok()?;
    b.retry_after.is_finite().then(|| (b.retry_after.max(0.0) * 1000.0).round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use rift_http::BlockReason;
    use std::collections::VecDeque;
    use std::future::Future;

    struct Fake {
        queue: Mutex<VecDeque<Result<RawResponse, String>>>,
        seen: Mutex<Vec<Request>>,
    }

    impl Fake {
        fn new(items: Vec<Result<RawResponse, String>>) -> Self {
            Self { queue: Mutex::new(items.into()), seen: Mutex::new(Vec::new()) }
        }
    }

    impl Transport for Fake {
        fn send(&self, req: Request) -> impl Future<Output = Result<RawResponse, String>> + Send {
            self.seen.lock().unwrap().push(req);
            let r = self.queue.lock().unwrap().pop_front().unwrap_or_else(|| Err("queue empty".into()));
            async move { r }
        }
    }

    fn resp(status: u16, headers: &[(&str, &str)], body: &str) -> Result<RawResponse, String> {
        Ok(RawResponse {
            status,
            headers: headers.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            body: body.as_bytes().to_vec(),
        })
    }

    fn client(fake: Fake) -> RestClient<Fake> {
        RestClient::with_transport(fake, "tok", user_agent("https://example.com", "0.0.1"), Config::default())
    }

    #[test]
    fn user_agent_has_documented_format() {
        assert_eq!(user_agent("https://x.dev", "1.2.3"), "DiscordBot (https://x.dev, 1.2.3)");
    }

    #[test]
    fn debug_never_prints_the_token() {
        let c = client(Fake::new(vec![]));
        assert!(!format!("{c:?}").contains("tok"));
    }

    #[test]
    fn retry_after_body_is_parsed_in_ms() {
        assert_eq!(retry_after_from_body(br#"{"retry_after": 64.57, "global": false}"#), Some(64_570));
        assert_eq!(retry_after_from_body(b"not json"), None);
    }

    #[tokio::test(start_paused = true)]
    async fn gateway_bot_request_is_well_formed() {
        let body = r#"{"url":"wss://gateway.discord.gg/","shards":9,
            "session_start_limit":{"total":1000,"remaining":999,"reset_after":14400000,"max_concurrency":1}}"#;
        let c = client(Fake::new(vec![resp(200, &[], body)]));
        let gb = c.gateway_bot().await.unwrap();
        assert_eq!(gb.shards, 9);

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/gateway/bot");
        assert!(seen[0].headers.contains(&("Authorization".to_owned(), "Bot tok".to_owned())));
        assert!(seen[0].headers.iter().any(|(k, v)| k == "User-Agent" && v.starts_with("DiscordBot (")));
    }

    #[tokio::test(start_paused = true)]
    async fn retries_a_429_using_the_retry_after_header() {
        let c = client(Fake::new(vec![
            resp(429, &[("retry-after", "0.05"), ("x-ratelimit-scope", "user")], "{}"),
            resp(200, &[], "{}"),
        ]));
        let r = c.request("GET", "/channels/1/messages", None).await.unwrap();
        assert_eq!(r.status, 200);
        assert_eq!(c.transport().seen.lock().unwrap().len(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn retries_a_429_using_the_json_body_when_the_header_is_missing() {
        let c = client(Fake::new(vec![
            resp(429, &[], r#"{"message":"You are being rate limited.","retry_after":0.05,"global":false}"#),
            resp(200, &[], "{}"),
        ]));
        let r = c.request("GET", "/channels/1/messages", None).await.unwrap();
        assert_eq!(r.status, 200);
    }

    #[tokio::test(start_paused = true)]
    async fn gives_up_after_max_attempts() {
        let items = (0..MAX_ATTEMPTS).map(|_| resp(429, &[("retry-after", "0.01")], "{}")).collect();
        let c = client(Fake::new(items));
        let r = c.request("GET", "/channels/1/messages", None).await;
        assert!(matches!(r, Err(Error::RateLimited)));
        assert_eq!(c.transport().seen.lock().unwrap().len(), MAX_ATTEMPTS as usize);
    }

    #[tokio::test(start_paused = true)]
    async fn a_transport_error_does_not_wedge_the_route() {
        let c = client(Fake::new(vec![Err("boom".into()), resp(200, &[], "{}")]));
        let first = c.request("GET", "/channels/1/messages", None).await;
        assert!(matches!(first, Err(Error::Transport(_))));
        // The probe slot was released, so this goes straight through.
        let second = c.request("GET", "/channels/1/messages", None).await.unwrap();
        assert_eq!(second.status, 200);
    }

    #[tokio::test(start_paused = true)]
    async fn circuit_breaker_stops_sending_before_the_ban_line() {
        let cfg = Config { invalid_trip_at: 2, ..Config::default() };
        let fake = Fake::new(vec![resp(403, &[], "{}"), resp(403, &[], "{}"), resp(200, &[], "{}")]);
        let c = RestClient::with_transport(fake, "tok", user_agent("https://example.com", "0.0.1"), cfg);

        assert_eq!(c.request("GET", "/channels/1/messages", None).await.unwrap().status, 403);
        assert_eq!(c.request("GET", "/channels/1/messages", None).await.unwrap().status, 403);
        let third = c.request("GET", "/channels/1/messages", None).await;
        assert!(matches!(third, Err(Error::Blocked(BlockReason::CircuitOpen { .. }))));
        // The third request never reached the network.
        assert_eq!(c.transport().seen.lock().unwrap().len(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn bodies_get_a_content_type() {
        let c = client(Fake::new(vec![resp(200, &[], "{}")]));
        c.request("POST", "/channels/1/messages", Some(br#"{"content":"hi"}"#.to_vec())).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert!(seen[0].headers.contains(&("Content-Type".to_owned(), "application/json".to_owned())));
        assert_eq!(seen[0].body.as_deref(), Some(&br#"{"content":"hi"}"#[..]));
    }
}
