//! The ONLY file that touches tokio-tungstenite. If a new release changes its
//! API, fix it here and nothing else moves.

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

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
    let (ws, _response) = connect_async(url).await.map_err(|e| e.to_string())?;
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

    /// Try to receive the next frame synchronously if already buffered in the transport.
    pub fn try_recv(&mut self) -> Option<Result<Frame, String>> {
        use futures_util::task::noop_waker_ref;
        use std::task::{Context, Poll};
        let mut cx = Context::from_waker(noop_waker_ref());
        match self.0.poll_next_unpin(&mut cx) {
            Poll::Ready(Some(Err(e))) => Some(Err(e.to_string())),
            Poll::Ready(Some(Ok(msg))) => Some(Ok(match msg {
                Message::Binary(b) => Frame::Binary(b),
                Message::Text(t) => Frame::Text(t.as_str().to_owned()),
                Message::Close(f) => Frame::Close(f.map(|f| u16::from(f.code))),
                _ => Frame::Ignore,
            })),
            _ => None,
        }
    }
}
