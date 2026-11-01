//! A session's handshake: each mismatch refused, limits negotiated down, the
//! MCP's checks of the reply, the instance's names, the session's deadlines,
//! and a golden vector of each hello and reply.

use std::collections::BTreeMap;
use std::time::Duration;

use sourcetrait_grammar_protocol::*;

#[path = "support/golden.rs"]
mod golden;

/// The golden vectors' file, every vector's bitcode bytes in hex.
const GOLDEN: &str = "tests/vectors/hello.nuon";

const UID: u32 = 4242;
const PID: u32 = 31337;
const NOM: &str = "nom0fixture";

fn instance() -> Instance {
    Instance::new("fixture", "default").expect("a valid instance")
}

fn worker() -> WorkerProfile {
    WorkerProfile {
        instance: instance(),
        uid: UID,
        pid: PID,
        nom: NOM.to_string(),
        limits: LIMITS_DEFAULT,
    }
}

fn hello() -> ClientHello {
    ClientHello::new(instance(), LIMITS_DEFAULT)
}

/// The worker's acceptance of the fixture hello.
fn reply() -> WorkerHello {
    match worker().reply(&hello()) {
        HelloReply::Accepted(accepted) => *accepted,
        refused => panic!("the fixture hello refused: {refused:?}"),
    }
}

/// Bounds past any ceiling, every one.
const GREEDY: Limits = Limits {
    frame: u32::MAX,
    decode_heap: u64::MAX,
    calls: u32::MAX,
    control: u32::MAX,
};

#[test]
fn a_valid_hello_is_accepted_under_the_worker_s_identity() {
    let accepted = reply();
    assert_eq!(accepted.instance, instance());
    assert_eq!((accepted.uid, accepted.pid, accepted.nom.as_str()), (UID, PID, NOM));
    assert_eq!(accepted.effective, LIMITS_DEFAULT);
    assert_eq!((accepted.version, accepted.schema, accepted.codec.as_str()), (PROTOCOL_VERSION, SCHEMA_ID, CODEC_ID));
}

#[test]
fn each_mismatch_is_refused() {
    let named = |id: &str, namespace: &str| Instance::new(id, namespace).expect("a valid instance");
    let cases = [
        ("a major version", ClientHello { version: ProtocolVersion { major: 2, minor: 0 }, ..hello() }, "remote::engine::version_mismatch"),
        ("a schema", ClientHello { schema: [0; 32], ..hello() }, "remote::engine::schema_mismatch"),
        ("a codec", ClientHello { codec: "bitcode-0.7.0".to_string(), ..hello() }, "remote::engine::codec_mismatch"),
        ("an id", ClientHello { expected: named("other", "default"), ..hello() }, "remote::engine::wrong_worker"),
        ("a namespace", ClientHello { expected: named("fixture", "test"), ..hello() }, "remote::engine::wrong_worker"),
        (
            "a malformed name",
            ClientHello { expected: Instance { id: "../x".to_string(), namespace: "default".to_string() }, ..hello() },
            "remote::engine::invalid_frame",
        ),
        ("a zero limit", ClientHello { requested: Limits { control: 0, ..LIMITS_DEFAULT }, ..hello() }, "remote::engine::invalid_frame"),
    ];
    for (name, hello, kind) in cases {
        let HelloReply::Refused(error) = worker().reply(&hello) else { panic!("{name}: accepted") };
        assert_eq!(error.kind(), kind, "{name}: {}", error.message());
    }
}

#[test]
fn a_minor_version_beneath_one_schema_is_accepted() {
    let newer = ClientHello { version: ProtocolVersion { major: 1, minor: 7 }, ..hello() };
    let HelloReply::Accepted(accepted) = worker().reply(&newer) else { panic!("refused") };
    assert_eq!(accepted.version, PROTOCOL_VERSION);
}

#[test]
fn raised_limits_negotiate_down_to_the_worker_s_and_the_hard_ceilings() {
    let asked = ClientHello { requested: GREEDY, ..hello() };
    assert_eq!(worker().check(&asked).expect("accepted"), LIMITS_DEFAULT);
    // A worker configured past the hard ceilings still grants none past them.
    let lavish = WorkerProfile { limits: GREEDY, ..worker() };
    assert_eq!(lavish.check(&asked).expect("accepted"), LIMITS_HARD);
    // A modest request is granted as asked.
    let modest = Limits { frame: 1 << 20, calls: 2, ..LIMITS_DEFAULT };
    let granted = worker().check(&ClientHello { requested: modest, ..hello() }).expect("accepted");
    assert_eq!(granted, modest);
    assert_eq!(granted.decode_budget(), DecodeBudget { heap: modest.decode_heap, ..DECODE_BUDGET_DEFAULT });
}

#[test]
fn the_mcp_accepts_the_worker_it_expects() {
    let worker = check_reply(HelloReply::Accepted(Box::new(reply())), &hello()).expect("accepted");
    assert_eq!(worker, reply());
}

#[test]
fn the_mcp_refuses_each_wrong_reply() {
    let other = Instance::new("fixture", "test").expect("a valid instance");
    let cases = [
        ("a major version", WorkerHello { version: ProtocolVersion { major: 0, minor: 9 }, ..reply() }, "remote::engine::version_mismatch"),
        ("a schema", WorkerHello { schema: [1; 32], ..reply() }, "remote::engine::schema_mismatch"),
        ("a codec", WorkerHello { codec: "bitcode-0.6.8".to_string(), ..reply() }, "remote::engine::codec_mismatch"),
        ("an instance", WorkerHello { instance: other, ..reply() }, "remote::engine::wrong_worker"),
        ("a nom not base62", WorkerHello { nom: "../nom".to_string(), ..reply() }, "remote::engine::invalid_frame"),
        ("a raised limit", WorkerHello { effective: Limits { frame: FRAME_HARD, ..LIMITS_DEFAULT }, ..reply() }, "remote::engine::invalid_frame"),
        ("a zero limit", WorkerHello { effective: Limits { calls: 0, ..LIMITS_DEFAULT }, ..reply() }, "remote::engine::invalid_frame"),
    ];
    for (name, worker, kind) in cases {
        let error = check_reply(HelloReply::Accepted(Box::new(worker)), &hello()).expect_err(name);
        assert_eq!(error.kind(), kind, "{name}: {}", error.message());
    }
    // A refusal is the worker's own error, passed through.
    let refusal = HelloReply::Refused(TransportError::Unavailable { reason: "runtime::namespace_held".to_string() });
    assert_eq!(check_reply(refusal, &hello()).expect_err("refused").kind(), "remote::engine::unavailable");
}

#[test]
fn the_mcp_holds_each_limit_to_the_hard_ceilings_whatever_it_asked() {
    let asked = ClientHello { requested: GREEDY, ..hello() };
    let past = [
        ("frame", Limits { frame: LIMITS_HARD.frame + 1, ..LIMITS_HARD }),
        ("decode_heap", Limits { decode_heap: LIMITS_HARD.decode_heap + 1, ..LIMITS_HARD }),
        ("calls", Limits { calls: LIMITS_HARD.calls + 1, ..LIMITS_HARD }),
        ("control", Limits { control: LIMITS_HARD.control + 1, ..LIMITS_HARD }),
    ];
    for (name, effective) in past {
        let error = check_reply(HelloReply::Accepted(Box::new(WorkerHello { effective, ..reply() })), &asked).expect_err(name);
        assert_eq!(error.kind(), "remote::engine::invalid_frame", "{name}");
        assert!(error.message().contains(name), "{name}: {}", error.message());
    }
    let at_the_ceilings = WorkerHello { effective: LIMITS_HARD, ..reply() };
    assert_eq!(check_reply(HelloReply::Accepted(Box::new(at_the_ceilings)), &asked).expect("accepted").effective, LIMITS_HARD);
}

#[test]
fn both_ends_agree_on_one_set_of_fixtures() {
    for requested in [LIMITS_DEFAULT, Limits { frame: 1 << 16, calls: 1, ..LIMITS_DEFAULT }, GREEDY] {
        let asked = ClientHello::new(instance(), requested);
        let worker = check_reply(worker().reply(&asked), &asked).expect("both ends accept");
        assert_eq!(worker.effective, requested.least(LIMITS_DEFAULT));
    }
}

#[test]
fn the_deadlines_are_finite_and_the_ping_beats_the_idle_window() {
    assert_eq!(HELLO_DEADLINE, Duration::from_secs(5));
    assert_eq!(REPLY_DEADLINE, Duration::from_secs(60));
    assert_eq!(AUTH_DEADLINE, Duration::from_secs(30));
    assert_eq!((PING_INTERVAL, SESSION_IDLE), (Duration::from_secs(10), Duration::from_secs(30)));
    assert!(PING_INTERVAL * 2 < SESSION_IDLE, "two pings fit the idle window");
    assert_eq!((ASK_DEADLINE, DRAIN_DEADLINE), (Duration::from_secs(30), Duration::from_secs(5)));
}

#[test]
fn an_instance_name_is_refused_unless_valid_and_never_sanitized() {
    let long = "x".repeat(65);
    for bad in ["", ".", "..", "-x", "_x", ".x", "a/b", "a b", "a\0b", "\u{e9}", long.as_str()] {
        assert!(Instance::new(bad, "default").is_err(), "{bad:?} as an id");
        assert!(Instance::new("fixture", bad).is_err(), "{bad:?} as a namespace");
    }
    let longest = "x".repeat(64);
    for good in ["a", "A9", "a.b_c-d", longest.as_str()] {
        let named = Instance::new(good, good).expect("a valid instance");
        assert_eq!(named.id, good, "sanitized");
    }
}

/// Every hello and reply variant's bitcode.
fn vectors() -> BTreeMap<String, golden::Vector> {
    let mut out = BTreeMap::new();
    out.insert("client_hello".to_string(), golden::vector(&hello()));
    out.insert("reply/accepted".to_string(), golden::vector(&HelloReply::Accepted(Box::new(reply()))));
    let refusal = TransportError::WrongWorker { expected: "fixture/default".to_string(), found: "fixture/test".to_string() };
    out.insert("reply/refused".to_string(), golden::vector(&HelloReply::Refused(refusal)));
    out
}

#[test]
fn each_hello_and_reply_has_its_golden_vector_and_decodes_within_the_hello_budget() {
    let actual = golden::hex_table(vectors());
    let golden = golden::read(GOLDEN, golden::VECTORS_MODEL, &actual);
    assert_eq!(golden, actual, "a hello's bytes changed; a deliberate change re-blesses {GOLDEN}");
    let vectors = vectors();
    for (name, vector) in &vectors {
        assert!(vector.bytes.len() <= HELLO_FRAME_MAX as usize, "{name} fits the hello's frame");
    }
    let client: ClientHello = decode_payload(&vectors["client_hello"].bytes, HELLO_BUDGET).expect("decoded");
    assert_eq!(client, hello());
    let accepted: HelloReply = decode_payload(&vectors["reply/accepted"].bytes, HELLO_BUDGET).expect("decoded");
    assert!(matches!(accepted, HelloReply::Accepted(worker) if *worker == reply()));
    let refused: HelloReply = decode_payload(&vectors["reply/refused"].bytes, HELLO_BUDGET).expect("decoded");
    assert!(matches!(refused, HelloReply::Refused(TransportError::WrongWorker { .. })));
    // A hello naming more than its budget's heap is refused as it decodes.
    let flood = ClientHello { expected: Instance { id: "x".repeat(70_000), namespace: "default".to_string() }, ..hello() };
    let outcome = decode_payload::<ClientHello>(&bitcode::encode(&flood), HELLO_BUDGET);
    assert!(matches!(outcome, Err(FrameError::Budget { bound: bitcode::Exceeded::Heap, .. })), "{outcome:?}");
}
