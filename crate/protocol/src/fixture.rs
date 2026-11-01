//! The v1 fixture driver: golden vectors decoded by type, held to their bytes.
use crate::*;

/// One golden vector as its file's row holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureVector {
    pub name: String,
    /// The value's type, its module paths dropped.
    pub ty: String,
    pub bytes: Vec<u8>,
}

/// The decoders a vector's type names, and the list of those names.
macro_rules! decoders {
    ($($name:ident),* $(,)?) => {
        /// Every type a v1 golden vector names.
        pub const FIXTURE_TYPES: &[&str] = &[$(stringify!($name)),*];

        /// `payload` decoded as the type `ty` names and encoded back, if one does.
        pub fn canonical(ty: &str, payload: &[u8], budget: DecodeBudget) -> Option<Result<Vec<u8>, FrameError>> {
            match ty {
                $(stringify!($name) => Some(decode_payload::<$name>(payload, budget).map(|value| bitcode::encode(&value))),)*
                _ => None,
            }
        }
    };
}

decoders!(
    ChannelSendData,
    ClientHello,
    Emergency,
    EngineRequest,
    EngineResponse,
    FromEngineSys,
    HelloReply,
    HostAnswer,
    McpFrame,
    NuVal,
    NuValError,
    Observation,
    PreparedSend,
    PurviewRow,
    RemoteChannelEvent,
    RemoteChannelRow,
    RemotePacket,
    RemoteResult,
    RemoteSendData,
    RuntimeError,
    SessionAsk,
    SpamThresholdsData,
    ToEngineSys,
    TransportError,
    WorkerFrame,
);

/// A vectors file's rows, read from its NUON text.
pub fn read_vectors(text: &str) -> Result<Vec<FixtureVector>, String> {
    let NuVal::List(rows) = NuVal::from_nuon(text).map_err(|e| e.to_string())? else {
        return Err("a vectors file that isn't a table".to_string());
    };
    rows.into_iter().map(read_row).collect()
}

/// One row: its name, its type, and its hex as bytes.
fn read_row(row: NuVal) -> Result<FixtureVector, String> {
    let NuVal::Record(fields) = row else { return Err("a row that isn't a record".to_string()) };
    let field = |key: &str| match fields.iter().find(|(name, _)| name == key) {
        Some((_, NuVal::String(text))) => Ok(text.clone()),
        _ => Err(format!("a row without its `{key}` text")),
    };
    let (name, ty, hex) = (field("name")?, field("type")?, field("hex")?);
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|at| hex.get(at..at + 2).and_then(|pair| u8::from_str_radix(pair, 16).ok()))
        .collect::<Option<Vec<u8>>>()
        .ok_or_else(|| format!("{name}: hex that isn't two digits a byte"))?;
    Ok(FixtureVector { name, ty, bytes })
}

/// A NUON file's digest: the SHA-256 of its value's canonical text.
pub fn digest(text: &str) -> Result<[u8; 32], String> {
    use sha2::{Digest, Sha256};
    let value = NuVal::from_nuon(text).map_err(|e| e.to_string())?;
    Ok(Sha256::digest(value.to_nuon(nuval::NuonStyle::Raw)).into())
}

/// What a vector fails: its type, its bytes' round trip, or a malformed form.
pub fn check_vector(vector: &FixtureVector, budget: DecodeBudget) -> Result<(), String> {
    let decode = |bytes: &[u8]| canonical(&vector.ty, bytes, budget);
    match decode(&vector.bytes) {
        None => return Err(format!("{}: a type `{}` no decoder takes", vector.name, vector.ty)),
        Some(Err(e)) => return Err(format!("{}: refused: {e}", vector.name)),
        Some(Ok(back)) if back != vector.bytes => return Err(format!("{}: decodes to other bytes", vector.name)),
        Some(Ok(_)) => {}
    }
    if let Some(cut) = (0..vector.bytes.len()).find(|cut| matches!(decode(&vector.bytes[..*cut]), Some(Ok(_)))) {
        return Err(format!("{}: decoded from its first {cut} bytes", vector.name));
    }
    let mut longer = vector.bytes.clone();
    longer.push(0);
    if matches!(decode(&longer), Some(Ok(_))) {
        return Err(format!("{}: decoded with a trailing byte", vector.name));
    }
    Ok(())
}
