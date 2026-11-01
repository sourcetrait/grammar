//! The remote-link wire: zstd-compressed bitcode in a length-delimited frame.
use crate::*;

/// The zstd level applied to every remote frame before it hits the wire.
const REMOTE_ZSTD_LEVEL: i32 = 3;

/// The most a remote frame inflates to.
pub(crate) const REMOTE_INFLATED_MAX: u64 = 8 * 1024 * 1024;

/// The most a remote frame takes on the wire, zstd's bound at the inflated cap.
fn remote_frame_max() -> usize {
    zstd::zstd_safe::compress_bound(REMOTE_INFLATED_MAX as usize)
}

/// Which of a link's two connections a handshake announces.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, ser::Serialize, ser::Deserialize, bitcode::Encode, bitcode::Decode,
)]
pub(crate) enum RemoteStream {
    /// The message/control connection.
    Message,
    /// The file-transfer connection.
    File,
}

/// The message connection's frame: one delivery, sent by either end.
#[derive(
    Debug, Clone, PartialEq, ser::Serialize, ser::Deserialize, bitcode::Encode, bitcode::Decode,
)]
pub(crate) enum MsgFrame {
    /// A message to deliver: its id, model, event, and the files it waits for.
    Deliver {
        id: String,
        model: String,
        event_nuon: String,
        files: Vec<String>,
    },
}

/// A file-connection frame: one ordered chunk of one dest file for a delivery.
#[derive(
    Debug, Clone, PartialEq, ser::Serialize, ser::Deserialize, bitcode::Encode, bitcode::Decode,
)]
pub(crate) enum FileFrame {
    Chunk {
        id: String,
        dest: String,
        seq: u32,
        bytes: Vec<u8>,
        last: bool,
    },
}

/// Messages a link's initiator sends to its acceptor.
#[derive(
    Debug, Clone, PartialEq, ser::Serialize, ser::Deserialize, bitcode::Encode, bitcode::Decode,
)]
pub(crate) enum InitiatorToAcceptor {
    /// The opening handshake: the initiator's nom and the stream it carries.
    Hello { mcp_nom: String, stream: RemoteStream },
    /// A message-connection frame (rides the Message connection).
    Msg(MsgFrame),
    /// A file-connection frame (rides the File connection).
    File(FileFrame),
    /// Transport-level close; the peer answers with its own `Close`.
    Close,
}

/// Messages a link's acceptor sends to its initiator.
#[derive(
    Debug, Clone, PartialEq, ser::Serialize, ser::Deserialize, bitcode::Encode, bitcode::Decode,
)]
pub(crate) enum AcceptorToInitiator {
    /// Handshake reply: the acceptor's McpNom.
    Hello { mcp_nom: String },
    Msg(MsgFrame),
    File(FileFrame),
    Close,
}

/// A tokio-util codec: a zstd-compressed bitcode `T` in a length-delimited frame.
pub(crate) struct BitcodeCodec<T> {
    frames: tku::LengthDelimitedCodec,
    _marker: PhantomData<T>,
}

impl<T> BitcodeCodec<T> {
    pub(crate) fn new() -> Self {
        Self {
            frames: tku::LengthDelimitedCodec::builder()
                .max_frame_length(remote_frame_max())
                .new_codec(),
            _marker: PhantomData,
        }
    }
}

/// A frame its peer would refuse, refused before it is queued.
pub(crate) fn check_outbound<T>(frame: &T) -> Result<(), String>
where
    T: bitcode::Encode + bitcode::DecodeOwned,
{
    let bytes = bitcode::encode(frame);
    if bytes.len() as u64 > REMOTE_INFLATED_MAX {
        return Err(format!(
            "remote send refused: a frame of {} bytes passes the peer's {REMOTE_INFLATED_MAX}-byte cap",
            bytes.len()
        ));
    }
    decode_payload::<T>(&bytes, DECODE_BUDGET_DEFAULT)
        .map(drop)
        .map_err(|e| format!("remote send refused: the peer would refuse a frame: {e}"))
}

impl<T> tku::Decoder for BitcodeCodec<T>
where
    T: bitcode::DecodeOwned,
{
    type Item = T;
    type Error = io::Error;

    fn decode(&mut self, src: &mut tku::BytesMut) -> Result<Option<T>, io::Error> {
        let Some(frame) = self.frames.decode(src)? else {
            return Ok(None);
        };
        let bytes = inflate(frame.as_ref())?;
        decode_payload(&bytes, DECODE_BUDGET_DEFAULT).map(Some).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("remote frame: {e}"))
        })
    }
}

/// A frame's zstd payload inflated whole, refused past `REMOTE_INFLATED_MAX`.
fn inflate(frame: &[u8]) -> io::Result<Vec<u8>> {
    let failed = |e: io::Error| {
        io::Error::new(io::ErrorKind::InvalidData, format!("remote frame: zstd decode failed: {e}"))
    };
    let decoder = zstd::stream::read::Decoder::new(frame).map_err(failed)?;
    let mut bytes = Vec::new();
    let mut bounded = std::io::Read::take(decoder, REMOTE_INFLATED_MAX + 1);
    std::io::Read::read_to_end(&mut bounded, &mut bytes).map_err(failed)?;
    if bytes.len() as u64 > REMOTE_INFLATED_MAX {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("remote frame: it inflates past {REMOTE_INFLATED_MAX} bytes"),
        ));
    }
    Ok(bytes)
}

impl<T> tku::Encoder<T> for BitcodeCodec<T>
where
    T: bitcode::Encode,
{
    type Error = io::Error;

    fn encode(&mut self, item: T, dst: &mut tku::BytesMut) -> Result<(), io::Error> {
        let bytes = bitcode::encode(&item);
        let compressed = zstd::encode_all(bytes.as_slice(), REMOTE_ZSTD_LEVEL)
            .map_err(|e| io::Error::other(format!("remote frame: zstd encode failed: {e}")))?;
        self.frames.encode(tku::Bytes::from(compressed), dst)
    }
}
