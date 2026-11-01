//! The ONLY file that touches tokio-tungstenite. If a new release changes its
//! API, fix it here and nothing else moves.

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, WebSocketConfig};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async_with_config, MaybeTlsStream, WebSocketStream};

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Transport-independent view of a websocket frame.
pub enum Frame {
    Binary(tokio_tungstenite::tungstenite::Bytes),
    Text(String),
    Close(Option<u16>),
    Ignore,
}

pub struct Tx(SplitSink<Ws, Message>);
pub struct Rx(SplitStream<Ws>);

pub async fn connect(url: &str) -> Result<(Tx, Rx), String> {
    let mut config = WebSocketConfig::default();
    config.read_buffer_size = 512 * 1024;
    config.write_buffer_size = 128 * 1024;
    config.max_message_size = None;
    config.max_frame_size = None;
    let (ws, _response) = connect_async_with_config(url, Some(config), true)
        .await
        .map_err(|e| e.to_string())?;
    let (sink, stream) = ws.split();
    Ok((Tx(sink), Rx(stream)))
}

impl Tx {
    pub async fn send_text(&mut self, text: String) -> Result<(), String> {
        self.0.send(Message::Text(text.into())).await.map_err(|e| e.to_string())
    }

    /// Close with 1000, which invalidates the session (bot goes offline).
    pub async fn close(&mut self) {
        let frame = CloseFrame { code: CloseCode::Normal, reason: "".into() };
        let _ = self.0.send(Message::Close(Some(frame))).await;
    }
}

impl Rx {
    /// `None` = stream ended. `Some(Err)` = transport error.
    pub async fn recv(&mut self) -> Option<Result<Frame, String>> {
        match self.0.next().await? {
            Err(e) => Some(Err(e.to_string())),
            Ok(msg) => Some(Ok(match msg {
                Message::Binary(b) => Frame::Binary(b),
                Message::Text(t) => Frame::Text(t.as_str().to_owned()),
                Message::Close(f) => Frame::Close(f.map(|f| u16::from(f.code))),
                _ => Frame::Ignore,
            })),
        }
    }

    /// Drain up to `max` frames synchronously if already buffered in the transport.
    pub fn try_recv_batch(&mut self, out: &mut Vec<Frame>, max: usize) -> Result<(), String> {
        use futures_util::task::noop_waker_ref;
        use std::task::{Context, Poll};
        let mut cx = Context::from_waker(noop_waker_ref());
        while out.len() < max {
            match self.0.poll_next_unpin(&mut cx) {
                Poll::Ready(Some(Ok(msg))) => match msg {
                    Message::Binary(b) => out.push(Frame::Binary(b)),
                    Message::Text(t) => out.push(Frame::Text(t.as_str().to_owned())),
                    Message::Close(f) => {
                        out.push(Frame::Close(f.map(|f| u16::from(f.code))));
                        break;
                    }
                    _ => out.push(Frame::Ignore),
                },
                Poll::Ready(Some(Err(e))) => return Err(e.to_string()),
                Poll::Pending | Poll::Ready(None) => break,
            }
        }
        Ok(())
    }
}
