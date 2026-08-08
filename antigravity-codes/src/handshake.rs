//! The length-prefixed stdio handshake that precedes the WebSocket session.
//!
//! Everything else in this protocol is protobuf-*JSON*, but the very first
//! exchange is binary protobuf on the harness's stdin/stdout:
//!
//! ```text
//! client -> harness   u32le length, then a serialised InputConfig
//! harness -> client   u32le length, then a serialised OutputConfig
//! ```
//!
//! `OutputConfig` carries the loopback port the harness just bound and the
//! per-process API key the WebSocket upgrade must present. Two messages, seven
//! fields between them, so this module hand-rolls the wire format rather than
//! taking a protobuf runtime dependency for it — see
//! `scripts/codegen_antigravity.py` for why a runtime is otherwise unnecessary.
//!
//! The field numbers below are pinned by the descriptor snapshot in
//! `tests/schemas/localharness.descriptor.bin`, and this module's tests lock them
//! against
//! bytes captured from a live 0.1.10 harness.

use crate::protocol::{ClientInfo, InputConfig, OutputConfig};

/// A protobuf wire type. Only these two occur in the handshake messages.
const WIRE_VARINT: u8 = 0;
const WIRE_LEN: u8 = 2;

fn tag(out: &mut Vec<u8>, field: u32, wire: u8) {
    put_varint(out, u64::from(field) << 3 | u64::from(wire));
}

fn put_varint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let byte = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

fn put_string(out: &mut Vec<u8>, field: u32, v: &str) {
    tag(out, field, WIRE_LEN);
    put_varint(out, v.len() as u64);
    out.extend_from_slice(v.as_bytes());
}

fn put_message(out: &mut Vec<u8>, field: u32, body: &[u8]) {
    tag(out, field, WIRE_LEN);
    put_varint(out, body.len() as u64);
    out.extend_from_slice(body);
}

fn put_u32(out: &mut Vec<u8>, field: u32, v: u32) {
    tag(out, field, WIRE_VARINT);
    put_varint(out, u64::from(v));
}

fn encode_client_info(ci: &ClientInfo) -> Vec<u8> {
    let mut out = Vec::new();
    if let Some(v) = &ci.language {
        put_string(&mut out, 1, v);
    }
    if let Some(v) = &ci.version {
        put_string(&mut out, 2, v);
    }
    if let Some(v) = &ci.language_version {
        put_string(&mut out, 3, v);
    }
    if let Some(v) = &ci.os {
        put_string(&mut out, 4, v);
    }
    if let Some(v) = &ci.os_version {
        put_string(&mut out, 5, v);
    }
    out
}

/// Serialises an [`InputConfig`] to binary protobuf, fields in tag order.
pub fn encode_input_config(cfg: &InputConfig) -> Vec<u8> {
    let mut out = Vec::new();
    if let Some(v) = &cfg.storage_directory {
        put_string(&mut out, 1, v);
    }
    if let Some(v) = cfg.port {
        put_u32(&mut out, 2, v);
    }
    if let Some(v) = &cfg.bind_address {
        put_string(&mut out, 3, v);
    }
    if let Some(ci) = &cfg.client_info {
        put_message(&mut out, 4, &encode_client_info(ci));
    }
    // `map<string, string> env = 5` — each entry is a synthetic message with
    // `key = 1` and `value = 2`. Sorted so the encoding is deterministic.
    let mut env: Vec<_> = cfg.env.iter().collect();
    env.sort_by(|a, b| a.0.cmp(b.0));
    for (k, v) in env {
        let mut entry = Vec::new();
        put_string(&mut entry, 1, k);
        put_string(&mut entry, 2, v);
        put_message(&mut out, 5, &entry);
    }
    out
}

/// Prefixes a serialised message with its length, as the harness expects.
pub fn frame(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len() + 4);
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    out
}

/// A malformed handshake reply.
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    /// The buffer ended in the middle of a field.
    #[error("truncated protobuf message at byte {0}")]
    Truncated(usize),
    /// A varint ran past the 10 bytes a u64 can occupy.
    #[error("malformed varint at byte {0}")]
    BadVarint(usize),
    /// A field used a wire type the handshake never emits.
    #[error("unsupported wire type {wire} for field {field}")]
    UnsupportedWireType {
        /// The protobuf field number that carried it.
        field: u32,
        /// The wire type encountered.
        wire: u8,
    },
    /// A string field was not valid UTF-8.
    #[error("field {0} is not valid UTF-8")]
    NotUtf8(u32),
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn varint(&mut self) -> Result<u64, DecodeError> {
        let mut value = 0u64;
        let mut shift = 0;
        loop {
            let byte = *self
                .buf
                .get(self.pos)
                .ok_or(DecodeError::Truncated(self.pos))?;
            self.pos += 1;
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
            shift += 7;
            if shift >= 64 {
                return Err(DecodeError::BadVarint(self.pos));
            }
        }
    }

    fn bytes(&mut self) -> Result<&'a [u8], DecodeError> {
        let len = self.varint()? as usize;
        let end = self
            .pos
            .checked_add(len)
            .ok_or(DecodeError::Truncated(self.pos))?;
        let slice = self
            .buf
            .get(self.pos..end)
            .ok_or(DecodeError::Truncated(self.pos))?;
        self.pos = end;
        Ok(slice)
    }

    /// Steps over a field this decoder does not care about.
    fn skip(&mut self, field: u32, wire: u8) -> Result<(), DecodeError> {
        match wire {
            WIRE_VARINT => {
                self.varint()?;
            }
            WIRE_LEN => {
                self.bytes()?;
            }
            1 => self.pos += 8,
            5 => self.pos += 4,
            _ => return Err(DecodeError::UnsupportedWireType { field, wire }),
        }
        Ok(())
    }
}

/// Parses the `OutputConfig` the harness writes to stdout.
///
/// Unknown fields are skipped, so a newer harness that adds to this message
/// still hands back a usable port and key.
pub fn decode_output_config(buf: &[u8]) -> Result<OutputConfig, DecodeError> {
    let mut r = Reader { buf, pos: 0 };
    let mut cfg = OutputConfig::default();
    while r.pos < buf.len() {
        let key = r.varint()?;
        let field = (key >> 3) as u32;
        let wire = (key & 7) as u8;
        match (field, wire) {
            (1, WIRE_VARINT) => cfg.port = Some(r.varint()? as i32),
            (2, WIRE_LEN) => {
                let raw = r.bytes()?;
                cfg.api_key = Some(
                    std::str::from_utf8(raw)
                        .map_err(|_| DecodeError::NotUtf8(2))?
                        .to_string(),
                );
            }
            _ => r.skip(field, wire)?,
        }
    }
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured from a live 0.1.10 harness handshake.
    const GOLDEN_INPUT: &str = "0a0c2f746d702f616773746f7265221e0a0472757374120\
6302e312e31301a04312e383522056c696e75782a0136";
    const GOLDEN_OUTPUT: &str = "08e5bc021220373130643035656337373862323462363338\
3663636632396530316536613031";

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn encodes_byte_for_byte_like_the_reference_client() {
        let cfg = InputConfig {
            storage_directory: Some("/tmp/agstore".into()),
            client_info: Some(ClientInfo {
                language: Some("rust".into()),
                version: Some("0.1.10".into()),
                language_version: Some("1.85".into()),
                os: Some("linux".into()),
                os_version: Some("6".into()),
            }),
            ..Default::default()
        };
        assert_eq!(encode_input_config(&cfg), unhex(GOLDEN_INPUT));
    }

    #[test]
    fn decodes_a_captured_output_config() {
        let cfg = decode_output_config(&unhex(GOLDEN_OUTPUT)).unwrap();
        assert_eq!(cfg.port, Some(40549));
        assert_eq!(
            cfg.api_key.as_deref(),
            Some("710d05ec778b24b6386ccf29e01e6a01")
        );
    }

    #[test]
    fn skips_fields_a_newer_harness_might_add() {
        let mut buf = unhex(GOLDEN_OUTPUT);
        put_string(&mut buf, 9, "something-new");
        put_u32(&mut buf, 10, 42);
        let cfg = decode_output_config(&buf).unwrap();
        assert_eq!(cfg.port, Some(40549));
    }

    #[test]
    fn rejects_a_truncated_message() {
        let buf = unhex(GOLDEN_OUTPUT);
        assert!(decode_output_config(&buf[..buf.len() - 4]).is_err());
    }

    #[test]
    fn frames_with_a_little_endian_length() {
        assert_eq!(frame(&[1, 2, 3]), vec![3, 0, 0, 0, 1, 2, 3]);
    }

    #[test]
    fn encodes_env_entries_deterministically() {
        let cfg = InputConfig {
            env: [
                ("B".to_string(), "2".to_string()),
                ("A".to_string(), "1".to_string()),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        };
        let once = encode_input_config(&cfg);
        assert_eq!(once, encode_input_config(&cfg));
        // Entry for "A" sorts first: tag 5 (0x2a), len 6, key "A", value "1".
        assert_eq!(&once[..2], &[0x2a, 0x06]);
    }
}
