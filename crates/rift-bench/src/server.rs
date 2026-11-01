//! Real loopback (`127.0.0.1:0`) Mock Discord Gateway + REST Server.
//!
//! Speaks the Discord v10 wire protocol:
//! - Opcode 10 `HELLO` on WebSocket connect
//! - Opcode 2 `IDENTIFY` / Opcode 6 `RESUME` handshake
//! - Opcode 11 `HEARTBEAT_ACK` responses to Opcode 1 `HEARTBEAT`
//! - Long-lived `zstd-stream` compression for `READY` and dispatch frames
//! - Clean Close (1000) frame handling

use crate::data::{generate_event_stream, ready_payload};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::io::{self, Write};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;

/// Configuration for [`MockGateway`].
#[derive(Clone, Debug)]
pub struct MockGatewayConfig {
    /// Heartbeat interval in milliseconds sent in `HELLO` (default 41,250 ms).
    pub heartbeat_interval_ms: u64,
    /// Number of dispatch events to stream after `READY` (default 5,000).
    pub event_count: usize,
    /// Number of members in the initial `GUILD_CREATE` event (default 100).
    pub guild_members: usize,
    /// Zstd compression level (default 3, matching Discord).
    pub zstd_level: i32,
    /// Custom raw JSON dispatch events to stream. If `None`, realistic synthetic events are used.
    pub custom_events: Option<Vec<Vec<u8>>>,
    /// Whether to stream uncompressed JSON text frames instead of zstd-stream (default false).
    pub uncompressed: bool,
}

impl Default for MockGatewayConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_ms: 41_250,
            event_count: 5_000,
            guild_members: 100,
            zstd_level: 3,
            custom_events: None,
            uncompressed: false,
        }
    }
}

/// Statistics and counters tracked by [`MockGateway`].
#[derive(Debug, Default)]
pub struct MockGatewayStats {
    /// Total connections accepted.
    pub connections_accepted: AtomicU64,
    /// Total IDENTIFY payloads successfully validated.
    pub identifies_received: AtomicU64,
    /// Total RESUME payloads successfully validated.
    pub resumes_received: AtomicU64,
    /// Total HEARTBEAT payloads received from clients.
    pub heartbeats_received: AtomicU64,
    /// Total HEARTBEAT_ACK payloads sent to clients.
    pub heartbeat_acks_sent: AtomicU64,
    /// Total dispatch frames compressed and streamed.
    pub events_streamed: AtomicU64,
    /// Whether any client sent a clean normal shutdown close frame.
    pub clean_shutdown_seen: AtomicBool,
}

/// Real local loopback Mock Gateway WebSocket Server speaking Discord v10 wire protocol.
pub struct MockGateway {
    addr: SocketAddr,
    config: Arc<MockGatewayConfig>,
    stats: Arc<MockGatewayStats>,
    shutdown_tx: broadcast::Sender<()>,
    server_task: Option<JoinHandle<()>>,
}

impl MockGateway {
    /// Binds to `127.0.0.1:0` and starts serving in the background with the provided configuration.
    pub async fn start(config: MockGatewayConfig) -> io::Result<Self> {
        Self::start_on_port(0, config).await
    }

    /// Binds to `127.0.0.1:{port}` (or ephemeral if port is 0) and starts serving.
    pub async fn start_on_port(port: u16, config: MockGatewayConfig) -> io::Result<Self> {
        let listener = TcpListener::bind(format!("127.0.0.1:{port}")).await?;
        let addr = listener.local_addr()?;
        let (shutdown_tx, _) = broadcast::channel(1);
        let config = Arc::new(config);
        let stats = Arc::new(MockGatewayStats::default());

        let server_config = Arc::clone(&config);
        let server_stats = Arc::clone(&stats);
        let mut shutdown_rx = shutdown_tx.subscribe();

        let server_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => break,
                    accept_res = listener.accept() => {
                        let Ok((stream, _peer)) = accept_res else { break; };
                        server_stats.connections_accepted.fetch_add(1, Ordering::SeqCst);
                        let conn_config = Arc::clone(&server_config);
                        let conn_stats = Arc::clone(&server_stats);
                        let mut conn_shutdown = shutdown_rx.resubscribe();

                        tokio::spawn(async move {
                            tokio::select! {
                                _ = conn_shutdown.recv() => {}
                                res = handle_gateway_connection(stream, conn_config, conn_stats) => {
                                    if let Err(e) = res {
                                        // Transport errors (e.g. client disconnect) are normal in benchmarks.
                                        let _ = e;
                                    }
                                }
                            }
                        });
                    }
                }
            }
        });

        Ok(Self {
            addr,
            config,
            stats,
            shutdown_tx,
            server_task: Some(server_task),
        })
    }

    /// Returns the gateway WebSocket URL (`ws://127.0.0.1:{port}`).
    pub fn url(&self) -> String {
        format!("ws://127.0.0.1:{}", self.addr.port())
    }

    /// Returns the bound socket address.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Access the server's runtime statistics.
    pub fn stats(&self) -> &Arc<MockGatewayStats> {
        &self.stats
    }

    /// Access the server's configuration.
    pub fn config(&self) -> &Arc<MockGatewayConfig> {
        &self.config
    }

    /// Shuts down the mock gateway server.
    pub async fn shutdown(mut self) {
        let _ = self.shutdown_tx.send(());
        if let Some(task) = self.server_task.take() {
            task.abort();
            let _ = task.await;
        }
    }
}

impl Drop for MockGateway {
    fn drop(&mut self) {
        let _ = self.shutdown_tx.send(());
        if let Some(ref task) = self.server_task {
            task.abort();
        }
    }
}

async fn handle_gateway_connection(
    stream: TcpStream,
    config: Arc<MockGatewayConfig>,
    stats: Arc<MockGatewayStats>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut ws = tokio_tungstenite::accept_async(stream).await?;

    // 1. Send Opcode 10 HELLO immediately on connection.
    let hello = json!({
        "op": 10,
        "d": {
            "heartbeat_interval": config.heartbeat_interval_ms
        }
    })
    .to_string();
    ws.send(Message::Text(hello.into())).await?;

    // 2. Read Opcode 2 IDENTIFY or Opcode 6 RESUME.
    let mut session_id = "bench_session_id".to_string();
    let mut authenticated = false;

    while let Some(msg) = ws.next().await {
        let msg = msg?;
        match msg {
            Message::Text(t) => {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&t) {
                    match val.get("op").and_then(|v| v.as_u64()) {
                        Some(2) => {
                            stats.identifies_received.fetch_add(1, Ordering::SeqCst);
                            session_id = format!(
                                "mock_sess_{}",
                                stats.identifies_received.load(Ordering::SeqCst)
                            );
                            authenticated = true;
                            break;
                        }
                        Some(6) => {
                            stats.resumes_received.fetch_add(1, Ordering::SeqCst);
                            authenticated = true;
                            break;
                        }
                        _ => {}
                    }
                }
            }
            Message::Binary(b) => {
                if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&b) {
                    match val.get("op").and_then(|v| v.as_u64()) {
                        Some(2) => {
                            stats.identifies_received.fetch_add(1, Ordering::SeqCst);
                            authenticated = true;
                            break;
                        }
                        Some(6) => {
                            stats.resumes_received.fetch_add(1, Ordering::SeqCst);
                            authenticated = true;
                            break;
                        }
                        _ => {}
                    }
                }
            }
            Message::Close(_) => return Ok(()),
            Message::Ping(p) => {
                ws.send(Message::Pong(p)).await?;
            }
            _ => {}
        }
    }

    if !authenticated {
        return Ok(());
    }

    // 3. Setup streaming with single long-lived zstd encoder flushed after every frame.
    let mut encoder = zstd::stream::write::Encoder::new(Vec::new(), config.zstd_level)?;
    let (mut ws_tx, mut ws_rx) = ws.split();
    let (out_tx, mut out_rx) = mpsc::channel::<Message>(8192);

    let writer_task = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if ws_tx.feed(msg).await.is_err() {
                break;
            }
            let mut count = 1;
            while count < 128 {
                match out_rx.try_recv() {
                    Ok(next_msg) => {
                        if ws_tx.feed(next_msg).await.is_err() {
                            let _ = ws_tx.close().await;
                            return;
                        }
                        count += 1;
                    }
                    Err(_) => break,
                }
            }
            if ws_tx.flush().await.is_err() {
                break;
            }
        }
        let _ = ws_tx.close().await;
    });

    let reader_out_tx = out_tx.clone();
    let reader_stats = Arc::clone(&stats);
    let reader_task = tokio::spawn(async move {
        while let Some(msg_res) = ws_rx.next().await {
            match msg_res {
                Ok(Message::Text(t)) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&t) {
                        if v.get("op").and_then(|op| op.as_u64()) == Some(1) {
                            reader_stats.heartbeats_received.fetch_add(1, Ordering::SeqCst);
                            let ack = json!({ "op": 11 }).to_string();
                            reader_stats.heartbeat_acks_sent.fetch_add(1, Ordering::SeqCst);
                            if reader_out_tx.send(Message::Text(ack.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                Ok(Message::Binary(b)) => {
                    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&b) {
                        if v.get("op").and_then(|op| op.as_u64()) == Some(1) {
                            reader_stats.heartbeats_received.fetch_add(1, Ordering::SeqCst);
                            let ack = json!({ "op": 11 }).to_string();
                            reader_stats.heartbeat_acks_sent.fetch_add(1, Ordering::SeqCst);
                            if reader_out_tx.send(Message::Text(ack.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                Ok(Message::Close(_frame)) => {
                    reader_stats.clean_shutdown_seen.store(true, Ordering::SeqCst);
                    break;
                }
                Ok(Message::Ping(p)) => {
                    if reader_out_tx.send(Message::Pong(p)).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
                _ => {}
            }
        }
    });

    // 4. Pre-generate and pre-compress READY and all dispatch frames BEFORE sending READY!
    let ready_json = ready_payload(&session_id, "ws://127.0.0.1:0");
    let ready_message = if config.uncompressed {
        let text_msg = String::from_utf8(ready_json)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Message::Text(text_msg.into())
    } else {
        encoder.write_all(&ready_json)?;
        encoder.flush()?;
        let ready_compressed = std::mem::take(encoder.get_mut());
        Message::Binary(ready_compressed.into())
    };

    let events = match &config.custom_events {
        Some(evs) => evs.clone(),
        None => generate_event_stream(config.event_count, config.guild_members),
    };

    let mut dispatch_messages = Vec::with_capacity(events.len());
    for event_json in events {
        let msg = if config.uncompressed {
            let text_msg = String::from_utf8(event_json)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            Message::Text(text_msg.into())
        } else {
            encoder.write_all(&event_json)?;
            encoder.flush()?;
            let compressed = std::mem::take(encoder.get_mut());
            Message::Binary(compressed.into())
        };
        dispatch_messages.push(msg);
    }

    // 5. Send READY frame (triggers client's burst timer).
    if out_tx.send(ready_message).await.is_err() {
        return Ok(());
    }

    // 6. Stream pre-computed N dispatch frames.
    for msg in dispatch_messages {
        if out_tx.send(msg).await.is_err() {
            return Ok(());
        }
        stats.events_streamed.fetch_add(1, Ordering::Relaxed);
    }

    // Keep serving heartbeats until client disconnects or shuts down.
    let _ = reader_task.await;
    drop(out_tx);
    let _ = writer_task.await;

    Ok(())
}

/// Lightweight loopback Mock REST Server for Discord v10 REST endpoints.
pub struct MockRestServer {
    addr: SocketAddr,
    shutdown_tx: broadcast::Sender<()>,
    server_task: Option<JoinHandle<()>>,
}

impl MockRestServer {
    /// Starts a mock REST server on `127.0.0.1:0` pointing gateway discovery to `gateway_ws_url`.
    pub async fn start(gateway_ws_url: String) -> io::Result<Self> {
        Self::start_on_port(0, gateway_ws_url).await
    }

    /// Starts a mock REST server on `127.0.0.1:{port}` pointing gateway discovery to `gateway_ws_url`.
    pub async fn start_on_port(port: u16, gateway_ws_url: String) -> io::Result<Self> {
        let listener = TcpListener::bind(format!("127.0.0.1:{port}")).await?;
        let addr = listener.local_addr()?;
        let (shutdown_tx, _) = broadcast::channel(1);
        let mut shutdown_rx = shutdown_tx.subscribe();

        let gateway_url = Arc::new(gateway_ws_url);
        let server_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => break,
                    accept_res = listener.accept() => {
                        let Ok((mut socket, _)) = accept_res else { break; };
                        let gw_url = Arc::clone(&gateway_url);

                        tokio::spawn(async move {
                            let mut buf = [0u8; 4096];
                            let n = match socket.read(&mut buf).await {
                                Ok(n) if n > 0 => n,
                                _ => return,
                            };
                            let req = String::from_utf8_lossy(&buf[..n]);

                            let body = if req.contains("/gateway/bot") {
                                json!({
                                    "url": gw_url.as_str(),
                                    "shards": 1,
                                    "session_start_limit": {
                                        "total": 1000,
                                        "remaining": 1000,
                                        "reset_after": 0,
                                        "max_concurrency": 1
                                    }
                                }).to_string()
                            } else {
                                json!({ "ok": true }).to_string()
                            };

                            let resp = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = socket.write_all(resp.as_bytes()).await;
                        });
                    }
                }
            }
        });

        Ok(Self {
            addr,
            shutdown_tx,
            server_task: Some(server_task),
        })
    }

    /// Returns the REST base URL (`http://127.0.0.1:{port}`).
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.addr.port())
    }

    /// Returns the bound socket address.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Shuts down the mock REST server.
    pub async fn shutdown(mut self) {
        let _ = self.shutdown_tx.send(());
        if let Some(task) = self.server_task.take() {
            task.abort();
            let _ = task.await;
        }
    }
}

/// Bundled Mock Discord Server containing both Gateway and REST servers.
pub struct MockDiscordServer {
    /// Mock Gateway WebSocket server.
    pub gateway: MockGateway,
    /// Mock REST server.
    pub rest: MockRestServer,
}

impl MockDiscordServer {
    /// Starts both Mock Gateway and Mock REST server on ephemeral loopback ports.
    pub async fn start(config: MockGatewayConfig) -> io::Result<Self> {
        Self::start_on_port(0, 0, config).await
    }

    /// Starts both Mock Gateway and Mock REST server on explicit ports (or 0 for ephemeral).
    pub async fn start_on_port(gw_port: u16, rest_port: u16, config: MockGatewayConfig) -> io::Result<Self> {
        let gateway = MockGateway::start_on_port(gw_port, config).await?;
        let rest = MockRestServer::start_on_port(rest_port, gateway.url()).await?;
        Ok(Self { gateway, rest })
    }

    /// Returns the Gateway WebSocket URL.
    pub fn gateway_url(&self) -> String {
        self.gateway.url()
    }

    /// Returns the REST HTTP URL.
    pub fn rest_url(&self) -> String {
        self.rest.url()
    }

    /// Shuts down both servers.
    pub async fn shutdown(self) {
        self.gateway.shutdown().await;
        self.rest.shutdown().await;
    }
}
