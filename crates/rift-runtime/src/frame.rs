//! Fast, allocation-free parse of a gateway frame's envelope.
//!
//! The driver needs four things from every frame: `op`, `s`, `t`, and *where
//! the payload `d` is*. It never needs `d` validated (that happens, if at all,
//! when you parse the event). `serde_json` validates every byte of `d` just to
//! find its end, which benchmarks showed costs more than decompressing the
//! frame. This scanner skips `d` structurally (string- and nesting-aware)
//! without building anything.
//!
//! Safety valve: whenever the scanner is unsure (escapes in keys or the event
//! name, duplicate keys, odd numbers, anything unexpected) it returns `None`
//! and [`parse_frame`] falls back to `serde_json`, so the fast path can only
//! ever agree with serde or decline. The tests check that over every key order
//! and a spread of payload shapes.

use serde::Deserialize;
use serde_json::value::RawValue;

/// The envelope of one gateway frame, borrowed from the frame's bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Parsed<'a> {
    pub op: u8,
    pub seq: Option<u64>,
    pub name: Option<&'a str>,
    /// Raw JSON text of `d`. `None` when absent or `null`.
    pub data: Option<&'a str>,
}

/// Parse a frame: fast scanner first, `serde_json` as the fallback.
pub(crate) fn parse_frame(bytes: &[u8]) -> Option<Parsed<'_>> {
    scan(bytes).or_else(|| parse_with_serde(bytes))
}

#[derive(Deserialize)]
struct Envelope<'a> {
    op: u8,
    #[serde(borrow)]
    d: Option<&'a RawValue>,
    s: Option<u64>,
    #[serde(borrow)]
    t: Option<&'a str>,
}

fn parse_with_serde(bytes: &[u8]) -> Option<Parsed<'_>> {
    let e: Envelope = serde_json::from_slice(bytes).ok()?;
    Some(Parsed { op: e.op, seq: e.s, name: e.t, data: e.d.map(RawValue::get) })
}

struct Cursor<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Cursor<'a> {
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.i += 1;
        Some(c)
    }

    fn eat(&mut self, want: u8) -> Option<()> {
        if self.peek()? == want {
            self.i += 1;
            Some(())
        } else {
            None
        }
    }

    #[inline(always)]
    fn ws(&mut self) {
        if let Some(&c) = self.b.get(self.i) {
            if matches!(c, b' ' | b'\n' | b'\r' | b'\t') {
                self.i += 1;
                while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
                    self.i += 1;
                }
            }
        }
    }

    /// Consume a literal `null` if present.
    #[inline(always)]
    fn null(&mut self) -> bool {
        if self.b.get(self.i..).is_some_and(|r| r.starts_with(b"null")) {
            self.i += 4;
            true
        } else {
            false
        }
    }

    /// A string with no escapes. Declines (None) on a backslash or control byte.
    #[inline(always)]
    fn plain_string(&mut self) -> Option<&'a [u8]> {
        self.eat(b'"')?;
        let start = self.i;
        if let Some(idx) = memchr::memchr2(b'"', b'\\', &self.b[self.i..]) {
            if self.b[self.i + idx] == b'"' {
                let s = &self.b[start..self.i + idx];
                if s.iter().any(|&c| c < 0x20) {
                    return None;
                }
                self.i += idx + 1;
                return Some(s);
            }
        }
        None
    }

    /// A non-negative integer: no sign, fraction, exponent or leading zeros.
    #[inline(always)]
    fn uint(&mut self) -> Option<u64> {
        let start = self.i;
        let mut v: u64 = 0;
        while let Some(&d @ b'0'..=b'9') = self.b.get(self.i) {
            v = v.checked_mul(10)?.checked_add(u64::from(d - b'0'))?;
            self.i += 1;
        }
        if self.i == start || (self.i - start > 1 && self.b[start] == b'0') {
            return None;
        }
        match self.peek() {
            Some(b'.' | b'e' | b'E' | b'-' | b'+') => None,
            _ => Some(v),
        }
    }

    /// Skip the body of a string (after the opening quote).
    fn skip_string_body(&mut self) -> Option<()> {
        while self.i < self.b.len() {
            let idx = memchr::memchr2(b'"', b'\\', &self.b[self.i..])?;
            if self.b[self.i + idx] == b'"' {
                self.i += idx + 1;
                return Some(());
            }
            // Skip backslash and the escaped byte
            self.i += idx + 2;
        }
        None
    }

    /// Skip one JSON value structurally. Does not validate it.
    fn skip_value(&mut self) -> Option<()> {
        match self.peek()? {
            b'{' | b'[' => {
                let mut depth = 0usize;
                loop {
                    match self.next()? {
                        b'{' | b'[' => depth += 1,
                        b'}' | b']' => {
                            depth -= 1;
                            if depth == 0 {
                                return Some(());
                            }
                        }
                        b'"' => self.skip_string_body()?,
                        _ => {}
                    }
                }
            }
            b'"' => {
                self.i += 1;
                self.skip_string_body()
            }
            _ => {
                // number / true / false / null: run to the next delimiter
                let start = self.i;
                while let Some(c) = self.peek() {
                    if matches!(c, b',' | b'}' | b']' | b' ' | b'\n' | b'\r' | b'\t') {
                        break;
                    }
                    self.i += 1;
                }
                (self.i > start).then_some(())
            }
        }
    }
}

/// The fast path. `None` means "not sure", not "invalid".
fn scan(b: &[u8]) -> Option<Parsed<'_>> {
    let s = std::str::from_utf8(b).ok()?;
    let mut c = Cursor { b, i: 0 };
    c.ws();
    c.eat(b'{')?;

    let mut op: Option<u8> = None;
    let mut seq: Option<u64> = None;
    let mut name: Option<&str> = None;
    let mut data: Option<&str> = None;
    let (mut seen_op, mut seen_s, mut seen_t, mut seen_d) = (false, false, false, false);

    loop {
        c.ws();
        let key = c.plain_string()?;
        c.ws();
        c.eat(b':')?;
        c.ws();
        match key {
            b"op" => {
                if std::mem::replace(&mut seen_op, true) {
                    return None;
                }
                op = Some(u8::try_from(c.uint()?).ok()?);
            }
            b"s" => {
                if std::mem::replace(&mut seen_s, true) {
                    return None;
                }
                if !c.null() {
                    seq = Some(c.uint()?);
                }
            }
            b"t" => {
                if std::mem::replace(&mut seen_t, true) {
                    return None;
                }
                if !c.null() {
                    let raw = c.plain_string()?;
                    name = Some(std::str::from_utf8(raw).ok()?);
                }
            }
            b"d" => {
                if std::mem::replace(&mut seen_d, true) {
                    return None;
                }
                let start = c.i;
                c.skip_value()?;
                if &b[start..c.i] != b"null" {
                    data = s.get(start..c.i);
                }
            }
            _ => c.skip_value()?,
        }
        c.ws();
        match c.next()? {
            b',' => continue,
            b'}' => break,
            _ => return None,
        }
    }

    c.ws();
    if c.i != b.len() {
        return None;
    }
    Some(Parsed { op: op?, seq, name, data })
}

#[cfg(test)]
mod tests {
    use super::*;

    type View = (u8, Option<u64>, Option<String>, Option<String>);

    fn view(p: Parsed<'_>) -> View {
        (p.op, p.seq, p.name.map(String::from), p.data.map(String::from))
    }

    fn fast(raw: &str) -> Option<View> {
        scan(raw.as_bytes()).map(view)
    }

    fn serde_only(raw: &str) -> Option<View> {
        parse_with_serde(raw.as_bytes()).map(view)
    }

    fn full(raw: &str) -> Option<View> {
        parse_frame(raw.as_bytes()).map(view)
    }

    #[test]
    fn common_shapes_take_the_fast_path_and_match_serde() {
        let shapes = [
            r#"{"t":"MESSAGE_CREATE","s":42,"op":0,"d":{"id":"1","content":"hi"}}"#,
            r#"{"op":0,"d":{"a":[1,2,{"b":"}]\"x"}]},"s":7,"t":"X"}"#,
            r#"{"op":10,"d":{"heartbeat_interval":41250}}"#,
            r#"{ "op" : 11 , "d" : null }"#,
            r#"{"op":9,"d":true}"#,
            r#"{"op":9,"d":false}"#,
            r#"{"op":0,"s":null,"t":null,"d":[]}"#,
            r#"{"d":"str with \\ backslash \" quote","op":0,"s":1,"t":"T"}"#,
            r#"{"op":0,"s":1,"t":"T","d":{"emoji":"🔥","n":"ñ"}}"#,
            r#"{"op":0,"s":1,"t":"T","d":12.5e3}"#,
            r#"{"extra":{"x":[1,2,3]},"op":0,"s":1,"t":"T","d":{}}"#,
            "{\n  \"op\": 0,\n  \"d\": {}\n}",
        ];
        for raw in shapes {
            let f = fast(raw);
            assert!(f.is_some(), "fast path declined: {raw}");
            assert_eq!(f, serde_only(raw), "{raw}");
        }
        // `null` payloads are None, as with serde.
        assert_eq!(fast(r#"{"op":11,"d":null}"#).unwrap().3, None);
        // The payload is the exact source text.
        assert_eq!(
            fast(r#"{"t":"X","s":1,"op":0,"d":{ "a" : [ 1 ] }}"#).unwrap().3.as_deref(),
            Some(r#"{ "a" : [ 1 ] }"#)
        );
    }

    #[test]
    fn braces_quotes_and_escapes_inside_strings_do_not_confuse_the_scanner() {
        let tricky = r#"{"op":0,"s":3,"t":"T","d":{"s":"}{][ \" ,:","t":"\\","u":"\u00e9 \"}\""}}"#;
        let f = fast(tricky).expect("fast path should handle this");
        assert_eq!(f, serde_only(tricky).unwrap());
        // The payload is exactly the inner object, braces-in-strings and all.
        assert_eq!(
            f.3.as_deref(),
            Some(r#"{"s":"}{][ \" ,:","t":"\\","u":"\u00e9 \"}\""}"#)
        );
    }

    #[test]
    fn fast_and_serde_agree_for_every_key_order_and_payload_shape() {
        let datas = [
            r#"{}"#,
            r#"[]"#,
            r#"{"a":1}"#,
            r#"{"s":"}{][ \" ,:"}"#,
            r#"{"q":"say \"hi\""}"#,
            r#"[1,[2,[3,{"x":null}]]]"#,
            r#""plain""#,
            r#""with \\ and \" quotes""#,
            r#"123"#,
            r#"-4.5e-3"#,
            r#"true"#,
            r#"{"u":"\u00e9 🔥"}"#,
        ];
        let mut checked = 0;
        for d in datas {
            let items = [
                r#""op":0"#.to_string(),
                r#""s":5"#.to_string(),
                r#""t":"EVT""#.to_string(),
                format!(r#""d":{d}"#),
            ];
            for a in 0..4 {
                for b in 0..4 {
                    for c in 0..4 {
                        for e in 0..4 {
                            let order = [a, b, c, e];
                            let distinct = (0..4).all(|k| order.iter().filter(|&&x| x == k).count() == 1);
                            if !distinct {
                                continue;
                            }
                            let parts: Vec<&str> = order.iter().map(|&k| items[k].as_str()).collect();
                            for raw in [format!("{{{}}}", parts.join(",")), format!(" {{ {} }} ", parts.join(" , "))] {
                                let f = fast(&raw);
                                assert!(f.is_some(), "fast path declined: {raw}");
                                assert_eq!(f, serde_only(&raw), "{raw}");
                                checked += 1;
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(checked, 12 * 24 * 2);
    }

    #[test]
    fn declined_or_invalid_inputs_fall_back_and_never_disagree_with_serde() {
        let declined = [
            r#"{"op":0,"s":1,"t":"A\u0042","d":{}}"#, // escape in the event name
            r#"{"op":0,"op":1}"#,                     // duplicate key
            r#"{"d":{}}"#,                            // no op
            r#"{"op":300}"#,                          // op doesn't fit u8
            r#"{"op":1.5}"#,                          // not an integer
            r#"{"op":01}"#,                           // leading zero
            r#"{"op":1} trailing"#,
            r#"{"op":1,"d":{"a":"#, // truncated
            r#"{"op":0,}"#,         // trailing comma
            "",
            "not json",
            "[]",
        ];
        for raw in declined {
            assert!(fast(raw).is_none(), "fast path should decline: {raw:?}");
            assert_eq!(full(raw), serde_only(raw), "{raw:?}");
        }
    }
}
