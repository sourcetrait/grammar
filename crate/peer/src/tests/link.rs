//! The link's verifier, its codec's frames, its send's checks, and its paths.
use crate::rv;
use crate::verify::PublicKeyVerifier;
use rustls::client::danger::ServerCertVerifier;
use rustls::server::danger::ClientCertVerifier;

/// The known public key's leaf passes on either side, any other leaf fails.
#[test]
fn public_key_verifier_accepts_only_the_matching_leaf() {
    let key_bytes = vec![1u8, 2, 3, 4, 5];
    let other_bytes = vec![9u8, 9, 9];
    let known = rv::CertificateDer::from(key_bytes.clone());
    let verifier = PublicKeyVerifier::new(known);

    let same = rv::CertificateDer::from(key_bytes);
    let different = rv::CertificateDer::from(other_bytes);
    let now = rustls::pki_types::UnixTime::now();
    let name = rustls::pki_types::ServerName::try_from("127.0.0.1").unwrap();

    assert!(ServerCertVerifier::verify_server_cert(&verifier, &same, &[], &name, &[], now).is_ok());
    assert!(
        ServerCertVerifier::verify_server_cert(&verifier, &different, &[], &name, &[], now).is_err()
    );

    assert!(ClientCertVerifier::verify_client_cert(&verifier, &same, &[], now).is_ok());
    assert!(ClientCertVerifier::verify_client_cert(&verifier, &different, &[], now).is_err());
}

/// What the codec encodes through zstd and bitcode, it decodes back unchanged.
#[test]
fn bitcode_codec_round_trips_a_message_through_zstd() {
    use crate::tku::{BytesMut, Decoder, Encoder};
    use crate::{BitcodeCodec, InitiatorToAcceptor, RemoteStream};

    let msg = InitiatorToAcceptor::Hello {
        mcp_nom: "abc123XYZ".to_string(),
        stream: RemoteStream::File,
    };
    let mut dst = BytesMut::new();
    let mut codec: BitcodeCodec<InitiatorToAcceptor> = BitcodeCodec::new();
    codec.encode(msg.clone(), &mut dst).expect("encode");
    let got = codec.decode(&mut dst).expect("decode ok").expect("a full frame");
    assert_eq!(got, msg, "encode -> zstd -> frame -> decode must round-trip");
}

/// A Deliver and a chunk of real bytes round-trip through both wire enums.
#[test]
fn bitcode_codec_round_trips_the_send_frames() {
    use crate::tku::{BytesMut, Decoder, Encoder};
    use crate::{AcceptorToInitiator, BitcodeCodec, FileFrame, InitiatorToAcceptor, MsgFrame};

    fn roundtrip_init(msg: InitiatorToAcceptor) {
        let mut dst = BytesMut::new();
        let mut codec: BitcodeCodec<InitiatorToAcceptor> = BitcodeCodec::new();
        codec.encode(msg.clone(), &mut dst).expect("encode");
        let got = codec.decode(&mut dst).expect("decode ok").expect("a full frame");
        assert_eq!(got, msg);
    }
    fn roundtrip_acc(msg: AcceptorToInitiator) {
        let mut dst = BytesMut::new();
        let mut codec: BitcodeCodec<AcceptorToInitiator> = BitcodeCodec::new();
        codec.encode(msg.clone(), &mut dst).expect("encode");
        let got = codec.decode(&mut dst).expect("decode ok").expect("a full frame");
        assert_eq!(got, msg);
    }

    let deliver = MsgFrame::Deliver {
        id: "id1".to_string(),
        model: "proj/thing/Changed".to_string(),
        event_nuon: "{a: 1, b: \"x\"}".to_string(),
        files: vec!["out.txt".to_string()],
    };
    let chunk = FileFrame::Chunk {
        id: "id1".to_string(),
        dest: "out.txt".to_string(),
        seq: 3,
        bytes: (0u8..=255).cycle().take(4096).collect(),
        last: true,
    };

    roundtrip_init(InitiatorToAcceptor::Msg(deliver.clone()));
    roundtrip_init(InitiatorToAcceptor::File(chunk.clone()));
    roundtrip_acc(AcceptorToInitiator::Msg(deliver));
    roundtrip_acc(AcceptorToInitiator::File(chunk));
}

/// A frame inflating past the cap is refused, however small it is on the wire.
#[test]
fn bitcode_codec_refuses_a_frame_inflating_past_its_cap() {
    use crate::codec::REMOTE_INFLATED_MAX;
    use crate::tku::{Bytes, BytesMut, Decoder, Encoder, LengthDelimitedCodec};
    use crate::{BitcodeCodec, InitiatorToAcceptor};

    let zeros = vec![0u8; REMOTE_INFLATED_MAX as usize + 1];
    let compressed = zstd::encode_all(zeros.as_slice(), 19).expect("compressed");
    assert!(compressed.len() < 4096, "a bomb of {} bytes", compressed.len());
    let mut dst = BytesMut::new();
    LengthDelimitedCodec::new().encode(Bytes::from(compressed), &mut dst).expect("framed");
    let mut codec: BitcodeCodec<InitiatorToAcceptor> = BitcodeCodec::new();
    let err = codec.decode(&mut dst).expect_err("refused");
    assert!(err.to_string().contains("inflates past"), "{err}");
}

/// A delivery past the decode budget is refused before its files' names allocate.
#[test]
fn bitcode_codec_refuses_a_frame_past_its_decode_budget() {
    use crate::tku::{BytesMut, Decoder, Encoder};
    use crate::{BitcodeCodec, DECODE_BUDGET_DEFAULT, InitiatorToAcceptor, MsgFrame};

    let names = DECODE_BUDGET_DEFAULT.elements as usize + 1;
    let deliver = MsgFrame::Deliver {
        id: "id1".to_string(),
        model: "proj/thing/Changed".to_string(),
        event_nuon: "{}".to_string(),
        files: vec![String::new(); names],
    };
    let mut dst = BytesMut::new();
    let mut codec: BitcodeCodec<InitiatorToAcceptor> = BitcodeCodec::new();
    codec.encode(InitiatorToAcceptor::Msg(deliver), &mut dst).expect("encode");
    let err = codec.decode(&mut dst).expect_err("refused");
    assert!(err.to_string().contains("elements bound"), "{err}");
}

/// Both wire enums encode a shared frame alike, so one check serves both roles.
#[test]
fn both_wire_enums_encode_the_shared_frames_alike() {
    use crate::{AcceptorToInitiator, FileFrame, InitiatorToAcceptor, MsgFrame};

    let deliver = MsgFrame::Deliver {
        id: "id1".to_string(),
        model: "proj/thing/Changed".to_string(),
        event_nuon: "{a: 1}".to_string(),
        files: vec!["out.txt".to_string(), "b".to_string()],
    };
    let chunk = FileFrame::Chunk {
        id: "id1".to_string(),
        dest: "out.txt".to_string(),
        seq: 70_000,
        bytes: vec![1, 2, 3],
        last: false,
    };
    assert_eq!(
        bitcode::encode(&InitiatorToAcceptor::Msg(deliver.clone())),
        bitcode::encode(&AcceptorToInitiator::Msg(deliver))
    );
    assert_eq!(
        bitcode::encode(&InitiatorToAcceptor::File(chunk.clone())),
        bitcode::encode(&AcceptorToInitiator::File(chunk))
    );
}

/// A send with a frame past its peer's cap is refused before any frame queues.
#[test]
fn a_send_past_the_inflated_cap_is_refused_before_it_is_queued() {
    use crate::{codec::REMOTE_INFLATED_MAX, link::push_frames};
    use crate::tk;

    let (msg_tx, mut msg_rx) = tk::unbounded_channel();
    let (file_tx, mut file_rx) = tk::unbounded_channel();
    let model = "m".repeat(REMOTE_INFLATED_MAX as usize + 1);
    let payloads = vec![("out.txt".to_string(), vec![7u8; 1000])];
    let err = push_frames(
        &msg_tx,
        &file_tx,
        "id1",
        model,
        "{}".to_string(),
        payloads,
        vec!["out.txt".to_string()],
    )
    .expect_err("refused");
    assert!(err.contains("byte cap"), "{err}");
    assert!(msg_rx.try_recv().is_err(), "no Deliver queued");
    assert!(file_rx.try_recv().is_err(), "no chunk queued");
}

/// A send past the decode budget's elements is refused before any frame queues.
#[test]
fn a_send_past_the_decode_budget_is_refused_before_it_is_queued() {
    use crate::link::push_frames;
    use crate::{DECODE_BUDGET_DEFAULT, tk};

    let (msg_tx, mut msg_rx) = tk::unbounded_channel();
    let (file_tx, mut file_rx) = tk::unbounded_channel();
    let n = DECODE_BUDGET_DEFAULT.elements as usize + 1;
    let payloads: Vec<(String, Vec<u8>)> = (0..n).map(|i| (format!("f{i}"), Vec::new())).collect();
    let dests = payloads.iter().map(|(dest, _)| dest.clone()).collect();
    let err = push_frames(
        &msg_tx,
        &file_tx,
        "id1",
        "proj/thing/Changed".to_string(),
        "{}".to_string(),
        payloads,
        dests,
    )
    .expect_err("refused");
    assert!(err.contains("elements bound"), "{err}");
    assert!(msg_rx.try_recv().is_err(), "no Deliver queued");
    assert!(file_rx.try_recv().is_err(), "no chunk queued");
}

/// A frame the check passes crosses the codec whole, incompressible at the cap.
#[test]
fn a_frame_the_check_passes_crosses_the_codec_whole() {
    use crate::codec::REMOTE_INFLATED_MAX;
    use crate::tku::{BytesMut, Decoder, Encoder};
    use crate::{BitcodeCodec, FileFrame, InitiatorToAcceptor, check_outbound};

    let mut state = 0x9e37_79b9_7f4a_7c15u64;
    let bytes: Vec<u8> = (0..REMOTE_INFLATED_MAX as usize - 64)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 24) as u8
        })
        .collect();
    let frame = InitiatorToAcceptor::File(FileFrame::Chunk {
        id: "id1".to_string(),
        dest: "out.bin".to_string(),
        seq: 0,
        bytes,
        last: true,
    });
    check_outbound(&frame).expect("within the cap and the budget");
    let mut dst = BytesMut::new();
    let mut codec: BitcodeCodec<InitiatorToAcceptor> = BitcodeCodec::new();
    codec.encode(frame.clone(), &mut dst).expect("encoded");
    assert!(dst.len() > REMOTE_INFLATED_MAX as usize, "{} bytes on the wire", dst.len());
    let got = codec.decode(&mut dst).expect("decode ok").expect("a full frame");
    assert_eq!(got, frame);
}

/// A relative dest passes; an empty, an absolute, or an escaping one fails.
#[test]
fn safe_dest_rejects_escapes() {
    use crate::safe_dest;
    assert!(safe_dest("out.txt"));
    assert!(safe_dest("sub/dir/out.txt"));
    assert!(!safe_dest(""));
    assert!(!safe_dest("/abs/path"));
    assert!(!safe_dest("../escape"));
    assert!(!safe_dest("sub/../../escape"));
}

/// A delivery's file lands beneath its peer's inbox directory, or nowhere.
#[test]
fn a_delivery_path_stays_beneath_the_inbox() {
    use crate::link::delivery_path;
    let inbox = std::path::Path::new("/srv/inbox");
    assert_eq!(
        delivery_path(inbox, "peernom0001", "eventid0001", "sub/out.txt"),
        Some(inbox.join("peernom0001").join("eventid0001").join("sub/out.txt")),
    );
    let escapes = [
        ("..", "eventid0001", "out.txt"),
        ("/srv/other", "eventid0001", "out.txt"),
        ("peernom0001", "../../escape", "out.txt"),
        ("peernom0001", "/srv/other/.ssh", "authorized_keys"),
        ("peernom0001", "", "out.txt"),
        ("peernom0001", "event.id", "out.txt"),
        ("peernom0001", "eventid0001", "../out.txt"),
        ("peernom0001", "eventid0001", "/abs"),
    ];
    for (nom, id, dest) in escapes {
        assert_eq!(delivery_path(inbox, nom, id, dest), None, "{nom} {id} {dest}");
    }
}

/// An old connection's end leaves its replacement, though alias and peer match.
#[test]
fn a_replaced_connections_end_leaves_its_replacement() {
    use crate::link::{EndedLink, ended_link};
    assert_eq!(ended_link(Some(1), 1), EndedLink::Current, "the alias's own connection ended");
    assert_eq!(
        ended_link(Some(2), 1),
        EndedLink::Replaced,
        "the first connection's late end, the alias redialed to the same peer",
    );
    assert_eq!(ended_link(None, 1), EndedLink::Closed, "a close took the entry first");
}
