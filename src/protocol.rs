//! Protocol implementation for JSON lines communication

use crate::error::{Error, Result};
use crate::messages::{Event, Request, Response};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as AsyncBufReader};

/// Protocol handler for Claude Code JSON lines communication
pub struct Protocol;

impl Protocol {
    /// Serialize a message to JSON lines format
    pub fn serialize<T: Serialize>(message: &T) -> Result<String> {
        let json = serde_json::to_string(message)?;
        Ok(format!("{}\n", json))
    }

    /// Deserialize a JSON line into a message
    pub fn deserialize<T: for<'de> Deserialize<'de>>(line: &str) -> Result<T> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Err(Error::Protocol("Empty line".to_string()));
        }
        Ok(serde_json::from_str(trimmed)?)
    }

    /// Write a message to a synchronous writer
    pub fn write_sync<W: Write, T: Serialize>(writer: &mut W, message: &T) -> Result<()> {
        let line = Self::serialize(message)?;
        writer.write_all(line.as_bytes())?;
        writer.flush()?;
        Ok(())
    }

    /// Read a message from a synchronous reader
    pub fn read_sync<R: BufRead, T: for<'de> Deserialize<'de>>(reader: &mut R) -> Result<T> {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            return Err(Error::ConnectionClosed);
        }
        Self::deserialize(&line)
    }

    /// Write a message to an async writer
    pub async fn write_async<W: AsyncWriteExt + Unpin, T: Serialize>(
        writer: &mut W,
        message: &T,
    ) -> Result<()> {
        let line = Self::serialize(message)?;
        writer.write_all(line.as_bytes()).await?;
        writer.flush().await?;
        Ok(())
    }

    /// Read a message from an async reader
    pub async fn read_async<R: AsyncBufReadExt + Unpin, T: for<'de> Deserialize<'de>>(
        reader: &mut R,
    ) -> Result<T> {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            return Err(Error::ConnectionClosed);
        }
        Self::deserialize(&line)
    }
}

/// Message envelope for routing different message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "message_class", rename_all = "snake_case")]
pub enum MessageEnvelope {
    Request(Request),
    Response(Response),
    Event(Event),
}

/// Stream processor for handling continuous message streams
pub struct StreamProcessor<R> {
    reader: BufReader<R>,
}

impl<R: std::io::Read> StreamProcessor<R> {
    /// Create a new stream processor
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
        }
    }

    /// Process the next message from the stream
    pub fn next_message<T: for<'de> Deserialize<'de>>(&mut self) -> Result<T> {
        Protocol::read_sync(&mut self.reader)
    }

    /// Process all messages in the stream
    pub fn process_all<T, F>(&mut self, mut handler: F) -> Result<()>
    where
        T: for<'de> Deserialize<'de>,
        F: FnMut(T) -> Result<()>,
    {
        loop {
            match self.next_message() {
                Ok(message) => handler(message)?,
                Err(Error::ConnectionClosed) => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

/// Async stream processor for handling continuous message streams
pub struct AsyncStreamProcessor<R> {
    reader: AsyncBufReader<R>,
}

impl<R: tokio::io::AsyncRead + Unpin> AsyncStreamProcessor<R> {
    /// Create a new async stream processor
    pub fn new(reader: R) -> Self {
        Self {
            reader: AsyncBufReader::new(reader),
        }
    }

    /// Process the next message from the stream
    pub async fn next_message<T: for<'de> Deserialize<'de>>(&mut self) -> Result<T> {
        Protocol::read_async(&mut self.reader).await
    }

    /// Process all messages in the stream
    pub async fn process_all<T, F, Fut>(&mut self, mut handler: F) -> Result<()>
    where
        T: for<'de> Deserialize<'de>,
        F: FnMut(T) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        loop {
            match self.next_message().await {
                Ok(message) => handler(message).await?,
                Err(Error::ConnectionClosed) => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::*;

    #[test]
    fn test_serialize_deserialize() {
        let request = Request {
            message_type: "request".to_string(),
            id: "test-123".to_string(),
            session_id: Some("session-456".to_string()),
            payload: RequestPayload::Initialize(InitializeRequest {
                working_directory: Some("/home/user".to_string()),
                environment: None,
                capabilities: None,
            }),
            metadata: None,
        };

        let serialized = Protocol::serialize(&request).unwrap();
        assert!(serialized.ends_with('\n'));

        let deserialized: Request = Protocol::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.id, request.id);
    }

    #[test]
    fn test_empty_line_error() {
        let result: Result<Request> = Protocol::deserialize("");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_json_error() {
        let result: Result<Request> = Protocol::deserialize("not valid json");
        assert!(result.is_err());
    }
}
