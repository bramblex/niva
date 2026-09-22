use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Wire protocol (UDP-like discipline over the ordered WebSocket).
///
/// Versioning: text sessions negotiate via `hello.v`, binary frames carry
/// `BIN_VERSION`. Bump both together; mismatches are refused loudly.
///
/// Text frames are JSON objects with a `t` discriminator. Binary frames
/// carry an 18-byte header (version, flags, id, seq) followed by payload;
/// message boundaries come from WS framing itself, so no length field.
///
/// Sequences are per-call and per-direction, starting at 1 for stream
/// frames; terminal `result` messages carry no meaningful seq. On a single
/// ordered socket nothing can actually arrive out of order — seq exists for
/// chunk reassembly framing and progress tracking, not loss recovery
/// (TCP already guarantees delivery).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum ClientMsg {
    /// Open a window binding: `{t:"hello", wid, v}`.
    Hello { wid: u8, v: u8 },
    /// Unary or stateful call: `{t:"call", id, method, args}`.
    Call {
        id: u64,
        method: String,
        args: Value,
    },
    /// Abort a stateful call: `{t:"cancel", id}`.
    Cancel { id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum ServerMsg {
    /// Terminal response: `{t:"result", id, code, message, data}`.
    /// code 0 = ok; -1 = handler error; -2 = timeout.
    Result {
        id: u64,
        code: i32,
        message: String,
        data: Value,
    },
    /// Stream push: `{t:"event", id?, seq, name, data}`. `id` ties the push
    /// to a stateful call; absent means a global broadcast.
    Event {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<u64>,
        seq: u64,
        name: String,
        data: Value,
    },
}

impl ServerMsg {
    pub fn result(id: u64, code: i32, message: String, data: Value) -> Self {
        ServerMsg::Result {
            id,
            code,
            message,
            data,
        }
    }

    pub fn event(id: Option<u64>, seq: u64, name: String, data: Value) -> Self {
        ServerMsg::Event {
            id,
            seq,
            name,
            data,
        }
    }

    pub fn encode(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            "{\"t\":\"result\",\"id\":0,\"code\":-1,\"message\":\"encode error\",\"data\":null}"
                .to_string()
        })
    }
}

/// Binary chunk header: version(1) + flags(1) + id u64 BE + seq u64 BE.
/// Current wire version (text hello + binary header).
pub const WIRE_VERSION: u8 = 1;
pub const BIN_VERSION: u8 = WIRE_VERSION;
pub const BIN_HEADER_LEN: usize = 18;
pub const FLAG_START: u8 = 0x01;
pub const FLAG_END: u8 = 0x02;
/// Set on stderr sub-stream frames (exec); clear means stdout / data.
pub const FLAG_STDERR: u8 = 0x04;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkHeader {
    pub id: u64,
    pub seq: u64,
    pub start: bool,
    pub end: bool,
    pub stderr: bool,
}

#[derive(Debug)]
pub enum ChunkError {
    TooShort,
    BadVersion(u8),
}

impl std::fmt::Display for ChunkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChunkError::TooShort => write!(f, "binary frame shorter than header"),
            ChunkError::BadVersion(v) => write!(f, "unknown chunk version {v}"),
        }
    }
}

impl std::error::Error for ChunkError {}

pub fn encode_chunk(
    id: u64,
    seq: u64,
    start: bool,
    end: bool,
    stderr: bool,
    payload: &[u8],
) -> Vec<u8> {
    let mut flags = 0u8;
    if start {
        flags |= FLAG_START;
    }
    if end {
        flags |= FLAG_END;
    }
    if stderr {
        flags |= FLAG_STDERR;
    }
    let mut out = Vec::with_capacity(BIN_HEADER_LEN + payload.len());
    out.push(BIN_VERSION);
    out.push(flags);
    out.extend_from_slice(&id.to_be_bytes());
    out.extend_from_slice(&seq.to_be_bytes());
    out.extend_from_slice(payload);
    out
}

pub fn decode_chunk(frame: &[u8]) -> Result<(ChunkHeader, &[u8]), ChunkError> {
    if frame.len() < BIN_HEADER_LEN {
        return Err(ChunkError::TooShort);
    }
    if frame[0] != BIN_VERSION {
        return Err(ChunkError::BadVersion(frame[0]));
    }
    let flags = frame[1];
    let id = u64::from_be_bytes(frame[2..10].try_into().unwrap());
    let seq = u64::from_be_bytes(frame[10..18].try_into().unwrap());
    Ok((
        ChunkHeader {
            id,
            seq,
            start: flags & FLAG_START != 0,
            end: flags & FLAG_END != 0,
            stderr: flags & FLAG_STDERR != 0,
        },
        &frame[BIN_HEADER_LEN..],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_carries_version() {
        let hello = ClientMsg::Hello {
            wid: 3,
            v: WIRE_VERSION,
        };
        let text = serde_json::to_string(&hello).unwrap();
        let back: ClientMsg = serde_json::from_str(&text).unwrap();
        match back {
            ClientMsg::Hello { wid, v } => {
                assert_eq!(wid, 3);
                assert_eq!(v, WIRE_VERSION);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn text_messages_roundtrip() {
        let call = ClientMsg::Call {
            id: 300,
            method: "fs.read".into(),
            args: serde_json::json!(["/tmp/x"]),
        };
        let text = serde_json::to_string(&call).unwrap();
        let back: ClientMsg = serde_json::from_str(&text).unwrap();
        match back {
            ClientMsg::Call { id, method, .. } => {
                assert_eq!(id, 300);
                assert_eq!(method, "fs.read");
            }
            _ => panic!("wrong variant"),
        }

        // u64 ids beyond the old u8 range must survive.
        let big = ClientMsg::Call {
            id: u64::MAX,
            method: "x".into(),
            args: Value::Null,
        };
        let text = serde_json::to_string(&big).unwrap();
        let back: ClientMsg = serde_json::from_str(&text).unwrap();
        match back {
            ClientMsg::Call { id, .. } => assert_eq!(id, u64::MAX),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn chunk_header_roundtrip() {
        let payload = b"hello-world";
        let frame = encode_chunk(42, 7, true, false, false, payload);
        assert_eq!(frame.len(), BIN_HEADER_LEN + payload.len());
        let (header, body) = decode_chunk(&frame).unwrap();
        assert_eq!(
            header,
            ChunkHeader {
                id: 42,
                seq: 7,
                start: true,
                end: false,
                stderr: false,
            }
        );
        assert_eq!(body, payload);
    }

    #[test]
    fn chunk_decode_rejects_garbage() {
        assert!(matches!(
            decode_chunk(&[1, 2, 3]),
            Err(ChunkError::TooShort)
        ));
        let mut frame = encode_chunk(1, 1, false, true, false, b"x");
        frame[0] = 0x7f;
        assert!(matches!(
            decode_chunk(&frame),
            Err(ChunkError::BadVersion(0x7f))
        ));
    }
}
