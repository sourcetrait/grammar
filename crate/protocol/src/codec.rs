//! The wire's framing: a preface, then length-prefixed bitcode frames.
use crate::*;

/// The four bytes each direction of a stream writes before its first frame.
pub const PREFACE: [u8; 4] = *b"GRMR";

/// The codec both ends must name in their hellos: the bitcode fork's own name.
pub const CODEC_ID: &str = bitcode::CODEC;

/// The most of a stream's first bytes a bad preface's refusal shows.
const HEAD_SHOWN: usize = 64;

/// The SHA-256 of `schema/v1.nuon`'s canonical form, named by both hellos.
pub const SCHEMA_ID: [u8; 32] =
    schema_id("4af9c4f49ae7864438773eeabd8c06a0fea8cfadc89fd0edf9b24cc8f364a156");

/// An ID's 32 bytes from its 64 lowercase hex digits, checked as it builds.
const fn schema_id(hex: &str) -> [u8; 32] {
    let hex = hex.as_bytes();
    assert!(hex.len() == 64, "a schema ID is 64 hex digits");
    let mut id = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        id[i] = (nibble(hex[2 * i]) << 4) | nibble(hex[2 * i + 1]);
        i += 1;
    }
    id
}

/// One lowercase hex digit's value.
const fn nibble(digit: u8) -> u8 {
    match digit {
        b'0'..=b'9' => digit - b'0',
        b'a'..=b'f' => digit - b'a' + 10,
        _ => panic!("a schema ID is lowercase hex"),
    }
}

/// The big-endian payload length before each frame.
const HEADER_LEN: usize = 4;

/// Refuse every codec but the pinned one, a compressed variant among them.
pub fn check_codec(id: &str) -> Result<(), FrameError> {
    if id == CODEC_ID {
        Ok(())
    } else {
        Err(FrameError::UnsupportedCodec { id: id.to_string() })
    }
}

/// Where in a stream a read ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameStage {
    Preface,
    Header,
    Body,
}

/// A framing failure; each closes its own stream alone.
#[derive(Debug)]
pub enum FrameError {
    /// The stream ended before its first byte.
    ClosedBeforePreface,
    /// A preface byte differed from `GRMR`, the stream's first bytes kept.
    BadPreface { at: usize, got: u8, head: Vec<u8> },
    /// The stream ended inside a preface, a header, or a body.
    Truncated { stage: FrameStage, held: usize, expected: usize },
    /// A frame declared no payload.
    ZeroLength,
    /// A received frame declared more than the stream's ceiling.
    Oversize { len: u32, ceiling: u32 },
    /// A frame to send holds more than the stream's ceiling.
    TooLarge { len: usize, ceiling: u32 },
    /// The peer named a codec other than the pinned one.
    UnsupportedCodec { id: String },
    /// A payload did not decode as its type, trailing bytes or an unknown variant.
    Decode,
    /// A payload declared more than its decode budget allows.
    Budget { bound: bitcode::Exceeded, limit: u64 },
    /// A payload's type holds a collection no decode budget bounds.
    Unbudgeted,
    /// The transport failed.
    Io(io::Error),
}

impl From<io::Error> for FrameError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ClosedBeforePreface => write!(f, "the stream closed before its preface"),
            Self::BadPreface { at, got, head } => {
                let head = String::from_utf8_lossy(head);
                write!(f, "preface byte {at} is {got:#04x}, not GRMR's; the stream began {head:?}")
            }
            Self::Truncated { stage, held, expected } => {
                write!(f, "the stream ended in a {stage:?} with {held} of {expected} bytes")
            }
            Self::ZeroLength => write!(f, "a frame declared no payload"),
            Self::Oversize { len, ceiling } => {
                write!(f, "a frame declared {len} bytes past the {ceiling}-byte ceiling")
            }
            Self::TooLarge { len, ceiling } => {
                write!(f, "a frame of {len} bytes exceeds the {ceiling}-byte ceiling")
            }
            Self::UnsupportedCodec { id } => write!(f, "codec {id} where {CODEC_ID} is required"),
            Self::Decode => write!(f, "a payload did not decode as its type"),
            Self::Budget { bound, limit } => {
                write!(f, "a payload passed its decode budget's {bound} bound of {limit}")
            }
            Self::Unbudgeted => {
                write!(f, "a payload's type holds a collection no decode budget bounds")
            }
            Self::Io(e) => write!(f, "the transport failed: {e}"),
        }
    }
}

impl std::error::Error for FrameError {}

impl FrameError {
    /// The stable transport error the frontend reports for this failure.
    pub fn transport(&self) -> TransportError {
        match self {
            Self::TooLarge { ceiling, .. } => TransportError::ResultTooLarge {
                limit: u64::from(*ceiling),
            },
            Self::UnsupportedCodec { id } => TransportError::CodecMismatch {
                expected: CODEC_ID.to_string(),
                found: id.clone(),
            },
            other => TransportError::InvalidFrame { reason: other.to_string() },
        }
    }
}

/// One stream's framing: the peer's preface, then frames up to the ceiling.
#[derive(Debug)]
pub struct FrameCodec {
    ceiling: u32,
    preface_read: usize,
    preface_written: bool,
    body_len: Option<usize>,
}

impl Default for FrameCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameCodec {
    /// A new stream's codec, held to the hello's ceiling until the session's is set.
    pub fn new() -> Self {
        Self {
            ceiling: HELLO_FRAME_MAX,
            preface_read: 0,
            preface_written: false,
            body_len: None,
        }
    }

    /// The ceiling later frames are held to, once the hellos negotiate it.
    pub fn set_ceiling(&mut self, ceiling: u32) {
        self.ceiling = ceiling;
    }

    /// The ceiling frames are held to now.
    pub fn ceiling(&self) -> u32 {
        self.ceiling
    }
}

impl tku::Decoder for FrameCodec {
    type Item = tku::Bytes;
    type Error = FrameError;

    fn decode(&mut self, src: &mut tku::BytesMut) -> Result<Option<tku::Bytes>, FrameError> {
        use tku::Buf;
        while self.preface_read < PREFACE.len() {
            let Some(&got) = src.first() else {
                return Ok(None);
            };
            if got != PREFACE[self.preface_read] {
                let mut head = PREFACE[..self.preface_read].to_vec();
                head.extend(src.iter().take(HEAD_SHOWN - self.preface_read));
                return Err(FrameError::BadPreface { at: self.preface_read, got, head });
            }
            src.advance(1);
            self.preface_read += 1;
        }
        let len = match self.body_len {
            Some(len) => len,
            None => {
                let Some(header) = src.get(..HEADER_LEN) else {
                    return Ok(None);
                };
                let len = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
                if len == 0 {
                    return Err(FrameError::ZeroLength);
                }
                if len > self.ceiling {
                    return Err(FrameError::Oversize { len, ceiling: self.ceiling });
                }
                src.advance(HEADER_LEN);
                let len = len as usize;
                self.body_len = Some(len);
                src.reserve(len.saturating_sub(src.len()));
                len
            }
        };
        if src.len() < len {
            return Ok(None);
        }
        self.body_len = None;
        Ok(Some(src.split_to(len).freeze()))
    }

    fn decode_eof(&mut self, src: &mut tku::BytesMut) -> Result<Option<tku::Bytes>, FrameError> {
        if let Some(frame) = self.decode(src)? {
            return Ok(Some(frame));
        }
        if self.preface_read == 0 {
            return Err(FrameError::ClosedBeforePreface);
        }
        if self.preface_read < PREFACE.len() {
            return Err(FrameError::Truncated {
                stage: FrameStage::Preface,
                held: self.preface_read,
                expected: PREFACE.len(),
            });
        }
        match self.body_len {
            Some(len) => Err(FrameError::Truncated {
                stage: FrameStage::Body,
                held: src.len(),
                expected: len,
            }),
            None if src.is_empty() => Ok(None),
            None => Err(FrameError::Truncated {
                stage: FrameStage::Header,
                held: src.len(),
                expected: HEADER_LEN,
            }),
        }
    }
}

impl tku::Encoder<tku::Bytes> for FrameCodec {
    type Error = FrameError;

    fn encode(&mut self, payload: tku::Bytes, dst: &mut tku::BytesMut) -> Result<(), FrameError> {
        if payload.is_empty() {
            return Err(FrameError::ZeroLength);
        }
        let len = u32::try_from(payload.len())
            .ok()
            .filter(|len| *len <= self.ceiling)
            .ok_or(FrameError::TooLarge { len: payload.len(), ceiling: self.ceiling })?;
        if !self.preface_written {
            dst.extend_from_slice(&PREFACE);
            self.preface_written = true;
        }
        dst.reserve(HEADER_LEN + payload.len());
        dst.extend_from_slice(&len.to_be_bytes());
        dst.extend_from_slice(&payload);
        Ok(())
    }
}

/// The codec over a byte stream both ways: an SSH channel, or a duplex.
pub fn framed<S>(stream: S) -> tku::Framed<S, FrameCodec>
where
    S: tk::AsyncRead + tk::AsyncWrite,
{
    tku::Framed::new(stream, FrameCodec::new())
}

/// A value as a frame's payload, encoded by the pinned bitcode.
pub fn payload<T: bitcode::Encode + ?Sized>(value: &T) -> tku::Bytes {
    tku::Bytes::from(bitcode::encode(value))
}

/// A payload as `T`, decoded within `budget`, a value past any bound refused.
pub fn decode_payload<T: bitcode::DecodeOwned>(
    payload: &[u8],
    budget: DecodeBudget,
) -> Result<T, FrameError> {
    bitcode::decode_budgeted(payload, budget).map_err(|e| match e {
        bitcode::BudgetError::Decode(_) => FrameError::Decode,
        bitcode::BudgetError::Exceeded(bound) => FrameError::Budget {
            bound,
            limit: match bound {
                bitcode::Exceeded::Elements => budget.elements,
                bitcode::Exceeded::Heap => budget.heap,
                bitcode::Exceeded::Depth => u64::from(budget.depth),
            },
        },
        bitcode::BudgetError::Unbudgeted => FrameError::Unbudgeted,
    })
}
