//! The engine's requests and answers on the wire: a golden bitcode vector for
//! every request, command, answer, document, and error variant, the round
//! trips, the conversions to and from the system's messages, and the kinds.

use std::collections::{BTreeMap, BTreeSet};

use sourcetrait_common::{
    datum::{Nom, Nonce},
    subsys::SubsysError,
};
use sourcetrait_grammar_protocol::*;

#[path = "support/golden.rs"]
mod golden;

/// A payload as `T` within the default decode budget, the path every frame takes.
fn decode<T: bitcode::DecodeOwned>(payload: &[u8]) -> Result<T, FrameError> {
    decode_payload(payload, DECODE_BUDGET_DEFAULT)
}

/// The golden vectors' file, every vector's bitcode bytes in hex.
const GOLDEN: &str = "tests/vectors/engine.nuon";

/// One operation's public nonce, fixed so the vectors are.
const NONCE: Nonce = Nonce::new_unchecked(0x0123_4567_89ab_cdef);

fn hosts() -> [(&'static str, Host); 2] {
    [("local", Host::Local), ("remote", Host::Remote(Nom::new_unchecked(u64::MAX)))]
}

fn located() -> Diagnostic {
    let source = Source { path: Some("acme/geo/rig/shape/mod.nu".into()), position: [u64::MAX, 0] };
    Diagnostic::error("definition::invalid", Some(source), "a definition's refusal")
}

fn warning() -> Diagnostic {
    Diagnostic::warning("lint::summary_length", None, "a summary past its length")
}

/// Every command, by its variant.
fn commands() -> Vec<(&'static str, EngineCommand)> {
    use EngineCommand as C;
    let source = || "equip/acme/geo".to_string();
    vec![
        ("processes", C::Processes),
        ("kill", C::Kill { nonce: "aOiwlfCpNKg".into() }),
        ("health", C::Health),
        ("teardown", C::Teardown),
        ("equip_new", C::EquipNew { source: source() }),
        ("equip_install", C::EquipInstall { source: source() }),
        ("equip_check", C::EquipCheck { source: source() }),
        ("equip_uninstall", C::EquipUninstall { source: source() }),
        ("equip_scaffold", C::EquipScaffold { namepaths: vec!["acme/geo:shape:area".into(), "acme/geo:shape".into()] }),
        ("equip_commit", C::EquipCommit { namepath: "acme/geo".into() }),
        ("signatures", C::Signatures { patterns: vec!["*".into(), ".".into(), "acme/".into()] }),
        ("help", C::Help { namepath: "gram:get:log".into() }),
    ]
}

/// Every request on each host, and every command.
fn requests() -> Vec<(String, ToEngineSys)> {
    let mut out = Vec::new();
    for (h, host) in hosts() {
        let execute = ExecuteRequest {
            host,
            nonce: NONCE,
            definition: "def execute [data: nothing]: nothing -> nothing {}".into(),
            data: "null".into(),
            timeout: None,
        };
        let rerun = RerunRequest {
            host,
            nonce: NONCE,
            source: "nonceSource".into(),
            data: "{repo: 'ключ ✓'}".into(),
            timeout: Some("30sec".into()),
        };
        let call = CallRequest {
            host,
            nonce: NONCE,
            namepath: "gram:get:log".into(),
            data: "{refs: []}".into(),
            timeout: Some("2min".into()),
        };
        out.push((format!("request/execute/{h}"), execute.into()));
        out.push((format!("request/rerun/{h}"), rerun.into()));
        out.push((format!("request/call/{h}"), call.into()));
    }
    for (c, command) in commands() {
        out.push((format!("request/control/{c}"), ControlRequest { host: Host::Local, command }.into()));
    }
    out
}

/// Every engine error, by its variant.
fn engine_errors() -> Vec<(&'static str, EngineError)> {
    use EngineError as E;
    vec![
        ("eval", E::Eval { reason: "boom".into() }),
        ("circular_import", E::CircularImport { files: "a.nu -> b.nu -> a.nu".into() }),
        ("init", E::Init { reason: "no home".into() }),
        ("equip_not_registered", E::EquipNotRegistered { equip: "sourcetrait/gram".into() }),
        ("equip_already_registered", E::EquipAlreadyRegistered { equip: "acme/geo".into() }),
        ("equip_invalid_name", E::EquipInvalidName { equip: "Acme/Geo".into(), reason: "uppercase".into() }),
        ("equip_name_denied", E::EquipNameDenied { equip: "sourcetrait/grammar".into() }),
        ("equip_invalid_source", E::EquipInvalidSource { source_dir: "equip/acme/geo".into(), reason: "no rig".into() }),
        ("equip_source_missing", E::EquipSourceMissing { path: "equip/acme/none".into() }),
        (
            "equip_source_path_mismatch",
            E::EquipSourcePathMismatch { equip: "acme/geo".into(), passed: "a".into(), registered: "b".into() },
        ),
        ("equip_no_kinds", E::EquipNoKinds { source_dir: "equip/acme/empty".into() }),
        ("equip_no_rig", E::EquipNoRig { equip: "acme/lib".into() }),
        ("equip_violations", E::EquipViolations { diagnostics: vec![located(), warning()] }),
        ("rig_invalid_name", E::RigInvalidName { name: "Geo".into(), reason: "uppercase".into() }),
        ("rig_invalid_module_path", E::RigInvalidModulePath { module_path: "a//b".into(), reason: "an empty segment".into() }),
        (
            "function_not_defined",
            E::FunctionNotDefined { rig: "sourcetrait/gram".into(), module_path: "get".into(), name: "nothing".into() },
        ),
        ("namepath_invalid", E::NamepathInvalid { namepath: "::".into(), reason: "empty".into() }),
        ("definition_violations", E::DefinitionViolations { diagnostics: vec![located()] }),
        ("data_invalid", E::DataInvalid { reason: "not NUON".into() }),
        ("result_invalid", E::ResultInvalid { reason: "a string".into() }),
        ("timeout_invalid", E::TimeoutInvalid { reason: "negative".into() }),
        ("rerun_invalid_nonce", E::RerunInvalidNonce { nonce: "!".into(), reason: "not base62".into() }),
        ("rerun_body_missing", E::RerunBodyMissing { nonce: "aOiwlfCpNKg".into() }),
        ("dispatch", E::Dispatch { reason: "busy".into() }),
        ("timeout", E::Timeout { timeout_ms: u64::MAX }),
        ("host_unserved", E::HostUnserved { host: "example".into() }),
        ("internal", E::Internal { phase: "io".into(), reason: "denied".into() }),
    ]
}

/// Every control answer, by its variant, with every document's shape.
fn control_answers() -> Vec<(&'static str, ControlAnswer)> {
    use ControlAnswer as A;
    let commit = CommitResult {
        added: vec!["rig/shape/area/mod.nu".into()],
        modified: Vec::new(),
        removed: vec!["rig/old/mod.nu".into()],
        warnings: vec![warning()],
    };
    let rows = vec![
        ProcessRow {
            nonce: "aOiwlfCpNKg".into(),
            tool: "execute".into(),
            started_at: u64::MAX,
            args: NuVal::Record(vec![("data".into(), NuVal::Nothing)]),
            source_nonce: None,
            path: None,
        },
        ProcessRow {
            nonce: "nonceRerun1".into(),
            tool: "rerun".into(),
            started_at: 0,
            args: NuVal::List(vec![NuVal::Int(i64::MIN)]),
            source_nonce: Some("nonceSource".into()),
            path: Some("acme/geo:shape:area".into()),
        },
    ];
    let health = EngineHealth {
        hung: vec![
            HungEval { nonce: "a".into(), tool: "call".into(), lane: Lane::Stateless, started_at: 1, cancelled_at: 2 },
            HungEval { nonce: "b".into(), tool: "execute".into(), lane: Lane::Stateful, started_at: 3, cancelled_at: u64::MAX },
        ],
        permits_held: 3,
        permits_cap: 16,
        jobs: 0,
    };
    vec![
        ("done", A::Done),
        ("processes", A::Processes(rows)),
        ("health", A::Health(health)),
        ("created", A::Created(vec!["rig/shape/area/mod.nu".into()])),
        ("installed", A::Installed { equip: "acme/geo".into(), result: commit.clone() }),
        ("checked", A::Checked(CheckSummary { ok: false, errors: vec![located()], warnings: vec![warning()] })),
        ("uninstalled", A::Uninstalled { equip: Some("acme/geo".into()) }),
        ("uninstalled_none", A::Uninstalled { equip: None }),
        ("committed", A::Committed(commit)),
        ("signatures", A::Signatures("acme\n geo # planar geometry helpers\n".into())),
        (
            "help_rig",
            A::Help(InspectDoc::Rig(RigDoc {
                srcdir: "equip/acme/geo/rig".into(),
                summary: "planar geometry helpers".into(),
                details: String::new(),
            })),
        ),
        (
            "help_module",
            A::Help(InspectDoc::Module(ModuleDoc {
                src: "equip/acme/geo/rig/shape/mod.nu".into(),
                summary: "shapes".into(),
                details: "areas".into(),
            })),
        ),
        (
            "help_call",
            A::Help(InspectDoc::Call(CallDoc {
                src: "equip/acme/geo/rig/shape/area/mod.nu".into(),
                signature: "area <width:float,height:float> <area:float> # the inputs' unit squared".into(),
                details: String::new(),
            })),
        ),
        ("help_signatures", A::Help(InspectDoc::Signatures(SignaturesDoc { signatures: "acme\n geo\n".into() }))),
    ]
}

/// A value's answer, each error's, and every control answer and its refusal.
fn answers() -> Vec<(String, FromEngineSys)> {
    let value = NuVal::Record(vec![("ok".into(), NuVal::Bool(true))]);
    let mut out = vec![(
        "answer/value/ok".to_string(),
        FromEngineSys::Value(ValueAnswer { nonce: NONCE, result: Ok(value) }),
    )];
    for (name, error) in engine_errors() {
        let answer = ValueAnswer { nonce: NONCE, result: Err(error) };
        out.push((format!("answer/value/error/{name}"), FromEngineSys::Value(answer)));
    }
    for (name, answer) in control_answers() {
        out.push((format!("answer/control/{name}"), FromEngineSys::Control(Ok(answer))));
    }
    let refusal = EngineError::EquipNoRig { equip: "acme/lib".into() };
    out.push(("answer/control/error".into(), FromEngineSys::Control(Err(refusal))));
    out
}

/// Every vector's name and its encoding.
fn encoded() -> BTreeMap<String, golden::Vector> {
    let requests = requests().into_iter().map(|(n, r)| (n, golden::vector(&r)));
    let answers = answers().into_iter().map(|(n, a)| (n, golden::vector(&a)));
    requests.chain(answers).collect()
}

#[test]
fn every_vector_encodes_to_its_golden_bytes() {
    let actual = golden::hex_table(encoded());
    let golden = golden::read(GOLDEN, golden::VECTORS_MODEL, &actual);
    assert_eq!(golden, actual, "an encoding moved; a deliberate change re-blesses {GOLDEN}");
}

#[test]
fn every_request_and_answer_round_trips() {
    for (name, request) in requests() {
        let back: ToEngineSys = decode(&bitcode::encode(&request)).expect(&name);
        assert_eq!(back, request, "{name}");
    }
    for (name, answer) in answers() {
        let back: FromEngineSys = decode(&bitcode::encode(&answer)).expect(&name);
        assert_eq!(back, answer, "{name}");
    }
}

#[test]
fn truncated_or_trailing_bytes_are_refused() {
    for (name, vector) in encoded() {
        let bytes = vector.bytes;
        let decode = |b: &[u8]| -> bool {
            if name.starts_with("request/") {
                decode::<ToEngineSys>(b).is_ok()
            } else {
                decode::<FromEngineSys>(b).is_ok()
            }
        };
        for cut in 0..bytes.len() {
            assert!(!decode(&bytes[..cut]), "{name} decoded from its first {cut} bytes");
        }
        let mut longer = bytes.clone();
        longer.push(0);
        assert!(!decode(&longer), "{name} decoded with a trailing byte");
    }
}

/// A command's variant, matched exhaustively so a new one fails the build here.
fn command_variant(c: &EngineCommand) -> &'static str {
    use EngineCommand as C;
    match c {
        C::Processes => "processes",
        C::Kill { .. } => "kill",
        C::Health => "health",
        C::Teardown => "teardown",
        C::EquipNew { .. } => "equip_new",
        C::EquipInstall { .. } => "equip_install",
        C::EquipCheck { .. } => "equip_check",
        C::EquipUninstall { .. } => "equip_uninstall",
        C::EquipScaffold { .. } => "equip_scaffold",
        C::EquipCommit { .. } => "equip_commit",
        C::Signatures { .. } => "signatures",
        C::Help { .. } => "help",
    }
}

/// An error's variant, matched exhaustively so a new one fails the build here.
fn error_variant(e: &EngineError) -> &'static str {
    use EngineError as E;
    match e {
        E::Eval { .. } => "eval",
        E::CircularImport { .. } => "circular_import",
        E::Init { .. } => "init",
        E::EquipNotRegistered { .. } => "equip_not_registered",
        E::EquipAlreadyRegistered { .. } => "equip_already_registered",
        E::EquipInvalidName { .. } => "equip_invalid_name",
        E::EquipNameDenied { .. } => "equip_name_denied",
        E::EquipInvalidSource { .. } => "equip_invalid_source",
        E::EquipSourceMissing { .. } => "equip_source_missing",
        E::EquipSourcePathMismatch { .. } => "equip_source_path_mismatch",
        E::EquipNoKinds { .. } => "equip_no_kinds",
        E::EquipNoRig { .. } => "equip_no_rig",
        E::EquipViolations { .. } => "equip_violations",
        E::RigInvalidName { .. } => "rig_invalid_name",
        E::RigInvalidModulePath { .. } => "rig_invalid_module_path",
        E::FunctionNotDefined { .. } => "function_not_defined",
        E::NamepathInvalid { .. } => "namepath_invalid",
        E::DefinitionViolations { .. } => "definition_violations",
        E::DataInvalid { .. } => "data_invalid",
        E::ResultInvalid { .. } => "result_invalid",
        E::TimeoutInvalid { .. } => "timeout_invalid",
        E::RerunInvalidNonce { .. } => "rerun_invalid_nonce",
        E::RerunBodyMissing { .. } => "rerun_body_missing",
        E::Dispatch { .. } => "dispatch",
        E::Timeout { .. } => "timeout",
        E::HostUnserved { .. } => "host_unserved",
        E::Internal { .. } => "internal",
    }
}

/// An answer's variant, its documents' shapes among them, matched exhaustively.
fn answer_variant(a: &ControlAnswer) -> &'static str {
    use ControlAnswer as A;
    match a {
        A::Done => "done",
        A::Processes(_) => "processes",
        A::Health(_) => "health",
        A::Created(_) => "created",
        A::Installed { .. } => "installed",
        A::Checked(_) => "checked",
        A::Uninstalled { .. } => "uninstalled",
        A::Committed(_) => "committed",
        A::Signatures(_) => "signatures",
        A::Help(InspectDoc::Rig(_)) => "help_rig",
        A::Help(InspectDoc::Module(_)) => "help_module",
        A::Help(InspectDoc::Call(_)) => "help_call",
        A::Help(InspectDoc::Signatures(_)) => "help_signatures",
    }
}

#[test]
fn every_variant_has_its_vector() {
    let commands: BTreeSet<_> = commands().iter().map(|(_, c)| command_variant(c)).collect();
    assert_eq!(commands.len(), 12);
    let errors: BTreeSet<_> = engine_errors().iter().map(|(_, e)| error_variant(e)).collect();
    assert_eq!(errors.len(), 27);
    let answers: BTreeSet<_> = control_answers().iter().map(|(_, a)| answer_variant(a)).collect();
    assert_eq!(answers.len(), 13);
    let messages: BTreeSet<_> = requests()
        .iter()
        .map(|(_, r)| match r {
            ToEngineSys::Execute(_) => "execute",
            ToEngineSys::Rerun(_) => "rerun",
            ToEngineSys::Call(_) => "call",
            ToEngineSys::Control(_) => "control",
        })
        .collect();
    assert_eq!(messages.len(), 4);
}

#[test]
fn an_answer_converts_back_only_from_its_own_variant() {
    let value = || FromEngineSys::Value(ValueAnswer { nonce: NONCE, result: Ok(NuVal::Nothing) });
    let control = || FromEngineSys::Control(Ok(ControlAnswer::Done));
    assert_eq!(
        ValueAnswer::try_from(value()).expect("a value's answer"),
        ValueAnswer { nonce: NONCE, result: Ok(NuVal::Nothing) },
    );
    assert!(matches!(ValueAnswer::try_from(control()), Err(SubsysError::ResponseType)));
    assert_eq!(EngineResult::<ControlAnswer>::try_from(control()).expect("a control answer"), Ok(ControlAnswer::Done));
    assert!(matches!(EngineResult::<ControlAnswer>::try_from(value()), Err(SubsysError::ResponseType)));
}

#[test]
fn each_single_condition_error_keeps_its_kind() {
    let kinds: BTreeMap<&str, &str> = engine_errors()
        .iter()
        .filter(|(_, e)| !matches!(e, EngineError::EquipViolations { .. } | EngineError::DefinitionViolations { .. }))
        .map(|(name, e)| (*name, e.kind()))
        .collect();
    let expected: BTreeMap<&str, &str> = [
        ("eval", "thread::returned_error"),
        ("circular_import", "module::circular_import"),
        ("init", "internal"),
        ("equip_not_registered", "equip::not_registered"),
        ("equip_already_registered", "equip::already_registered"),
        ("equip_invalid_name", "equip::invalid_name"),
        ("equip_name_denied", "equip::name_denied"),
        ("equip_invalid_source", "equip::invalid_source"),
        ("equip_source_missing", "equip::source_missing"),
        ("equip_source_path_mismatch", "equip::source_path_mismatch"),
        ("equip_no_kinds", "equip::no_kinds"),
        ("equip_no_rig", "equip::no_rig"),
        ("rig_invalid_name", "rig::invalid_name"),
        ("rig_invalid_module_path", "rig::invalid_module_path"),
        ("function_not_defined", "function::not_defined"),
        ("namepath_invalid", "namepath::invalid"),
        ("data_invalid", "data::invalid"),
        ("result_invalid", "result::invalid"),
        ("timeout_invalid", "timeout::invalid"),
        ("rerun_invalid_nonce", "rerun::invalid_nonce"),
        ("rerun_body_missing", "rerun::body_missing"),
        ("dispatch", "thread::dispatch"),
        ("timeout", "thread::timeout"),
        ("host_unserved", "host::unserved"),
        ("internal", "internal"),
    ]
    .into_iter()
    .collect();
    assert_eq!(kinds, expected);
}

#[test]
fn an_error_names_a_builtin_by_its_bare_name() {
    let unregistered = EngineError::EquipNotRegistered { equip: "sourcetrait/gram".into() };
    assert_eq!(unregistered.message(), "equip `gram` is not registered");
    let undefined = EngineError::FunctionNotDefined {
        rig: "sourcetrait/grammar".into(),
        module_path: "eye".into(),
        name: "look".into(),
    };
    assert_eq!(undefined.to_string(), "function `look` is not defined in `grammar:eye`");
    let other = EngineError::EquipNoRig { equip: "acme/lib".into() };
    assert_eq!(other.message(), "equip `acme/lib` has no rig");
}

#[test]
fn a_diagnostic_keeps_its_severity_in_bitcode_and_not_in_its_envelope() {
    let error = Diagnostic::error("lint::summary_length", None, "a summary past its length");
    assert_ne!(bitcode::encode(&warning()), bitcode::encode(&error));
    let back: Diagnostic = decode(&bitcode::encode(&warning())).expect("decoded");
    assert_eq!(back.severity, Severity::Warning);
    let NuVal::Record(envelope) = nuval::to_nuval(&warning()).expect("its serde value") else {
        panic!("a diagnostic serializes as a record");
    };
    assert!(!envelope.iter().any(|(key, _)| key == "severity"), "{envelope:?}");
}

#[test]
fn a_violations_error_reports_its_error_rows_alone() {
    let violations = EngineError::EquipViolations { diagnostics: vec![located(), warning()] };
    assert_eq!(violations.to_string(), "a definition's refusal; a summary past its length");
    let rows = violations.rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, "definition::invalid");
    let single = EngineError::Timeout { timeout_ms: 1500 }.rows();
    assert_eq!(single.len(), 1);
    assert_eq!((single[0].kind.as_str(), single[0].message.as_str()), ("thread::timeout", "eval timed out after 1500 ms"));
}
