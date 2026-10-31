//! Test-only fake transport.

use crate::client::{user_agent, RestClient};
use crate::transport::{RawResponse, Request, Transport};
use rift_http::Config;
use std::collections::VecDeque;
use std::future::Future;
use std::sync::Mutex;

pub(crate) struct Fake {
    queue: Mutex<VecDeque<Result<RawResponse, String>>>,
    pub(crate) seen: Mutex<Vec<Request>>,
}

impl Transport for Fake {
    fn send(&self, req: Request) -> impl Future<Output = Result<RawResponse, String>> + Send {
        self.seen.lock().unwrap().push(req);
        let r = self.queue.lock().unwrap().pop_front().unwrap_or_else(|| Err("queue empty".into()));
        async move { r }
    }
}

pub(crate) fn resp(status: u16, body: &str) -> Result<RawResponse, String> {
    Ok(RawResponse { status, headers: Vec::new(), body: body.as_bytes().to_vec() })
}

pub(crate) fn client(items: Vec<Result<RawResponse, String>>) -> RestClient<Fake> {
    let fake = Fake { queue: Mutex::new(items.into()), seen: Mutex::new(Vec::new()) };
    RestClient::with_transport(fake, "tok", user_agent("https://example.com", "0.0.1"), Config::default())
}
