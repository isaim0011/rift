//! The ONLY file that touches reqwest.

use std::future::Future;

/// A single part in a multipart/form-data request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    /// Form field name, e.g. `"payload_json"` or `"files[0]"`.
    pub name: String,
    /// Optional filename in Content-Disposition header.
    pub filename: Option<String>,
    /// Optional MIME content-type, e.g. `"application/json"` or `"image/png"`.
    pub content_type: Option<String>,
    /// Body bytes of this part.
    pub data: Vec<u8>,
}

impl Part {
    /// Create a new part with name and data.
    pub fn new(name: impl Into<String>, data: impl Into<Vec<u8>>) -> Self {
        Self {
            name: name.into(),
            filename: None,
            content_type: None,
            data: data.into(),
        }
    }

    /// Set the filename for this part.
    pub fn filename(mut self, name: impl Into<String>) -> Self {
        self.filename = Some(name.into());
        self
    }

    /// Set the content-type for this part.
    pub fn content_type(mut self, ct: impl Into<String>) -> Self {
        self.content_type = Some(ct.into());
        self
    }
}

/// A multipart/form-data payload containing parts and a boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Multipart {
    /// Boundary string.
    pub boundary: String,
    /// Parts of this multipart body.
    pub parts: Vec<Part>,
}

impl Multipart {
    /// Create a new multipart payload with the given boundary and parts.
    pub fn new(boundary: impl Into<String>, parts: Vec<Part>) -> Self {
        Self {
            boundary: boundary.into(),
            parts,
        }
    }

    /// Encode this multipart payload into wire bytes following RFC 7578.
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        for part in &self.parts {
            buf.extend_from_slice(b"--");
            buf.extend_from_slice(self.boundary.as_bytes());
            buf.extend_from_slice(b"\r\n");

            buf.extend_from_slice(b"Content-Disposition: form-data; name=\"");
            buf.extend_from_slice(part.name.as_bytes());
            buf.extend_from_slice(b"\"");
            if let Some(filename) = &part.filename {
                buf.extend_from_slice(b"; filename=\"");
                buf.extend_from_slice(filename.as_bytes());
                buf.extend_from_slice(b"\"");
            }
            buf.extend_from_slice(b"\r\n");

            if let Some(ct) = &part.content_type {
                buf.extend_from_slice(b"Content-Type: ");
                buf.extend_from_slice(ct.as_bytes());
                buf.extend_from_slice(b"\r\n");
            }

            buf.extend_from_slice(b"\r\n");
            buf.extend_from_slice(&part.data);
            buf.extend_from_slice(b"\r\n");
        }

        buf.extend_from_slice(b"--");
        buf.extend_from_slice(self.boundary.as_bytes());
        buf.extend_from_slice(b"--\r\n");

        buf
    }
}

/// An outgoing request.
#[derive(Clone, Debug)]
pub struct Request {
    /// Upper-case HTTP method.
    pub method: String,
    /// Absolute URL.
    pub url: String,
    /// Header name/value pairs.
    pub headers: Vec<(String, String)>,
    /// Optional body.
    pub body: Option<Vec<u8>>,
    /// Optional multipart form parts (if this request is multipart).
    pub multipart: Option<Multipart>,
}

/// A completed response.
#[derive(Clone, Debug)]
pub struct RawResponse {
    /// HTTP status.
    pub status: u16,
    /// Header name/value pairs (names as sent by the server).
    pub headers: Vec<(String, String)>,
    /// Body bytes.
    pub body: Vec<u8>,
}

/// Something that can send a request. Implement this to swap HTTP stacks or to
/// fake the network in tests.
pub trait Transport: Send + Sync {
    /// Send `req`. `Err` means no response was received.
    fn send(&self, req: Request) -> impl Future<Output = Result<RawResponse, String>> + Send;
}

/// The default transport, backed by reqwest.
pub struct ReqwestTransport {
    client: reqwest::Client,
}

impl ReqwestTransport {
    /// Build a client.
    pub fn new() -> Result<Self, String> {
        reqwest::Client::builder()
            .build()
            .map(|client| Self { client })
            .map_err(|e| e.to_string())
    }
}

impl Transport for ReqwestTransport {
    async fn send(&self, req: Request) -> Result<RawResponse, String> {
        let method = reqwest::Method::from_bytes(req.method.as_bytes()).map_err(|e| e.to_string())?;
        let mut rb = self.client.request(method, req.url.as_str());
        for (k, v) in &req.headers {
            rb = rb.header(k.as_str(), v.as_str());
        }
        let body = req.body.or_else(|| req.multipart.as_ref().map(|mp| mp.encode()));
        if let Some(body) = body {
            rb = rb.body(body);
        }
        // `without_url`: interaction tokens live in the URL path; keep them out of error text.
        let resp = rb.send().await.map_err(|e| e.without_url().to_string())?;
        let status = resp.status().as_u16();
        let headers = resp
            .headers()
            .iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.as_str().to_owned(), v.to_owned())))
            .collect();
        let body = resp.bytes().await.map_err(|e| e.without_url().to_string())?.to_vec();
        Ok(RawResponse { status, headers, body })
    }
}
