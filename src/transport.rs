//! Adaptive transport layer that auto-detects between JSONL and LSP-style framing.
//!
//! Different MCP clients use different message framing:
//! - **JSONL**: Messages delimited by newlines `{"jsonrpc":"2.0",...}\n`
//! - **LSP-style**: `Content-Length: N\r\n\r\n{"jsonrpc":"2.0",...}`
//!
//! This module provides an adaptive codec that detects the format from incoming
//! messages and responds in the same format.

use std::marker::PhantomData;
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use serde::{de::DeserializeOwned, Serialize};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{Mutex, RwLock};
use tokio_util::bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder, FramedRead, FramedWrite};

/// Detected message framing format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FramingFormat {
    /// Newline-delimited JSON (default MCP stdio format)
    #[default]
    JsonLines,
    /// LSP-style Content-Length headers
    Lsp,
}

/// Adaptive codec that handles both JSONL and LSP-style message framing.
///
/// The codec auto-detects the incoming format and responds using the same format.
/// When `shared_format` is provided, the detected format is stored there for sharing
/// between reader and writer codecs.
#[derive(Debug)]
pub struct AdaptiveCodec<T> {
    _marker: PhantomData<fn() -> T>,
    /// Detected format for incoming messages (also used for outgoing)
    detected_format: Option<FramingFormat>,
    /// Shared format state between reader and writer (if provided)
    shared_format: Option<Arc<RwLock<Option<FramingFormat>>>>,
    /// Buffer state for JSONL parsing
    next_index: usize,
    max_length: usize,
    is_discarding: bool,
    /// Buffer state for LSP parsing
    expected_content_length: Option<usize>,
}

impl<T> Default for AdaptiveCodec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> AdaptiveCodec<T> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
            detected_format: None,
            shared_format: None,
            next_index: 0,
            max_length: usize::MAX,
            is_discarding: false,
            expected_content_length: None,
        }
    }

    /// Create a codec with shared format state
    pub fn with_shared_format(shared: Arc<RwLock<Option<FramingFormat>>>) -> Self {
        Self {
            _marker: PhantomData,
            detected_format: None,
            shared_format: Some(shared),
            next_index: 0,
            max_length: usize::MAX,
            is_discarding: false,
            expected_content_length: None,
        }
    }

    pub fn detected_format(&self) -> Option<FramingFormat> {
        self.detected_format
    }

    /// Detect format by peeking at buffer contents
    fn detect_format(buf: &[u8]) -> Option<FramingFormat> {
        // Skip any leading whitespace
        let trimmed = buf.iter().position(|&b| !b.is_ascii_whitespace());
        let start = trimmed.unwrap_or(0);

        if buf.len() <= start {
            return None; // Need more data
        }

        let first_byte = buf[start];

        // LSP-style starts with 'C' from "Content-Length:"
        if first_byte == b'C' {
            if buf[start..].starts_with(b"Content-Length:") {
                return Some(FramingFormat::Lsp);
            }
            // Could be partial "Content-Length", need more data
            if buf.len() - start < 15 {
                return None;
            }
        }

        // JSONL starts with '{'
        if first_byte == b'{' {
            return Some(FramingFormat::JsonLines);
        }

        // Unknown format, default to JSONL
        Some(FramingFormat::JsonLines)
    }

    /// Parse LSP-style headers, returns content length if complete
    fn parse_lsp_headers(buf: &[u8]) -> Option<(usize, usize)> {
        // Look for header/body separator: \r\n\r\n
        let separator = b"\r\n\r\n";
        let sep_pos = buf.windows(separator.len()).position(|w| w == separator)?;

        let header_bytes = &buf[..sep_pos];
        let header_str = std::str::from_utf8(header_bytes).ok()?;

        // Parse Content-Length from headers
        for line in header_str.lines() {
            let line = line.trim();
            if let Some(value) = line.strip_prefix("Content-Length:") {
                let length: usize = value.trim().parse().ok()?;
                let body_start = sep_pos + separator.len();
                return Some((length, body_start));
            }
        }

        None
    }
}

fn without_carriage_return(s: &[u8]) -> &[u8] {
    if let Some(&b'\r') = s.last() {
        &s[..s.len() - 1]
    } else {
        s
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AdaptiveCodecError {
    #[error("max line length exceeded")]
    MaxLineLengthExceeded,
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<AdaptiveCodecError> for std::io::Error {
    fn from(value: AdaptiveCodecError) -> Self {
        match value {
            AdaptiveCodecError::Io(e) => e,
            other => std::io::Error::new(std::io::ErrorKind::InvalidData, other),
        }
    }
}

impl<T: DeserializeOwned> Decoder for AdaptiveCodec<T> {
    type Item = T;
    type Error = AdaptiveCodecError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if buf.is_empty() {
            return Ok(None);
        }

        // Auto-detect format if not yet determined
        if self.detected_format.is_none() {
            match Self::detect_format(buf) {
                Some(fmt) => {
                    self.detected_format = Some(fmt);
                    eprintln!("[transport] Detected framing format: {:?}", fmt);
                    // Store in shared state if available
                    if let Some(ref shared) = self.shared_format {
                        // Use try_write to avoid blocking - if we can't get the lock,
                        // the encoder will get the format on next attempt
                        if let Ok(mut guard) = shared.try_write() {
                            *guard = Some(fmt);
                        }
                    }
                }
                None => return Ok(None), // Need more data to detect
            }
        }

        match self.detected_format.unwrap() {
            FramingFormat::Lsp => self.decode_lsp(buf),
            FramingFormat::JsonLines => self.decode_jsonl(buf),
        }
    }
}

impl<T: DeserializeOwned> AdaptiveCodec<T> {
    fn decode_lsp(&mut self, buf: &mut BytesMut) -> Result<Option<T>, AdaptiveCodecError> {
        // If we already know content length, try to read body
        if let Some(content_length) = self.expected_content_length {
            if buf.len() >= content_length {
                let body = buf.split_to(content_length);
                self.expected_content_length = None;
                let item: T = serde_json::from_slice(&body)?;
                return Ok(Some(item));
            }
            return Ok(None); // Need more data
        }

        // Parse headers to get content length
        match Self::parse_lsp_headers(buf) {
            Some((content_length, body_start)) => {
                // Consume headers
                buf.advance(body_start);
                self.expected_content_length = Some(content_length);
                // Recursively try to read body
                self.decode_lsp(buf)
            }
            None => Ok(None), // Need more data for headers
        }
    }

    fn decode_jsonl(&mut self, buf: &mut BytesMut) -> Result<Option<T>, AdaptiveCodecError> {
        loop {
            let read_to = std::cmp::min(self.max_length.saturating_add(1), buf.len());

            let newline_offset = buf[self.next_index..read_to]
                .iter()
                .position(|b| *b == b'\n');

            match (self.is_discarding, newline_offset) {
                (true, Some(offset)) => {
                    buf.advance(offset + self.next_index + 1);
                    self.is_discarding = false;
                    self.next_index = 0;
                }
                (true, None) => {
                    buf.advance(read_to);
                    self.next_index = 0;
                    if buf.is_empty() {
                        return Ok(None);
                    }
                }
                (false, Some(offset)) => {
                    let newline_index = offset + self.next_index;
                    self.next_index = 0;
                    let line = buf.split_to(newline_index + 1);
                    let line = &line[..line.len() - 1];
                    let line = without_carriage_return(line);

                    // Skip empty lines
                    if line.is_empty() {
                        continue;
                    }

                    let item: T = serde_json::from_slice(line)?;
                    return Ok(Some(item));
                }
                (false, None) if buf.len() > self.max_length => {
                    self.is_discarding = true;
                    return Err(AdaptiveCodecError::MaxLineLengthExceeded);
                }
                (false, None) => {
                    self.next_index = read_to;
                    return Ok(None);
                }
            }
        }
    }
}

impl<T: Serialize> Encoder<T> for AdaptiveCodec<T> {
    type Error = AdaptiveCodecError;

    fn encode(&mut self, item: T, buf: &mut BytesMut) -> Result<(), Self::Error> {
        // First check local detected format, then shared format, then default to JSONL
        let format = self
            .detected_format
            .or_else(|| {
                self.shared_format
                    .as_ref()
                    .and_then(|shared| shared.try_read().ok().and_then(|guard| *guard))
            })
            .unwrap_or(FramingFormat::JsonLines);

        match format {
            FramingFormat::Lsp => {
                // Serialize to temp buffer first to get length
                let json = serde_json::to_vec(&item)?;
                let header = format!("Content-Length: {}\r\n\r\n", json.len());
                buf.put_slice(header.as_bytes());
                buf.put_slice(&json);
            }
            FramingFormat::JsonLines => {
                serde_json::to_writer(buf.writer(), &item)?;
                buf.put_u8(b'\n');
            }
        }

        Ok(())
    }
}

/// Type alias for the framed writer with adaptive codec
type AdaptiveWriter<W, T> = FramedWrite<W, AdaptiveCodec<T>>;

/// Adaptive transport that wraps AsyncRead/AsyncWrite with format auto-detection.
///
/// This transport uses `AdaptiveCodec` to handle both JSONL and LSP-style framing,
/// auto-detecting the format from incoming messages.
pub struct AdaptiveTransport<R, W, Tx, Rx>
where
    R: AsyncRead,
    W: AsyncWrite,
{
    read: FramedRead<R, AdaptiveCodec<Rx>>,
    write: Arc<Mutex<Option<AdaptiveWriter<W, Tx>>>>,
}

impl<R, W, Tx, Rx> AdaptiveTransport<R, W, Tx, Rx>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    pub fn new(read: R, write: W) -> Self {
        // Create shared format state so reader and writer use the same detected format
        let shared_format = Arc::new(RwLock::new(None));
        let read = FramedRead::new(
            read,
            AdaptiveCodec::<Rx>::with_shared_format(shared_format.clone()),
        );
        let write = Arc::new(Mutex::new(Some(FramedWrite::new(
            write,
            AdaptiveCodec::<Tx>::with_shared_format(shared_format),
        ))));
        Self { read, write }
    }
}

use rmcp::service::{RxJsonRpcMessage, ServiceRole, TxJsonRpcMessage};
use rmcp::transport::{IntoTransport, Transport};

pub enum AdaptiveTransportAdapter {}

impl<Role, R, W> Transport<Role>
    for AdaptiveTransport<R, W, TxJsonRpcMessage<Role>, RxJsonRpcMessage<Role>>
where
    Role: ServiceRole,
    R: AsyncRead + Send + Unpin,
    W: AsyncWrite + Send + Unpin + 'static,
    RxJsonRpcMessage<Role>: DeserializeOwned,
    TxJsonRpcMessage<Role>: Serialize,
{
    type Error = std::io::Error;

    fn send(
        &mut self,
        item: TxJsonRpcMessage<Role>,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send + 'static {
        let lock = self.write.clone();
        async move {
            let mut guard = lock.lock().await;
            if let Some(ref mut writer) = *guard {
                writer.send(item).await.map_err(Into::into)
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotConnected,
                    "Transport is closed",
                ))
            }
        }
    }

    fn receive(
        &mut self,
    ) -> impl std::future::Future<Output = Option<RxJsonRpcMessage<Role>>> + Send {
        let next = self.read.next();
        async {
            next.await.and_then(|result| {
                result
                    .inspect_err(|e| {
                        eprintln!("[transport] Error reading message: {}", e);
                    })
                    .ok()
            })
        }
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        let mut guard = self.write.lock().await;
        drop(guard.take());
        Ok(())
    }
}

/// Wrapper to force using AdaptiveTransport instead of rmcp's default.
pub struct AdaptiveStdio {
    stdin: tokio::io::Stdin,
    stdout: tokio::io::Stdout,
}

impl AdaptiveStdio {
    pub fn new() -> Self {
        Self {
            stdin: tokio::io::stdin(),
            stdout: tokio::io::stdout(),
        }
    }
}

impl Default for AdaptiveStdio {
    fn default() -> Self {
        Self::new()
    }
}

impl<Role> IntoTransport<Role, std::io::Error, AdaptiveTransportAdapter> for AdaptiveStdio
where
    Role: ServiceRole,
    RxJsonRpcMessage<Role>: DeserializeOwned,
    TxJsonRpcMessage<Role>: Serialize,
{
    fn into_transport(self) -> impl Transport<Role, Error = std::io::Error> + 'static {
        AdaptiveTransport::<
            tokio::io::Stdin,
            tokio::io::Stdout,
            TxJsonRpcMessage<Role>,
            RxJsonRpcMessage<Role>,
        >::new(self.stdin, self.stdout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_jsonl() {
        let buf = br#"{"jsonrpc":"2.0","method":"initialize"}"#;
        assert_eq!(
            AdaptiveCodec::<()>::detect_format(buf),
            Some(FramingFormat::JsonLines)
        );
    }

    #[test]
    fn test_detect_lsp() {
        let buf = b"Content-Length: 42\r\n\r\n{\"jsonrpc\":\"2.0\"}";
        assert_eq!(
            AdaptiveCodec::<()>::detect_format(buf),
            Some(FramingFormat::Lsp)
        );
    }

    #[test]
    fn test_detect_with_whitespace() {
        let buf = b"  \n  {\"jsonrpc\":\"2.0\"}";
        assert_eq!(
            AdaptiveCodec::<()>::detect_format(buf),
            Some(FramingFormat::JsonLines)
        );
    }

    #[test]
    fn test_parse_lsp_headers() {
        let buf = b"Content-Length: 18\r\n\r\n{\"jsonrpc\":\"2.0\"}";
        let result = AdaptiveCodec::<()>::parse_lsp_headers(buf);
        assert_eq!(result, Some((18, 22))); // 18 bytes content, body starts at 22
    }

    #[test]
    fn test_parse_lsp_headers_with_content_type() {
        let buf =
            b"Content-Length: 18\r\nContent-Type: application/json\r\n\r\n{\"jsonrpc\":\"2.0\"}";
        let result = AdaptiveCodec::<()>::parse_lsp_headers(buf);
        assert!(result.is_some());
        let (len, _) = result.unwrap();
        assert_eq!(len, 18);
    }

    #[test]
    fn test_decode_jsonl() {
        let mut codec = AdaptiveCodec::<serde_json::Value>::new();
        let mut buf = BytesMut::from(
            &br#"{"jsonrpc":"2.0","id":1}
"#[..],
        );

        let result = codec.decode(&mut buf).unwrap();
        assert!(result.is_some());
        assert_eq!(codec.detected_format(), Some(FramingFormat::JsonLines));

        let msg = result.unwrap();
        assert_eq!(msg["jsonrpc"], "2.0");
        assert_eq!(msg["id"], 1);
    }

    #[test]
    fn test_decode_lsp() {
        let mut codec = AdaptiveCodec::<serde_json::Value>::new();
        let json = r#"{"jsonrpc":"2.0","id":1}"#;
        let msg = format!("Content-Length: {}\r\n\r\n{}", json.len(), json);
        let mut buf = BytesMut::from(msg.as_bytes());

        let result = codec.decode(&mut buf).unwrap();
        assert!(result.is_some());
        assert_eq!(codec.detected_format(), Some(FramingFormat::Lsp));

        let msg = result.unwrap();
        assert_eq!(msg["jsonrpc"], "2.0");
        assert_eq!(msg["id"], 1);
    }

    #[test]
    fn test_encode_jsonl() {
        let mut codec = AdaptiveCodec::<serde_json::Value>::new();
        codec.detected_format = Some(FramingFormat::JsonLines);

        let msg = serde_json::json!({"jsonrpc": "2.0", "id": 1});
        let mut buf = BytesMut::new();
        codec.encode(msg, &mut buf).unwrap();

        let output = String::from_utf8_lossy(&buf);
        assert!(output.ends_with('\n'));
        assert!(output.starts_with('{'));
    }

    #[test]
    fn test_encode_lsp() {
        let mut codec = AdaptiveCodec::<serde_json::Value>::new();
        codec.detected_format = Some(FramingFormat::Lsp);

        let msg = serde_json::json!({"jsonrpc": "2.0", "id": 1});
        let mut buf = BytesMut::new();
        codec.encode(msg, &mut buf).unwrap();

        let output = String::from_utf8_lossy(&buf);
        assert!(output.starts_with("Content-Length:"));
        assert!(output.contains("\r\n\r\n"));
    }

    #[test]
    fn test_multiple_jsonl_messages() {
        let mut codec = AdaptiveCodec::<serde_json::Value>::new();
        let mut buf = BytesMut::from(
            &br#"{"jsonrpc":"2.0","id":1}
{"jsonrpc":"2.0","id":2}
"#[..],
        );

        let msg1 = codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(msg1["id"], 1);

        let msg2 = codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(msg2["id"], 2);
    }

    #[test]
    fn test_multiple_lsp_messages() {
        let mut codec = AdaptiveCodec::<serde_json::Value>::new();

        let json1 = r#"{"jsonrpc":"2.0","id":1}"#;
        let json2 = r#"{"jsonrpc":"2.0","id":2}"#;
        let msg = format!(
            "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
            json1.len(),
            json1,
            json2.len(),
            json2
        );
        let mut buf = BytesMut::from(msg.as_bytes());

        let msg1 = codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(msg1["id"], 1);

        let msg2 = codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(msg2["id"], 2);
    }
}
