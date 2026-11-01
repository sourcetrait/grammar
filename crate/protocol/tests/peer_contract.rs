//! The remote Channel's portable data: the seven commands' typed results held to
//! the catalog, the lifecycle and delivery events in their 0.0.0-9 shapes, a
//! relayed packet held to a relay's rules, a send's dests held plain, and
//! golden vectors.

use std::collections::{BTreeMap, BTreeSet};

use sourcetrait_grammar_protocol::*;

#[path = "support/golden.rs"]
mod golden;

/// The golden vectors' file, every vector's bitcode bytes in hex.
const GOLDEN: &str = "tests/vectors/peer.nuon";

const NOM: &str = "peernom0001";

fn text(s: &str) -> NuVal {
    NuVal::String(s.to_string())
}

fn record(fields: &[(&str, NuVal)]) -> NuVal {
    NuVal::Record(fields.iter().map(|(key, value)| (key.to_string(), value.clone())).collect())
}

fn row() -> RemoteChannelRow {
    RemoteChannelRow {
        alias: "lab/peer".to_string(),
        address: Some("127.0.0.1:7000".to_string()),
        listen: None,
        cert: Some("peer.pem".to_string()),
        state: RemoteLinkState::Open,
        endpoint: Some("127.0.0.1:7000".to_string()),
        remote_mcp_nom: Some(NOM.to_string()),
    }
}

fn failure() -> RemoteFailure {
    RemoteFailure { kind: "listen".to_string(), message: "address in use".to_string() }
}

fn packet(model: &str, event: NuVal) -> RemotePacket {
    RemotePacket { delivery: "eventid0001".to_string(), origin: NOM.to_string(), model: model.to_string(), event }
}

/// Every result shape a remote command answers.
fn results() -> Vec<(&'static str, RemoteResult)> {
    vec![
        ("rows", RemoteResult::Rows(vec![row()])),
        ("row", RemoteResult::Row(row())),
        ("listening", RemoteResult::Listening(RemoteListenResult { endpoint: "127.0.0.1:7000".to_string() })),
        ("done", RemoteResult::Done),
    ]
}

/// Every lifecycle and delivery event, each shape of `Disconnected` among them.
fn events() -> Vec<(&'static str, RemoteChannelEvent)> {
    use RemoteChannelEvent as E;
    vec![
        ("connected", E::Connected { remote: "lab/peer".to_string(), mcp_nom: NOM.to_string() }),
        ("disconnected/closed", E::Disconnected { remote: "lab/peer".to_string(), mcp_nom: None, error: None }),
        ("disconnected/ended", E::Disconnected { remote: "lab/peer".to_string(), mcp_nom: Some(NOM.to_string()), error: None }),
        ("disconnected/failed", E::Disconnected { remote: "lab/peer".to_string(), mcp_nom: None, error: Some(failure()) }),
        ("sent", E::Sent { id: "eventid0001".to_string(), mcp_nom: NOM.to_string() }),
        ("unsent", E::Unsent { id: "eventid0001".to_string(), mcp_nom: NOM.to_string(), error: failure() }),
    ]
}

/// A remote send as a worker prepares it, its event's NUON and one file.
fn prepared() -> PreparedSend {
    PreparedSend {
        mcp_nom: NOM.to_string(),
        model: "peer/Report".to_string(),
        event_nuon: "{n: 1}".to_string(),
        files: vec![SendFile { dest: "out/report.nuon".to_string(), bytes: b"{ok: true}".to_vec() }],
    }
}

#[test]
fn each_remote_command_has_one_result_shape_its_catalog_entry_declares() {
    let catalog: Vec<_> = COMMANDS.iter().filter(|spec| spec.words().starts_with("remote channel")).collect();
    assert_eq!(catalog.iter().map(|spec| spec.kind).collect::<Vec<_>>(), REMOTE_COMMANDS);
    for spec in &catalog {
        let fits: Vec<_> = results().into_iter().filter(|(_, result)| result.answers(spec.kind)).collect();
        assert_eq!(fits.len(), 1, "{}", spec.words());
        let declared = match fits[0].1 {
            RemoteResult::Rows(_) => "table",
            RemoteResult::Row(_) | RemoteResult::Listening(_) => "record",
            RemoteResult::Done => "nothing",
        };
        assert_eq!(spec.result, declared, "{}", spec.words());
    }
    for spec in COMMANDS.iter().filter(|spec| !REMOTE_COMMANDS.contains(&spec.kind)) {
        assert!(results().iter().all(|(_, result)| !result.answers(spec.kind)), "{}", spec.words());
    }
}

#[test]
fn a_row_and_a_listen_render_as_the_tools_always_have() {
    let none = NuVal::Nothing;
    assert_eq!(
        nuval::to_nuval(&row()).expect("a value"),
        record(&[
            ("alias", text("lab/peer")),
            ("address", text("127.0.0.1:7000")),
            ("listen", none),
            ("cert", text("peer.pem")),
            ("state", text("open")),
            ("endpoint", text("127.0.0.1:7000")),
            ("remote_mcp_nom", text(NOM)),
        ])
    );
    assert_eq!(
        nuval::to_nuval(&RemoteListenResult { endpoint: "127.0.0.1:7000".to_string() }).expect("a value"),
        record(&[("endpoint", text("127.0.0.1:7000"))])
    );
}

#[test]
fn each_event_carries_its_packets_0_0_0_9_record_and_model() {
    let error = record(&[("kind", text("listen")), ("message", text("address in use"))]);
    let shapes = [
        (MODEL_CONNECTED, record(&[("remote", text("lab/peer")), ("mcp_nom", text(NOM))])),
        (MODEL_DISCONNECTED, record(&[("remote", text("lab/peer"))])),
        (MODEL_DISCONNECTED, record(&[("remote", text("lab/peer")), ("mcp_nom", text(NOM))])),
        (MODEL_DISCONNECTED, record(&[("remote", text("lab/peer")), ("error", error.clone())])),
        (MODEL_SENT, record(&[("id", text("eventid0001")), ("mcp_nom", text(NOM))])),
        (MODEL_UNSENT, record(&[("id", text("eventid0001")), ("mcp_nom", text(NOM)), ("error", error)])),
    ];
    for ((name, event), (model, shape)) in events().into_iter().zip(shapes) {
        assert_eq!(event.model(), model, "{name}");
        assert_eq!(event.event(), shape, "{name}: an absent key left out, never null");
    }
}

#[test]
fn a_peer_id_is_base62_alone() {
    let max = "a".repeat(PEER_ID_MAX);
    for good in [NOM, "eventid0001", "7", max.as_str()] {
        assert!(valid_peer_id(good), "{good}");
    }
    let long = "a".repeat(PEER_ID_MAX + 1);
    for bad in ["", "..", "../x", "/srv/other", "a/b", "a.b", "a b", "a\\b", "nöm", "id\0", long.as_str()] {
        assert!(!valid_peer_id(bad), "{bad:?}");
    }
}

#[test]
fn a_send_s_dest_is_a_plain_relative_path() {
    for good in ["out.txt", "sub/dir/out.txt", EVENT_SPILL_DEST] {
        assert!(safe_dest(good), "{good}");
    }
    for bad in ["", "/abs/path", "../escape", "sub/../../escape", "./here"] {
        assert!(!safe_dest(bad), "{bad:?}");
    }
}

#[test]
fn a_relayed_packet_is_held_to_a_relays_rules() {
    let ok = packet("peer/Checkpoint", record(&[("n", NuVal::Int(1))]));
    ok.check().expect("an ordinary relay");
    assert_eq!(ok.packet_from(), format!("remote/channel/{NOM}"));
    let refusals = [
        ("a host model", packet("mcp/channel/Open", NuVal::Nothing)),
        ("a host's remote report", packet(MODEL_SENT, NuVal::Nothing)),
        ("no model", packet("", NuVal::Nothing)),
        ("an origin that escapes", RemotePacket { origin: "../..".to_string(), ..packet("peer/Note", NuVal::Nothing) }),
        ("a delivery that escapes", RemotePacket { delivery: "/srv/other".to_string(), ..packet("peer/Note", NuVal::Nothing) }),
    ];
    for (name, packet) in refusals {
        assert_eq!(packet.check().expect_err(name).kind(), "daemon::invalid_frame", "{name}");
    }
}

/// Every peer message's bitcode.
fn vectors() -> BTreeMap<String, golden::Vector> {
    let mut out = BTreeMap::new();
    for (name, result) in results() {
        out.insert(format!("result/{name}"), golden::vector(&result));
    }
    for (name, event) in events() {
        out.insert(format!("event/{name}"), golden::vector(&event));
    }
    out.insert("packet".to_string(), golden::vector(&packet("peer/Checkpoint", record(&[("event_bytes", NuVal::Int(641))]))));
    out.insert("send/prepared".to_string(), golden::vector(&prepared()));
    out
}

#[test]
fn each_message_has_its_golden_vector_and_round_trips() {
    let actual = golden::hex_table(vectors());
    let golden = golden::read(GOLDEN, golden::VECTORS_MODEL, &actual);
    assert_eq!(golden, actual, "a peer message's bytes changed; a deliberate change re-blesses {GOLDEN}");
    let covered: BTreeSet<_> = results()
        .iter()
        .map(|(_, result)| match result {
            RemoteResult::Rows(_) => "rows",
            RemoteResult::Row(_) => "row",
            RemoteResult::Listening(_) => "listening",
            RemoteResult::Done => "done",
        })
        .collect();
    assert_eq!(covered.len(), 4);
    for (name, result) in results() {
        let back: RemoteResult = decode_payload(&bitcode::encode(&result), DECODE_BUDGET_DEFAULT).expect(name);
        assert_eq!(back, result, "{name}");
    }
    for (name, event) in events() {
        let back: RemoteChannelEvent = decode_payload(&bitcode::encode(&event), DECODE_BUDGET_DEFAULT).expect(name);
        assert_eq!(back, event, "{name}");
    }
    let back: PreparedSend = decode_payload(&bitcode::encode(&prepared()), DECODE_BUDGET_DEFAULT).expect("a send");
    assert_eq!((back.files.len(), back.files[0].bytes.as_slice()), (1, &b"{ok: true}"[..]));
    for (name, vector) in vectors() {
        assert!(vector.bytes.len() <= FRAME_DEFAULT as usize, "{name} fits a frame");
    }
}
