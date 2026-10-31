//! Per-connection zstd-stream decompression.
//!
//! Discord's `zstd-stream` keeps one decompression context alive for the whole
//! connection; each websocket message is one gateway message but does not end a
//! zstd frame, so we call the streaming decoder until input is consumed and the
//! output buffer is no longer full.

use std::io;
use zstd::stream::raw::{Decoder, InBuffer, Operation, OutBuffer};

const CHUNK: usize = 64 * 1024;

/// Long-lived streaming zstd decoder preserving decompression context across WebSocket messages.
pub struct StreamDecoder {
    dec: Decoder<'static>,
    out: Vec<u8>,
}

impl StreamDecoder {
    /// Creates a new streaming zstd decoder with a 64 KiB output buffer.
    pub fn new() -> io::Result<Self> {
        Ok(Self { dec: Decoder::new()?, out: Vec::with_capacity(CHUNK) })
    }

    /// Decode one websocket message. The returned slice is valid until the next call.
    pub fn decode(&mut self, data: &[u8]) -> io::Result<&[u8]> {
        self.out.clear();
        let mut input = InBuffer::around(data);
        loop {
            if self.out.capacity() - self.out.len() < CHUNK / 4 {
                self.out.reserve(CHUNK);
            }
            {
                let start = self.out.len();
                let mut output = OutBuffer::around_pos(&mut self.out, start);
                self.dec.run(&mut input, &mut output)?;
            }
            let output_full = self.out.len() == self.out.capacity();
            if input.pos >= data.len() && !output_full {
                break;
            }
        }
        Ok(&self.out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Mimic Discord: one long-lived encoder, flushed after every message.
    fn encode_stream(messages: &[&[u8]]) -> Vec<Vec<u8>> {
        let mut enc = zstd::stream::write::Encoder::new(Vec::new(), 3).unwrap();
        messages
            .iter()
            .map(|m| {
                enc.write_all(m).unwrap();
                enc.flush().unwrap();
                std::mem::take(enc.get_mut())
            })
            .collect()
    }

    #[test]
    fn decodes_flushed_messages_with_shared_context() {
        let msgs: [&[u8]; 3] = [br#"{"op":10,"d":{"heartbeat_interval":41250}}"#, b"{\"op\":11}", b"{\"op\":11}"];
        let wire = encode_stream(&msgs);
        let mut d = StreamDecoder::new().unwrap();
        for (w, m) in wire.iter().zip(msgs.iter()) {
            assert_eq!(d.decode(w).unwrap(), *m);
        }
    }

    #[test]
    fn decodes_payloads_larger_than_one_chunk() {
        let big: Vec<u8> = (0..1_000_000u32).map(|i| (i % 251) as u8).collect();
        let wire = encode_stream(&[&big]);
        let mut d = StreamDecoder::new().unwrap();
        assert_eq!(d.decode(&wire[0]).unwrap(), &big[..]);
    }
}
