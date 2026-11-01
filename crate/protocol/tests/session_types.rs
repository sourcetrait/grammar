//! The session's data on the wire: the interface catalog held to its baseline, a
//! golden bitcode vector for every frame, request, response, ask, answer,
//! operator, Channel, peer, and error variant, their round trips, and the
//! errors' kinds and rows.

use std::collections::{BTreeMap, BTreeSet};

use sourcetrait_common::datum::Nonce;
use sourcetrait_grammar_protocol::*;

#[path = "support/golden.rs"]
mod golden;

/// A payload as `T` within the default decode budget, the path every frame takes.
fn decode<T: bitcode::DecodeOwned>(payload: &[u8]) -> Result<T, FrameError> {
    decode_payload(payload, DECODE_BUDGET_DEFAULT)
}

/// The golden vectors' file, every vector's bitcode bytes in hex.
const GOLDEN: &str = "tests/vectors/session.nuon";

fn warning() -> Diagnostic {
    Diagnostic::warning("lint::summary_length", None, "a summary past its length")
}

fn refusal() -> Diagnostic {
    let source = Source { path: None, position: [1, 10] };
    Diagnostic::error("interface::invalid", Some(source), "one `interface` command, alone")
}

/// A command kind, matched exhaustively so a new one fails the build here.
fn kind_name(kind: CommandKind) -> &'static str {
    use CommandKind as K;
    match kind {
        K::Info => "info",
        K::Help => "help",
        K::ProcessList => "process list",
        K::ProcessKill => "process kill",
        K::PurviewList => "purview list",
        K::PurviewSet => "purview set",
        K::PurviewExtend => "purview extend",
        K::PurviewConfigure => "purview configure",
        K::ChannelOpen => "channel open",
        K::ChannelSync => "channel sync",
        K::ChannelReady => "channel ready",
        K::ChannelClose => "channel close",
        K::ChannelConfig => "channel config",
        K::RemoteList => "remote channel list",
        K::RemoteGet => "remote channel get",
        K::RemoteSet => "remote channel set",
        K::RemoteUnset => "remote channel unset",
        K::RemoteListen => "remote channel listen",
        K::RemoteConnect => "remote channel connect",
        K::RemoteClose => "remote channel close",
        K::EngineList => "remote engine list",
        K::EngineGet => "remote engine get",
        K::EngineSet => "remote engine set",
        K::EngineUnset => "remote engine unset",
        K::EngineConnect => "remote engine connect",
        K::EngineClose => "remote engine close",
        K::EngineImport => "remote engine import",
        K::EquipNew => "equip new",
        K::EquipInstall => "equip install",
        K::EquipCheck => "equip check",
        K::EquipUninstall => "equip uninstall",
        K::EquipScaffold => "equip scaffold",
        K::EquipCommit => "equip commit",
    }
}

#[test]
fn the_catalog_holds_the_baseline_commands() {
    assert_eq!(COMMANDS.len(), 33);
    let words: BTreeSet<&str> = COMMANDS.iter().map(|spec| spec.words()).collect();
    assert_eq!(words.len(), 33, "a command's words repeat");
    for spec in COMMANDS {
        assert_eq!(spec.words(), kind_name(spec.kind), "{}", spec.name);
        assert_eq!(spec.name, format!("{INTERFACE} {}", spec.words()));
        assert!(spec.summary.len() <= 80, "{}: a summary past 80", spec.name);
        assert!(["record", "table", "nothing"].contains(&spec.result), "{}", spec.name);
        assert!(command_spec(spec.name).is_some_and(|found| found.kind == spec.kind));
        assert!(command_by_words(spec.words()).is_some_and(|found| found.kind == spec.kind));
    }
    let remote: Vec<&str> =
        COMMANDS.iter().filter(|spec| spec.topic == "Remote").map(|spec| spec.words()).collect();
    assert_eq!(remote.len(), 14, "the seven remote channel commands and the seven remote engine ones");
    let topics: Vec<&str> = COMMANDS.iter().map(|spec| spec.topic).fold(Vec::new(), |mut seen, topic| {
        if seen.last() != Some(&topic) {
            seen.push(topic);
        }
        seen
    });
    assert_eq!(topics, ["General", "Process", "Purview", "Channel", "Remote", "Equip"]);
    assert_eq!(HELP_WORDS, ["interface", "grimm"]);
}

#[test]
fn a_denied_command_leaves_both_blocks() {
    let all_list = render_command_list(|_| false);
    let all_signatures = render_signatures(|_| false);
    for spec in COMMANDS {
        assert!(all_list.contains(spec.name), "{}", spec.name);
        assert!(all_signatures.contains(&spec.signature_text()), "{}", spec.name);
        let list = render_command_list(|words| words == spec.words());
        let signatures = render_signatures(|words| words == spec.words());
        assert!(!list.contains(&format!("- {}\n", spec.name)), "{}", spec.name);
        assert!(!signatures.contains(&format!("`{}`", spec.signature_text())), "{}", spec.name);
    }
}

#[test]
fn a_signature_line_names_each_parameter_and_the_result() {
    let set = command_by_words("remote channel set").expect("set");
    assert_eq!(
        set.signature_text(),
        "remote channel set [alias: string, --address: string, --listen: string, --cert: string]: nothing -> record",
    );
    let help = command_by_words("help").expect("help");
    assert_eq!(help.signature_text(), "help [namepath: string]: nothing -> record");
    let engine = command_by_words("remote engine set").expect("engine set");
    assert_eq!(
        engine.signature_text(),
        "remote engine set [alias: string, --address: string, --user: string, --key: string, \
         --known-hosts: list<string>, --known-hosts-file: string, --command: string, --id: string, \
         --namespace: string, --workdir: string]: nothing -> record",
    );
}

/// Every operator vector.
fn operator() -> Vec<(String, Emergency)> {
    let all = vec![
        Emergency::HungEngineThread(HungEngineThreadEmergency {
            nonce: "aOiwlfCpNKg".into(),
            lane: "stateless".into(),
            tool: "execute".into(),
            started_at: 1,
            cancelled_at: 2,
            hung_ms: u64::MAX,
            pool_held: u64::MAX,
            pool_cap: 16,
        }),
        Emergency::CpuWarning(CpuWarningEmergency { cpu_pct: 812.5, sample_ms: 5000 }),
        Emergency::RamWarning(RamWarningEmergency { rss_kb: u64::MAX }),
        Emergency::VramWarning(VramWarningEmergency { used_mib: 24_000, total_mib: 24_576 }),
        Emergency::BackgroundJobsWarning(BackgroundJobsWarningEmergency { job_count: u64::MAX }),
        Emergency::DiskWarning(DiskWarningEmergency { mount: "/".into(), used_pct: 95, threshold_pct: 90 }),
        Emergency::ChannelSpamWarning(ChannelSpamWarningEmergency {
            origin: "thread/aOiwlfCpNKg".into(),
            hits: 21,
            window_secs: 1,
            rate: 20,
        }),
        Emergency::ChannelSpamError(spam_error()),
        Emergency::Critical(CriticalEmergency { reason: "every permit held".into(), hung: 16, cap: 16 }),
    ];
    all.into_iter().map(|e| (format!("emergency/{}", e.kind().name()), e)).collect()
}

fn spam_error() -> ChannelSpamErrorEmergency {
    ChannelSpamErrorEmergency {
        origin: "thread/aOiwlfCpNKg".into(),
        hits: 101,
        window_secs: 10,
        rate: 100,
        action: "stopped the eval".into(),
    }
}

/// Every runtime error, by its variant.
fn runtime_errors() -> Vec<(&'static str, RuntimeError)> {
    use RuntimeError as R;
    vec![
        ("interface_violations", R::InterfaceViolations { diagnostics: vec![refusal(), warning()] }),
        ("host_unknown", R::HostUnknown { host: "example".into() }),
        ("schema_invalid", R::SchemaInvalid { reason: "not a type".into() }),
        ("purview_invalid_id", R::PurviewInvalidId { id: "Bad".into(), reason: "uppercase".into() }),
        ("channel_start", R::ChannelStart { reason: "no port".into() }),
        ("channel_not_open", R::ChannelNotOpen),
        ("channel_not_claimed", R::ChannelNotClaimed { mode: ChannelMode::Sync }),
        ("channel_wrong_mode", R::ChannelWrongMode { open: ChannelMode::Async, asked: ChannelMode::Sync }),
        ("channel_peer_gone", R::ChannelPeerGone),
        ("remote_invalid_params", R::RemoteInvalidParams { reason: "no cert".into() }),
        ("remote_already_open", R::RemoteAlreadyOpen { alias: "lab/peer".into() }),
        ("remote_not_open", R::RemoteNotOpen { alias: "lab/peer".into() }),
        ("remote_open_failed", R::RemoteOpenFailed { alias: "lab/peer".into(), reason: "refused".into() }),
        ("remote_channel_unverified", R::RemoteChannelUnverified),
        ("remotes_file", R::RemotesFile { reason: "line 3".into() }),
        ("internal", R::Internal { phase: "io".into(), reason: "denied".into() }),
    ]
}

/// Every transport error, by its variant.
fn transport_errors() -> Vec<(&'static str, TransportError)> {
    use TransportError as T;
    vec![
        ("version_mismatch", T::VersionMismatch { expected: "1.0".into(), found: "2.0".into() }),
        ("schema_mismatch", T::SchemaMismatch { expected: "a".into(), found: "b".into() }),
        ("codec_mismatch", T::CodecMismatch { expected: "cereal-bitcode-0.0.0-1".into(), found: "bitcode-0.6.9".into() }),
        ("wrong_worker", T::WrongWorker { expected: "fixture/default".into(), found: "fixture/test".into() }),
        ("connect_failed", T::ConnectFailed { reason: "refused".into() }),
        ("host_key_rejected", T::HostKeyRejected { reason: "ssh-ed25519 SHA256:x".into() }),
        ("authentication_failed", T::AuthenticationFailed { reason: "publickey".into() }),
        ("unavailable", T::Unavailable { reason: "runtime::namespace_held".into() }),
        ("busy", T::Busy),
        ("invalid_frame", T::InvalidFrame { reason: "a trailing byte".into() }),
        ("result_too_large", T::ResultTooLarge { limit: 8 << 20 }),
        ("session_lost", T::SessionLost { reason: "the worker's stream ended".into() }),
    ]
}

fn nonce() -> Nonce {
    Nonce::new_unchecked(7)
}

fn call() -> CallRequest {
    CallRequest {
        host: Host::Local,
        nonce: nonce(),
        namepath: "gram:get:log".into(),
        data: "{refs: []}".into(),
        timeout: None,
    }
}

fn info() -> WorkerInfo {
    WorkerInfo {
        version: "0.0.0-9".into(),
        nu_version: "0.116.1".into(),
        id: "fixture".into(),
        namespace: "default".into(),
        nom: "workernom01".into(),
        work_dir: "/srv/work".into(),
        tmp: "/srv/tmp/grammar-ssh.x".into(),
        transient: "/run/user/2001/grammar-ssh.y".into(),
        plugins: vec![PluginInfo("nu_plugin_eye".into(), Some("0.1.0".into())), PluginInfo("bad".into(), None)],
    }
}

fn prepared() -> PreparedSend {
    PreparedSend {
        mcp_nom: "peernom0001".into(),
        model: "example/Receipt".into(),
        event_nuon: "{n: 1}".into(),
        files: vec![SendFile { dest: "out.nuon".into(), bytes: vec![0, 0xff] }],
    }
}

fn channel_send() -> ChannelSendData {
    ChannelSendData { from: "thread/x".into(), model: "example/Note".into(), event: NuVal::Nothing, attached: None }
}

/// Every engine request, by its variant.
fn requests() -> Vec<(&'static str, EngineRequest)> {
    vec![
        ("wire", EngineRequest::Wire(call().into())),
        ("purview", EngineRequest::Purview(PurviewRequest::Resolve(PurviewResolveRequest { ids: vec!["core".into()] }))),
        ("info", EngineRequest::Info),
    ]
}

/// Every engine response, by its variant.
fn responses() -> Vec<(&'static str, EngineResponse)> {
    let answer = ValueAnswer { nonce: nonce(), result: Ok(NuVal::Nothing) };
    let views = vec![PurviewView("core".into(), vec!["gram:".into()])];
    vec![
        ("wire", EngineResponse::Wire(FromEngineSys::Value(answer))),
        ("purview", EngineResponse::Purview(PurviewAnswer::Listed(Ok(views)))),
        ("info", EngineResponse::Info(info())),
        ("refused", EngineResponse::Refused(TransportError::Busy)),
    ]
}

/// Every ask, by its variant.
fn asks() -> Vec<(&'static str, SessionAsk)> {
    vec![
        ("channel_send", SessionAsk::ChannelSend(channel_send())),
        ("stopped", SessionAsk::Stopped(spam_error())),
        ("remote_send", SessionAsk::RemoteSend(prepared())),
        ("config", SessionAsk::Config),
    ]
}

/// Every host answer, by its variant.
fn answers() -> Vec<(&'static str, HostAnswer)> {
    let stop = HostStop { hits: 101, window_secs: 10, rate: 100, notify: true, reason: "too fast".into() };
    vec![
        ("sent", HostAnswer::Sent("eventid0001".into())),
        ("refused", HostAnswer::Refused("not ready".into())),
        ("stopped", HostAnswer::Stopped(stop)),
        ("config", HostAnswer::Config(NuVal::Record(vec![("channel".into(), NuVal::Nothing)]))),
        ("pinned", HostAnswer::Pinned),
        ("noted", HostAnswer::Noted),
    ]
}

/// Every frame each side sends, by its variant.
fn frames() -> Vec<(&'static str, golden::Vector)> {
    let ping = Ping { nonce: u64::MAX };
    let closed = SessionClose { error: Some(TransportError::SessionLost { reason: "no frame in 30 s".into() }) };
    vec![
        ("mcp/request", golden::vector(&McpFrame::Request { id: 1, request: EngineRequest::Info })),
        ("mcp/answer", golden::vector(&McpFrame::Answer { id: 2, answer: HostAnswer::Noted })),
        ("mcp/ping", golden::vector(&McpFrame::Ping(ping))),
        ("mcp/close", golden::vector(&McpFrame::Close(SessionClose { error: None }))),
        ("worker/response", golden::vector(&WorkerFrame::Response { id: 1, response: EngineResponse::Info(info()) })),
        ("worker/ask", golden::vector(&WorkerFrame::Ask { id: 2, ask: SessionAsk::Config })),
        ("worker/emergency", golden::vector(&WorkerFrame::Emergency(Emergency::RamWarning(RamWarningEmergency { rss_kb: 1 })))),
        ("worker/pong", golden::vector(&WorkerFrame::Pong(ping))),
        ("worker/close", golden::vector(&WorkerFrame::Close(closed))),
    ]
}

fn misc() -> Vec<(&'static str, golden::Vector)> {
    let observation = Observation {
        engine: Some(EngineHealth {
            hung: vec![HungEval { nonce: "n".into(), tool: "call".into(), lane: Lane::Stateful, started_at: 1, cancelled_at: 2 }],
            permits_held: 1,
            permits_cap: 16,
            jobs: 0,
        }),
        listening: true,
    };
    let remote = RemoteSendData { mcp_nom: "peernom0001".into(), model: "example/Receipt".into(), event: NuVal::Nothing, attached: Some(NuVal::List(Vec::new())) };
    let row = RemoteChannelRow {
        alias: "lab/peer".into(),
        address: None,
        listen: Some("127.0.0.1:7000".into()),
        cert: Some("peer.pem".into()),
        state: RemoteLinkState::Open,
        endpoint: Some("127.0.0.1:7000".into()),
        remote_mcp_nom: Some("peernom0001".into()),
    };
    let thresholds = SpamThresholdsData { warn_window_secs: 1, warn_rate: 20, error_window_secs: 10, error_rate: 100 };
    vec![
        ("observation", golden::vector(&observation)),
        ("channel_send", golden::vector(&channel_send())),
        ("remote_send", golden::vector(&remote)),
        ("remote_row", golden::vector(&row)),
        ("spam_thresholds", golden::vector(&thresholds)),
        ("purview_row", golden::vector(&PurviewRow { id: "core".into(), namepath_patterns: vec!["gram:".into()] })),
    ]
}

/// Every vector's name and its encoding.
fn encoded() -> BTreeMap<String, golden::Vector> {
    let mut out = BTreeMap::new();
    for (name, e) in operator() {
        out.insert(name, golden::vector(&e));
    }
    for (name, e) in runtime_errors() {
        out.insert(format!("runtime_error/{name}"), golden::vector(&e));
    }
    for (name, e) in transport_errors() {
        out.insert(format!("transport_error/{name}"), golden::vector(&e));
    }
    for (name, r) in requests() {
        out.insert(format!("request/{name}"), golden::vector(&r));
    }
    for (name, r) in responses() {
        out.insert(format!("response/{name}"), golden::vector(&r));
    }
    for (name, a) in asks() {
        out.insert(format!("ask/{name}"), golden::vector(&a));
    }
    for (name, a) in answers() {
        out.insert(format!("answer/{name}"), golden::vector(&a));
    }
    for (name, vector) in frames() {
        out.insert(format!("frame/{name}"), vector);
    }
    for (name, vector) in misc() {
        out.insert(format!("data/{name}"), vector);
    }
    out
}

#[test]
fn every_vector_encodes_to_its_golden_bytes() {
    let actual = golden::hex_table(encoded());
    let golden = golden::read(GOLDEN, golden::VECTORS_MODEL, &actual);
    assert_eq!(golden, actual, "an encoding moved; a deliberate change re-blesses {GOLDEN}");
}

/// A value's bitcode round trip, its encoding the same once back.
fn round_trips<T>(name: &str, value: &T)
where
    T: bitcode::Encode + for<'a> bitcode::Decode<'a>,
{
    let bytes = bitcode::encode(value);
    let back: T = decode(&bytes).expect(name);
    assert_eq!(bitcode::encode(&back), bytes, "{name}");
}

#[test]
fn every_vector_round_trips() {
    for (name, e) in operator() {
        round_trips(&name, &e);
    }
    for (name, e) in runtime_errors() {
        round_trips(name, &e);
    }
    for (name, e) in transport_errors() {
        round_trips(name, &e);
    }
    for (name, r) in requests() {
        round_trips(name, &r);
    }
    for (name, r) in responses() {
        round_trips(name, &r);
    }
    for (name, a) in asks() {
        round_trips(name, &a);
    }
    for (name, a) in answers() {
        round_trips(name, &a);
    }
}

#[test]
fn every_variant_has_its_vector() {
    let emergencies: BTreeSet<_> = operator().iter().map(|(_, e)| e.kind().name()).collect();
    assert_eq!(emergencies.len(), 9);
    let requests: BTreeSet<_> = requests()
        .iter()
        .map(|(_, r)| match r {
            EngineRequest::Wire(_) => "wire",
            EngineRequest::Purview(_) => "purview",
            EngineRequest::Info => "info",
        })
        .collect();
    assert_eq!(requests.len(), 3);
    let responses: BTreeSet<_> = responses()
        .iter()
        .map(|(_, r)| match r {
            EngineResponse::Wire(_) => "wire",
            EngineResponse::Purview(_) => "purview",
            EngineResponse::Info(_) => "info",
            EngineResponse::Refused(_) => "refused",
        })
        .collect();
    assert_eq!(responses.len(), 4);
    let asks: BTreeSet<_> = asks()
        .iter()
        .map(|(_, a)| match a {
            SessionAsk::ChannelSend(_) => "channel_send",
            SessionAsk::Stopped(_) => "stopped",
            SessionAsk::RemoteSend(_) => "remote_send",
            SessionAsk::Config => "config",
        })
        .collect();
    assert_eq!(asks.len(), 4);
    let answers: BTreeSet<_> = answers()
        .iter()
        .map(|(_, a)| match a {
            HostAnswer::Sent(_) => "sent",
            HostAnswer::Refused(_) => "refused",
            HostAnswer::Stopped(_) => "stopped",
            HostAnswer::Config(_) => "config",
            HostAnswer::Pinned => "pinned",
            HostAnswer::Noted => "noted",
        })
        .collect();
    assert_eq!(answers.len(), 6);
    let kinds: BTreeSet<_> = runtime_errors()
        .into_iter()
        .filter(|(_, e)| !matches!(e, RuntimeError::InterfaceViolations { .. }))
        .map(|(_, e)| e.kind())
        .collect();
    assert_eq!(kinds.len(), 15);
    let transport: BTreeSet<_> = transport_errors().iter().map(|(_, e)| e.kind()).collect();
    assert_eq!(transport.len(), 12);
}

#[test]
fn a_call_takes_a_call_slot_and_every_other_request_a_control_one() {
    assert!(EngineRequest::Wire(call().into()).is_call());
    let control = ControlRequest { host: Host::Local, command: EngineCommand::Kill { nonce: "n".into() } };
    for (name, request) in [("control", EngineRequest::Wire(control.into())), ("info", EngineRequest::Info)] {
        assert!(!request.is_call(), "{name}");
    }
}

#[test]
fn a_violations_refusal_reports_its_errors_alone() {
    let rows = RuntimeError::InterfaceViolations { diagnostics: vec![refusal(), warning()] }.rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, "interface::invalid");
    let single = RuntimeError::ChannelWrongMode { open: ChannelMode::Async, asked: ChannelMode::Sync }.rows();
    assert_eq!(single.len(), 1);
    assert_eq!(single[0].kind, "channel::wrong_mode");
    assert!(single[0].message.contains("open in `async` mode, not `sync`"), "{}", single[0].message);
}

#[test]
fn a_transport_failure_keeps_what_never_ran_apart_from_what_may_have() {
    for (name, error) in transport_errors() {
        assert!(error.kind().starts_with("remote::engine::"), "{name}");
        assert_eq!(error.clone().rows().len(), 1, "{name}");
    }
    let auth = TransportError::AuthenticationFailed { reason: "publickey".into() };
    let lost = TransportError::SessionLost { reason: "eof".into() };
    assert_ne!(auth.kind(), lost.kind());
    assert!(auth.message().contains("before any request was sent"));
    assert!(lost.message().contains("its outcome unknown"), "{}", lost.message());
}

#[test]
fn a_mode_and_a_link_state_serialize_as_their_words() {
    let word = |s: &str| NuVal::String(s.into());
    assert_eq!(nuval::to_nuval(&ChannelMode::Async).expect("a value"), word("async"));
    assert_eq!(nuval::to_nuval(&RemoteLinkState::Listening).expect("a value"), word("listening"));
    assert_eq!(
        nuval::to_nuval(&PluginInfo("p".into(), None)).expect("a value"),
        NuVal::List(vec![word("p"), NuVal::Nothing]),
    );
    assert_eq!(
        nuval::to_nuval(&PurviewView("core".into(), vec![])).expect("a value"),
        NuVal::List(vec![word("core"), NuVal::List(vec![])]),
    );
}

#[test]
fn the_reservations_and_caps_hold_their_values() {
    assert_eq!(reserved_prefix("mcp/channel/Open"), Some(MCP_RESERVED_PREFIX));
    assert_eq!(reserved_prefix("remote/channel/Sent"), Some(REMOTE_RESERVED_PREFIX));
    assert_eq!(reserved_prefix("example/Note"), None);
    assert_eq!(SYNC_QUEUE_CAP, 1000);
    assert_eq!(LOCAL_HOST, "local");
}

const _: () = assert!(MAX_FRAME_BYTES < 1 << 20);
