//! The wire's framing: the preface, the length-prefixed frames, their ceilings,
//! how a stream may end, the bounded decode's refusals, and the schema and its ID.
use std::collections::BTreeMap;
use std::path::Path;

use futures_util::{SinkExt, StreamExt};
use sourcetrait_grammar_protocol::{
    CODEC_ID, DECODE_BUDGET_DEFAULT, EmergencyKind, FRAME_DEFAULT, FrameCodec, FrameError, FrameStage,
    HELLO_FRAME_MAX, Lane, NuVal, PREFACE, SCHEMA_ID, TransportError, check_codec, framed, payload,
};

/// A payload as `T` within the default decode budget.
fn decode_payload<T: bitcode::DecodeOwned>(payload: &[u8]) -> Result<T, FrameError> {
    sourcetrait_grammar_protocol::decode_payload(payload, DECODE_BUDGET_DEFAULT)
}
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

#[path = "support/golden.rs"]
mod golden;

/// The wire bytes of `frames`, the preface first.
fn wire(frames: &[&[u8]]) -> Vec<u8> {
    let mut out = PREFACE.to_vec();
    for frame in frames {
        out.extend_from_slice(&(frame.len() as u32).to_be_bytes());
        out.extend_from_slice(frame);
    }
    out
}

/// Feed `bytes` to a fresh codec one byte at a time, collecting the frames.
fn feed_bytewise(bytes: &[u8]) -> Result<Vec<Bytes>, FrameError> {
    let mut codec = FrameCodec::new();
    let mut buf = BytesMut::new();
    let mut frames = Vec::new();
    for byte in bytes {
        buf.extend_from_slice(&[*byte]);
        if let Some(frame) = codec.decode(&mut buf)? {
            frames.push(frame);
        }
    }
    Ok(frames)
}

/// Decode `bytes` whole, then end the stream.
fn decode_to_end(codec: &mut FrameCodec, bytes: &[u8]) -> Result<Vec<Bytes>, FrameError> {
    let mut buf = BytesMut::from(bytes);
    let mut frames = Vec::new();
    while let Some(frame) = codec.decode(&mut buf)? {
        frames.push(frame);
    }
    while let Some(frame) = codec.decode_eof(&mut buf)? {
        frames.push(frame);
    }
    Ok(frames)
}

#[test]
fn a_frame_is_the_preface_a_big_endian_length_and_its_bitcode() {
    let mut codec = FrameCodec::new();
    let mut out = BytesMut::new();
    let value = [7u8; 16];
    codec.encode(payload(&value), &mut out).expect("encoded");
    codec.encode(payload(&value), &mut out).expect("encoded again");
    // The preface goes out once, before the first frame alone.
    let frame = bitcode::encode(&value);
    assert_eq!(out.as_ref(), wire(&[&frame, &frame]).as_slice());
    // Golden: bitcode packs the sixteen equal bytes into four, the same on
    // every platform.
    let golden = "4752414d4430303100000004090700000000000409070000";
    let hex: String = out.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(hex, golden);
    let back = decode_to_end(&mut FrameCodec::new(), &out).expect("decoded");
    assert_eq!(back.len(), 2);
    assert_eq!(decode_payload::<[u8; 16]>(&back[0]).expect("the bytes"), value);
}

#[test]
fn one_byte_reads_assemble_each_frame() {
    let bytes = wire(&[b"alpha", b"be"]);
    let frames = feed_bytewise(&bytes).expect("framed");
    assert_eq!(frames, vec![Bytes::from_static(b"alpha"), Bytes::from_static(b"be")]);
}

#[test]
fn coalesced_frames_come_apart_in_order() {
    let bytes = wire(&[b"one", b"two", b"three"]);
    let frames = decode_to_end(&mut FrameCodec::new(), &bytes).expect("framed");
    assert_eq!(frames, vec![Bytes::from_static(b"one"), Bytes::from_static(b"two"), Bytes::from_static(b"three")]);
}

#[test]
fn a_bad_preface_is_refused_at_its_first_wrong_byte() {
    // Five bytes are enough: the decoder does not wait for eight.
    let err = feed_bytewise(b"GRAMX").expect_err("refused");
    assert!(matches!(err, FrameError::BadPreface { at: 4, got: b'X' }), "{err}");
    assert_eq!(err.transport().kind(), "daemon::invalid_frame");
}

#[test]
fn an_empty_stream_closed_before_its_preface() {
    let err = decode_to_end(&mut FrameCodec::new(), b"").expect_err("refused");
    assert!(matches!(err, FrameError::ClosedBeforePreface), "{err}");
}

#[test]
fn a_stream_ending_inside_a_preface_a_header_or_a_body_says_where() {
    let full = wire(&[b"0123456789"]);
    let cases = [
        (5, FrameStage::Preface, 5, 8),
        (10, FrameStage::Header, 2, 4),
        (15, FrameStage::Body, 3, 10),
    ];
    for (cut, stage, held, expected) in cases {
        let err = decode_to_end(&mut FrameCodec::new(), &full[..cut]).expect_err("truncated");
        match err {
            FrameError::Truncated { stage: s, held: h, expected: e } => {
                assert_eq!((s, h, e), (stage, held, expected), "cut at {cut}");
            }
            other => panic!("cut at {cut}: {other}"),
        }
    }
}

#[test]
fn a_stream_ending_at_a_frame_boundary_ends_cleanly() {
    let frames = decode_to_end(&mut FrameCodec::new(), &wire(&[b"done"])).expect("clean");
    assert_eq!(frames, vec![Bytes::from_static(b"done")]);
    // The preface alone, then an end, is a clean end too.
    assert!(decode_to_end(&mut FrameCodec::new(), &PREFACE).expect("clean").is_empty());
}

/// A header declaring `len`, after the preface.
fn header(len: u32) -> BytesMut {
    let mut buf = BytesMut::from(&PREFACE[..]);
    buf.extend_from_slice(&len.to_be_bytes());
    buf
}

#[test]
fn zero_and_oversize_lengths_are_refused_before_any_reservation() {
    let mut codec = FrameCodec::new();
    let err = codec.decode(&mut header(0)).expect_err("refused");
    assert!(matches!(err, FrameError::ZeroLength), "{err}");
    for len in [HELLO_FRAME_MAX + 1, u32::MAX] {
        let mut codec = FrameCodec::new();
        let mut buf = header(len);
        let err = codec.decode(&mut buf).expect_err("refused");
        assert!(matches!(err, FrameError::Oversize { len: l, ceiling } if l == len && ceiling == HELLO_FRAME_MAX), "{err}");
        // Refused at the header: nothing near the declared length was reserved.
        assert!(buf.capacity() < 64, "capacity {}", buf.capacity());
    }
}

#[test]
fn each_ceiling_holds_exactly_and_refuses_one_past_it() {
    for (ceiling, set) in [(HELLO_FRAME_MAX, None), (FRAME_DEFAULT, Some(FRAME_DEFAULT))] {
        let mut codec = FrameCodec::new();
        // The hello comes first on a session, under its own ceiling.
        let mut buf = BytesMut::from(wire(&[b"hello"]).as_slice());
        assert!(codec.decode(&mut buf).expect("hello").is_some());
        if let Some(ceiling) = set {
            codec.set_ceiling(ceiling);
        }
        let mut buf = BytesMut::from(&ceiling.to_be_bytes()[..]);
        buf.extend_from_slice(&vec![1u8; ceiling as usize]);
        let frame = codec.decode(&mut buf).expect("at the ceiling").expect("whole");
        assert_eq!(frame.len(), ceiling as usize);
        let err = codec.decode(&mut BytesMut::from(&(ceiling + 1).to_be_bytes()[..])).expect_err("past it");
        assert!(matches!(err, FrameError::Oversize { .. }), "{err}");
    }
}

#[test]
fn a_frame_to_send_must_hold_something_and_fit_the_ceiling() {
    let mut codec = FrameCodec::new();
    let mut out = BytesMut::new();
    let err = codec.encode(Bytes::new(), &mut out).expect_err("empty");
    assert!(matches!(err, FrameError::ZeroLength), "{err}");
    let big = Bytes::from(vec![0u8; HELLO_FRAME_MAX as usize + 1]);
    let err = codec.encode(big, &mut out).expect_err("too large");
    assert!(matches!(err, FrameError::TooLarge { .. }), "{err}");
    assert!(matches!(err.transport(), TransportError::ResultTooLarge { limit } if limit == u64::from(HELLO_FRAME_MAX)));
    assert!(out.is_empty(), "a refused frame writes nothing, the preface included");
}

#[test]
fn the_codec_is_the_bitcode_the_lock_resolves() {
    let lock = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.lock"))
        .expect("the workspace's Cargo.lock");
    let version = lock
        .split("[[package]]")
        .find(|block| block.contains("name = \"bitcode\""))
        .and_then(|block| block.lines().find_map(|l| l.strip_prefix("version = \"")))
        .and_then(|v| v.strip_suffix('"'))
        .expect("bitcode in the lock");
    assert_eq!(CODEC_ID, format!("bitcode-{version}"));
    assert!(check_codec(CODEC_ID).is_ok());
    for other in ["bitcode-0.6.9+zstd", "bitcode-0.6.8", ""] {
        let err = check_codec(other).expect_err("refused");
        assert_eq!(err.transport().kind(), "daemon::codec_mismatch");
    }
}

/// One variant more than EmergencyKind, in the same four-bit packing.
#[allow(dead_code)]
#[derive(bitcode::Encode)]
enum TenKinds {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
}

#[test]
fn a_bounded_decode_refuses_trailing_bytes_and_unknown_variants() {
    let id = [3u8; 16];
    let mut bytes = bitcode::encode(&id);
    assert_eq!(decode_payload::<[u8; 16]>(&bytes).expect("whole"), id);
    bytes.push(0);
    assert!(matches!(decode_payload::<[u8; 16]>(&bytes), Err(FrameError::Decode)));
    assert_eq!(decode_payload::<Lane>(&bitcode::encode(&Lane::Stateful)).expect("a lane"), Lane::Stateful);
    // A Lane's two variants fill every index its encoding can hold. An
    // EmergencyKind's nine pack into four bits as ten do, so a ten-variant
    // enum's last encodes an index that names none of the nine.
    let kind = EmergencyKind::Critical;
    assert_eq!(decode_payload::<EmergencyKind>(&bitcode::encode(&kind)).expect("a kind"), kind);
    let unknown = bitcode::encode(&TenKinds::J);
    assert!(matches!(decode_payload::<EmergencyKind>(&unknown), Err(FrameError::Decode)));
}

#[tokio::test]
async fn the_adapter_carries_frames_over_a_byte_stream_a_byte_at_a_time() {
    // A one-byte duplex hands every read a single byte.
    let (near, far) = tokio::io::duplex(1);
    let mut sender = framed(near);
    let mut receiver = framed(far);
    let sent = vec![payload(&[1u8; 16]), payload(&[2u8; 16])];
    let send = tokio::spawn(async move {
        for frame in sent {
            sender.send(frame).await.expect("sent");
        }
    });
    let mut got = Vec::new();
    while got.len() < 2 {
        let frame = receiver.next().await.expect("a frame").expect("framed");
        got.push(decode_payload::<[u8; 16]>(&frame).expect("the bytes"));
    }
    send.await.expect("sender");
    assert_eq!(got, vec![[1u8; 16], [2u8; 16]]);
}

/// A small deterministic generator, so a property case reruns from its seed.
struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// A value below `n`.
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// The seeds the property cases run from, kept so a failure reruns exactly.
const SEEDS: [u64; 8] = [
    1,
    2,
    3,
    0x9e37_79b9_7f4a_7c15,
    0xd1b5_4a32_d192_ed03,
    0x2545_f491_4f6c_dd1d,
    0x5851_f42d_4c95_7f2d,
    0xbf58_476d_1ce4_e5b9,
];

#[test]
fn any_split_of_a_valid_stream_yields_the_same_frames() {
    for seed in SEEDS {
        let mut rng = XorShift(seed);
        let frames: Vec<Vec<u8>> = (0..1 + rng.below(8))
            .map(|_| (0..1 + rng.below(512)).map(|_| rng.next() as u8).collect())
            .collect();
        let bytes = wire(&frames.iter().map(Vec::as_slice).collect::<Vec<_>>());
        let mut codec = FrameCodec::new();
        let mut buf = BytesMut::new();
        let mut got = Vec::new();
        let mut at = 0;
        while at < bytes.len() {
            let end = (at + 1 + rng.below(64) as usize).min(bytes.len());
            buf.extend_from_slice(&bytes[at..end]);
            at = end;
            while let Some(frame) = codec.decode(&mut buf).expect("a valid stream") {
                got.push(frame.to_vec());
            }
        }
        assert!(codec.decode_eof(&mut buf).expect("a clean end").is_none(), "seed {seed:#x}");
        assert_eq!(got, frames, "seed {seed:#x}");
    }
}

#[test]
fn random_frames_never_pass_the_ceiling_or_reserve_past_it() {
    for seed in SEEDS {
        let mut rng = XorShift(seed);
        let mut bytes = PREFACE.to_vec();
        for _ in 0..1 + rng.below(16) {
            // Lengths near the ceiling and zero turn up as often as small ones.
            let len = match rng.below(4) {
                0 => 0,
                1 => HELLO_FRAME_MAX - 2 + rng.below(4) as u32,
                _ => rng.below(256) as u32,
            };
            bytes.extend_from_slice(&len.to_be_bytes());
            let body = len.min(4096) as usize - rng.below(2) as usize * (len.min(4096) as usize / 2);
            bytes.extend((0..body).map(|_| rng.next() as u8));
        }
        let mut codec = FrameCodec::new();
        let mut buf = BytesMut::from(bytes.as_slice());
        let ended = loop {
            match codec.decode(&mut buf) {
                Ok(Some(frame)) => assert!(frame.len() <= HELLO_FRAME_MAX as usize, "seed {seed:#x}"),
                Ok(None) => break codec.decode_eof(&mut buf),
                Err(e) => break Err(e),
            }
            let bound = bytes.len() + HELLO_FRAME_MAX as usize;
            assert!(buf.capacity() <= bound, "seed {seed:#x}: capacity {}", buf.capacity());
        };
        if let Err(e) = ended {
            assert!(
                matches!(e, FrameError::ZeroLength | FrameError::Oversize { .. } | FrameError::Truncated { .. }),
                "seed {seed:#x}: {e}",
            );
        }
    }
}

/// The payloads DecodeBudget decodes: small encodings of values far larger
/// once decoded, one past each of the contract's decoded budgets, and one deep
/// enough to exhaust a recursive decoder's stack. They are encoded here from
/// real values and never decoded; the pinned bitcode makes them the same
/// bytes everywhere, so their lengths and hashes retain them.
fn budget_vectors() -> BTreeMap<String, Vec<u8>> {
    /// The contract's decoded collection budget.
    const ELEMENTS: usize = 100_000;
    /// The contract's decoded depth budget.
    const DEPTH: usize = 64;
    let nested = |depth: usize| {
        let mut value = NuVal::Nothing;
        for _ in 0..depth {
            value = NuVal::List(vec![value]);
        }
        value
    };
    let vectors = [
        ("list_past_elements", NuVal::List(vec![NuVal::Nothing; ELEMENTS + 1])),
        ("record_past_elements", NuVal::Record(vec![(String::new(), NuVal::Nothing); ELEMENTS + 1])),
        ("list_past_depth", nested(DEPTH + 1)),
        ("list_deep", nested(10_000)),
    ];
    vectors.into_iter().map(|(name, value)| (name.to_string(), bitcode::encode(&value))).collect()
}

#[test]
fn the_budget_vectors_are_retained_small_and_never_decoded_here() {
    // A deep value's encoding and drop recurse, so they run on a large stack.
    let vectors = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(budget_vectors)
        .expect("a thread")
        .join()
        .expect("the vectors");
    let pinned = NuVal::List(
        vectors
            .iter()
            .map(|(name, bytes)| {
                use sha2::{Digest, Sha256};
                let sha256: String = Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect();
                golden::row([
                    ("name", NuVal::String(name.clone())),
                    ("bytes", NuVal::Int(bytes.len() as i64)),
                    ("sha256", NuVal::String(sha256)),
                ])
            })
            .collect(),
    );
    let retained = golden::read(BUDGET_VECTORS, "tests/vectors/Budget.nutype", &pinned);
    assert_eq!(retained, pinned, "a vector moved; a deliberate change re-blesses {BUDGET_VECTORS}");
    // Every one fits a single frame, so the frame's byte ceiling alone stops
    // none of them.
    for (name, bytes) in &vectors {
        assert!(bytes.len() <= FRAME_DEFAULT as usize, "{name}: {} bytes", bytes.len());
    }
}

/// The retained budget vectors, each payload's length and SHA-256.
const BUDGET_VECTORS: &str = "tests/vectors/budget.nuon";

/// The wire's schema: every public type bitcode encodes in the protocol and
/// nuval, read from their source as the NUON tables `Schema.nutype` types,
/// with the canonical form its ID hashes.
mod schema {
    use std::collections::BTreeMap;
    use std::path::Path;

    use nuval::{NuVal, NuonStyle};
    use sha2::{Digest, Sha256};
    use syn::punctuated::Punctuated;
    use syn::{Meta, Token};

    use super::golden::row;

    /// The checked-in schema, under the protocol crate.
    pub const FILE: &str = "schema/v1.nuon";

    /// The schema's model, beside it.
    pub const MODEL: &str = "schema/Schema.nutype";

    /// The crates whose source declares the wire's own types, by directory.
    const CRATES: [&str; 2] = ["protocol", "nuval"];

    /// Common's datum, the workspace's sibling, whose types the wire carries
    /// where a field names them.
    const DATUM: &str = "../../../common/crates/sourcetrait_datum/src";

    /// A field: its name, a tuple's by position, and its type as written.
    type Field = (String, String);

    /// What the source declares under a name.
    #[derive(Clone)]
    enum Shape {
        Struct { generics: Vec<String>, fields: Vec<Field> },
        Enum { generics: Vec<String>, variants: Vec<(String, Vec<Field>)> },
        Alias { generics: Vec<String>, ty: String },
    }

    /// A schema's records by name.
    type Records = BTreeMap<String, Shape>;

    /// The schema as the source declares it: each type, and each alias and
    /// datum type the types' fields reach, as `Schema.nutype`'s tables.
    pub fn generate() -> NuVal {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut types = Records::new();
        let mut reachable = Records::new();
        for name in CRATES {
            let src = manifest.join("..").join(name).join("src");
            walk_file(&src.join("lib.rs"), &src, &mut types, &mut reachable);
        }
        let mut datum = Records::new();
        let src = manifest.join(DATUM);
        walk_file(&src.join("lib.rs"), &src, &mut datum, &mut reachable);
        for (name, shape) in datum {
            record(&mut reachable, name, shape);
        }
        let mut aliases = Records::new();
        for (name, shape) in reach(&types, &reachable) {
            match shape {
                Shape::Alias { .. } => record(&mut aliases, name, shape),
                _ => record(&mut types, name, shape),
            }
        }
        tables(&types, &aliases)
    }

    /// The four tables: aliases, types, enums' variants, and every field.
    fn tables(types: &Records, aliases: &Records) -> NuVal {
        let text = |s: &str| NuVal::String(s.to_string());
        let alias_rows = aliases
            .iter()
            .filter_map(|(name, shape)| match shape {
                Shape::Alias { generics, ty } => {
                    Some(row([("name", text(name)), ("generics", text(&generics.join(","))), ("type", text(ty))]))
                }
                _ => None,
            })
            .collect();
        let field_row = |owner: &str, variant: Option<&str>, index: usize, (name, ty): &Field| {
            row([
                ("owner", text(owner)),
                ("variant", variant.map_or(NuVal::Nothing, text)),
                ("index", NuVal::Int(index as i64)),
                ("name", text(name)),
                ("type", text(ty)),
            ])
        };
        let (mut type_rows, mut variant_rows, mut field_rows) = (Vec::new(), Vec::new(), Vec::new());
        for (name, shape) in types {
            let type_row = |kind: &str, generics: &[String]| {
                row([("name", text(name)), ("kind", text(kind)), ("generics", text(&generics.join(",")))])
            };
            match shape {
                Shape::Struct { generics, fields } => {
                    type_rows.push(type_row("struct", generics));
                    field_rows.extend(fields.iter().enumerate().map(|(i, f)| field_row(name, None, i, f)));
                }
                Shape::Enum { generics, variants } => {
                    type_rows.push(type_row("enum", generics));
                    for (index, (variant, fields)) in variants.iter().enumerate() {
                        variant_rows.push(row([
                            ("owner", text(name)),
                            ("index", NuVal::Int(index as i64)),
                            ("name", text(variant)),
                        ]));
                        field_rows
                            .extend(fields.iter().enumerate().map(|(i, f)| field_row(name, Some(variant), i, f)));
                    }
                }
                Shape::Alias { .. } => {}
            }
        }
        NuVal::Record(vec![
            ("aliases".into(), NuVal::List(alias_rows)),
            ("types".into(), NuVal::List(type_rows)),
            ("variants".into(), NuVal::List(variant_rows)),
            ("fields".into(), NuVal::List(field_rows)),
        ])
    }

    /// The SHA-256 of a schema text's canonical form.
    pub fn id(text: &str) -> [u8; 32] {
        let value = NuVal::from_nuon(text).expect("the schema's NUON");
        let mut id = [0u8; 32];
        id.copy_from_slice(&Sha256::digest(canonical(&value).as_bytes()));
        id
    }

    /// The form the ID hashes: raw NUON with every record's keys sorted, the
    /// text nushell's `to nuon --raw` writes for the same value.
    pub fn canonical(value: &NuVal) -> String {
        sorted(value).to_nuon(NuonStyle::Raw)
    }

    /// The value with every record's keys sorted.
    fn sorted(value: &NuVal) -> NuVal {
        match value {
            NuVal::Record(entries) => {
                let mut entries: Vec<(String, NuVal)> =
                    entries.iter().map(|(key, value)| (key.clone(), sorted(value))).collect();
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                NuVal::Record(entries)
            }
            NuVal::List(items) => NuVal::List(items.iter().map(sorted).collect()),
            other => other.clone(),
        }
    }

    /// The file's items, its child modules' files under `dir`.
    fn walk_file(file: &Path, dir: &Path, types: &mut Records, aliases: &mut Records) {
        let text = std::fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("read {}: {e}", file.display()));
        let parsed =
            syn::parse_file(&text).unwrap_or_else(|e| panic!("parse {}: {e}", file.display()));
        walk(&parsed.items, dir, types, aliases);
    }

    /// Each public wire type and alias, modules followed, test modules skipped.
    fn walk(items: &[syn::Item], dir: &Path, types: &mut Records, aliases: &mut Records) {
        for item in items {
            match item {
                syn::Item::Mod(m) if !is_test(&m.attrs) => {
                    let child = dir.join(m.ident.to_string());
                    match &m.content {
                        Some((_, inner)) => walk(inner, &child, types, aliases),
                        None => {
                            let file = dir.join(format!("{}.rs", m.ident));
                            walk_file(&file, &child, types, aliases);
                        }
                    }
                }
                syn::Item::Struct(s) if is_public(&s.vis) && on_wire(&s.attrs) => {
                    let name = s.ident.to_string();
                    let shape = Shape::Struct { generics: generics(&s.generics), fields: fields(&s.fields, &name) };
                    record(types, name, shape);
                }
                syn::Item::Enum(e) if is_public(&e.vis) && on_wire(&e.attrs) => {
                    let name = e.ident.to_string();
                    let variants = e
                        .variants
                        .iter()
                        .map(|v| {
                            let variant = format!("{name}::{}", v.ident);
                            fixed(&v.attrs, &variant);
                            (v.ident.to_string(), fields(&v.fields, &variant))
                        })
                        .collect();
                    record(types, name, Shape::Enum { generics: generics(&e.generics), variants });
                }
                syn::Item::Type(t) if is_public(&t.vis) => {
                    let shape = Shape::Alias { generics: generics(&t.generics), ty: render(&t.ty) };
                    record(aliases, t.ident.to_string(), shape);
                }
                _ => {}
            }
        }
    }

    /// Records a type or an alias by its name, which the schema keeps unique.
    fn record(records: &mut Records, name: String, shape: Shape) {
        if records.insert(name.clone(), shape).is_some() {
            panic!("{name} is declared twice; the schema names each type once");
        }
    }

    /// The records the types name, followed through what each of those names.
    fn reach(types: &Records, reachable: &Records) -> Records {
        let mut texts: Vec<String> = types.values().flat_map(named).collect();
        let mut reached = Records::new();
        while let Some(text) = texts.pop() {
            for word in text.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
                if let Some(shape) = reachable.get(word)
                    && !types.contains_key(word)
                    && !reached.contains_key(word)
                {
                    texts.extend(named(shape));
                    reached.insert(word.to_string(), shape.clone());
                }
            }
        }
        reached
    }

    /// Every type a record names: an alias's own, a type's fields'.
    fn named(shape: &Shape) -> Vec<String> {
        match shape {
            Shape::Alias { ty, .. } => vec![ty.clone()],
            Shape::Struct { fields, .. } => fields.iter().map(|(_, ty)| ty.clone()).collect(),
            Shape::Enum { variants, .. } => {
                variants.iter().flat_map(|(_, fields)| fields.iter().map(|(_, ty)| ty.clone())).collect()
            }
        }
    }

    /// Whether the item is `pub`, the wire's types all being public.
    fn is_public(vis: &syn::Visibility) -> bool {
        matches!(vis, syn::Visibility::Public(_))
    }

    /// Whether a `cfg` names `test`: such a module never reaches the wire.
    fn is_test(attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|a| a.path().is_ident("cfg") && words(a).iter().any(|w| w == "test"))
    }

    /// An attribute's argument words, punctuation dropped.
    fn words(attr: &syn::Attribute) -> Vec<String> {
        match &attr.meta {
            Meta::List(list) => list
                .tokens
                .to_string()
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .filter(|w| !w.is_empty())
                .map(String::from)
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Whether bitcode's derive encodes the type: `cereal::derived` without
    /// `not(Bitcode)`, or a derive naming bitcode's `Encode`.
    fn on_wire(attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|a| {
            let path = path_text(a.path());
            if path == "cereal::derived" {
                if cereal_listed(a, "has").iter().any(|p| p == "Bitcode") {
                    panic!("a hand-written bitcode impl: its layout isn't its fields'");
                }
                !cereal_listed(a, "not").iter().any(|p| p == "Bitcode")
            } else if path == "derive" {
                let derives = a
                    .parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
                    .expect("a derive's paths");
                derives.iter().any(|p| path_text(p).ends_with("bitcode::Encode"))
            } else {
                false
            }
        })
    }

    /// The names a `cereal::derived` lists under `not(..)` or `has(..)`.
    fn cereal_listed(attr: &syn::Attribute, list: &str) -> Vec<String> {
        let metas = attr
            .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            .expect("cereal's arguments");
        metas
            .iter()
            .filter_map(|m| match m {
                Meta::List(l) if l.path.is_ident(list) => Some(l),
                _ => None,
            })
            .flat_map(|l| {
                l.parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
                    .expect("a cereal list's names")
                    .iter()
                    .map(path_text)
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Refuses a `cfg` or `cfg_attr` on a field or a variant, which would make
    /// the layout depend on the build.
    fn fixed(attrs: &[syn::Attribute], what: &str) {
        if attrs.iter().any(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr")) {
            panic!("{what} carries a cfg: the wire's layout would vary with the build");
        }
    }

    /// Whether bitcode skips the field. Any other bitcode attribute is refused
    /// until the schema records what it does.
    fn skipped(attrs: &[syn::Attribute], what: &str) -> bool {
        attrs.iter().filter(|a| a.path().is_ident("bitcode")).any(|a| match words(a).as_slice() {
            [word] if word == "skip" => true,
            other => panic!("{what}: bitcode attribute {other:?} isn't one the schema records"),
        })
    }

    /// Each field bitcode encodes, in order, by name or by position.
    fn fields(fields: &syn::Fields, owner: &str) -> Vec<Field> {
        fields
            .iter()
            .enumerate()
            .filter_map(|(i, f)| {
                let name = f.ident.as_ref().map_or_else(|| i.to_string(), ToString::to_string);
                let what = format!("{owner}.{name}");
                fixed(&f.attrs, &what);
                (!skipped(&f.attrs, &what)).then(|| (name, render(&f.ty)))
            })
            .collect()
    }

    /// The generic parameters in order: a type's name, a lifetime, a const.
    fn generics(generics: &syn::Generics) -> Vec<String> {
        generics
            .params
            .iter()
            .map(|p| match p {
                syn::GenericParam::Type(t) => t.ident.to_string(),
                syn::GenericParam::Lifetime(l) => format!("'{}", l.lifetime.ident),
                syn::GenericParam::Const(c) => format!("const {}: {}", c.ident, render(&c.ty)),
            })
            .collect()
    }

    /// A type as text in one spacing, whatever the source's whitespace.
    fn render(ty: &syn::Type) -> String {
        match ty {
            syn::Type::Path(p) if p.qself.is_none() => path_text(&p.path),
            syn::Type::Array(a) => format!("[{}; {}]", render(&a.elem), length(&a.len)),
            syn::Type::Slice(s) => format!("[{}]", render(&s.elem)),
            syn::Type::Tuple(t) => {
                let elems: Vec<String> = t.elems.iter().map(render).collect();
                match elems.as_slice() {
                    [one] => format!("({one},)"),
                    _ => format!("({})", elems.join(", ")),
                }
            }
            syn::Type::Reference(r) => {
                let lifetime = r.lifetime.as_ref().map_or_else(String::new, |l| format!("'{} ", l.ident));
                let mutability = if r.mutability.is_some() { "mut " } else { "" };
                format!("&{lifetime}{mutability}{}", render(&r.elem))
            }
            syn::Type::Paren(p) => render(&p.elem),
            syn::Type::Group(g) => render(&g.elem),
            _ => panic!("a field type the schema can't describe: a qualified path, pointer, function, trait, or macro"),
        }
    }

    /// A path as `a::b<C, D>`.
    fn path_text(path: &syn::Path) -> String {
        let segments: Vec<String> = path
            .segments
            .iter()
            .map(|s| match &s.arguments {
                syn::PathArguments::None => s.ident.to_string(),
                syn::PathArguments::AngleBracketed(a) => {
                    let args: Vec<String> = a
                        .args
                        .iter()
                        .map(|g| match g {
                            syn::GenericArgument::Type(t) => render(t),
                            syn::GenericArgument::Lifetime(l) => format!("'{}", l.ident),
                            syn::GenericArgument::Const(e) => length(e),
                            _ => panic!("{}: an associated type or a bound in a field's type", s.ident),
                        })
                        .collect();
                    format!("{}<{}>", s.ident, args.join(", "))
                }
                syn::PathArguments::Parenthesized(_) => panic!("{}: a function trait in a field's type", s.ident),
            })
            .collect();
        let lead = if path.leading_colon.is_some() { "::" } else { "" };
        format!("{lead}{}", segments.join("::"))
    }

    /// An array's length or a const argument: a literal integer alone, so a
    /// constant's value can't change the layout without changing the schema.
    fn length(expr: &syn::Expr) -> String {
        match expr {
            syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(int), .. }) => int.base10_digits().to_string(),
            _ => panic!("a length the schema can't record: write it as a literal integer"),
        }
    }
}

#[test]
fn the_schema_records_every_public_wire_type_as_the_source_declares_it() {
    let generated = schema::generate();
    let file = golden::read(schema::FILE, schema::MODEL, &generated);
    assert_eq!(
        schema::canonical(&file),
        schema::canonical(&generated),
        "a wire type changed; a deliberate change re-blesses {} and SCHEMA_ID",
        schema::FILE,
    );
}

#[test]
fn the_schema_id_is_the_hash_of_the_schema_s_canonical_form() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(schema::FILE);
    let text = std::fs::read_to_string(&path).expect("the schema");
    let id = schema::id(&text);
    // A checkout's line endings, and the file's layout, change its bytes but
    // never its ID.
    assert_eq!(schema::id(&text.replace('\n', "\r\n")), id);
    let file = NuVal::from_nuon(&text).expect("the schema's NUON");
    assert_eq!(schema::id(&file.to_nuon(nuval::NuonStyle::Raw)), id);
    let hex: String = id.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(SCHEMA_ID, id, "the checked-in schema's ID is {hex}");
}
