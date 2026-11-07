//! The engine's public face, started on a host of its own.
//!
//! Each request is answered through its system. One engine serves the binary,
//! since `init` holds once.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use nuval::NuVal;
use sourcetrait_common::testing::prelude::*;
use sourcetrait_common::{datum, subsys};
use sourcetrait_grammar_engine::{
    AwaitEvent, AwaitRequest, CallRequest, ControlAnswer, ControlRequest, DefineRequest, Diagnostic,
    EVAL_STACK_SIZE, EngineCommand,
    EngineError, ExecuteDefinition,
    EngineHandle, EngineIdentity, EngineParams, EngineSystem, ExecuteRequest, FromEngine, Host,
    HostAnswer,
    HostAsk, RerunRequest, TRACE_FILE, ToEngine, ValueAnswer, ensure_equip_store, init,
    nuon_data,
};

/// A test's NUON data as the value a tool's edge takes it to.
fn data_value(text: &str) -> NuVal {
    nuon_data(text, "definition").unwrap_or_else(|e| panic!("`{text}` is NUON: {e:?}"))
}

static TESTING: testing::ModuleWith<TestEngine> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|module| TestEngine::new(module.temp_dir()))
});

/// The model of each Channel send the host was handed.
static SENT: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Each await job's event the host was handed.
static EVENTS: Mutex<Vec<AwaitEvent>> = Mutex::new(Vec::new());

/// The engine's controller: each Channel send's model kept and answered an id.
struct TestHost;

impl subsys::Handler<EngineSystem> for TestHost {
    fn on_packet(&mut self, pkt: subsys::Packet<FromEngine>) -> impl Future<Output = Option<subsys::Packet<FromEngine>>> + 'static + Send {
        std::future::ready(Some(pkt))
    }

    fn on_request(&mut self, pkt: subsys::Packet<FromEngine>) -> impl Future<Output = Option<ToEngine>> + 'static + Send {
        let answer = match pkt.take_msg() {
            FromEngine::Host(HostAsk::ChannelSend(send)) => {
                let id = format!("sent-{}", send.model.replace('/', "-"));
                SENT.lock().unwrap_or_else(|e| e.into_inner()).push(send.model);
                Some(HostAnswer::Sent(id))
            }
            FromEngine::Host(HostAsk::Stopped(_)) => Some(HostAnswer::Noted),
            FromEngine::Host(HostAsk::RemoteSend(_)) => {
                Some(HostAnswer::Refused("the test host links no peers".to_string()))
            }
            FromEngine::Host(HostAsk::Config) => Some(HostAnswer::Config(NuVal::Record(Vec::new()))),
            FromEngine::Host(HostAsk::Pin(_)) => {
                Some(HostAnswer::Refused("the test host pins nothing".to_string()))
            }
            FromEngine::Host(HostAsk::Await(event)) => {
                let id = format!("sent-{}", event.model.replace('/', "-"));
                EVENTS.lock().unwrap_or_else(|e| e.into_inner()).push(event);
                Some(HostAnswer::Sent(id))
            }
            FromEngine::Wire(_) | FromEngine::Purview(_) | FromEngine::Defined(_) => None,
        };
        std::future::ready(answer.map(ToEngine::Host))
    }
}

/// The binary's engine, its runtime, and the root its homes sit under.
struct TestEngine {
    rt: tokio::runtime::Runtime,
    engine: Arc<EngineHandle>,
    nonces: datum::NonceGenerator,
    root: PathBuf,
}

impl TestEngine {
    fn new(root: &Path) -> Self {
        let params = EngineParams {
            id: "test".to_string(),
            namespace: "engine".to_string(),
            work_dir: root.join("work"),
            tmp_dir: root.join("tmp"),
            transient_dir: root.join("transient"),
            data_home: root.join("data"),
            cache_home: root.join("cache"),
            identity: EngineIdentity::draw(),
        };
        for dir in [
            &params.work_dir,
            &params.tmp_dir,
            &params.transient_dir,
            &params.data_home,
            &params.cache_home,
        ] {
            std::fs::create_dir_all(dir).expect("a home for the engine");
        }
        init(params).expect("the engine's init");
        ensure_equip_store().expect("the equip store");
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_stack_size(EVAL_STACK_SIZE)
            .build()
            .expect("a runtime");
        let engine = rt.block_on(EngineHandle::start(TestHost)).expect("the engine's system");
        Self {
            rt,
            engine: Arc::new(engine),
            nonces: datum::NonceGenerator::new(),
            root: root.to_path_buf(),
        }
    }

    /// A definition's model as a tool's edge takes it from the engine, or its refusal's rows.
    fn define(&self, definition: &str) -> Result<ExecuteDefinition, Vec<Diagnostic>> {
        let req = DefineRequest {
            definition: definition.to_string(),
        };
        self.rt.block_on(self.engine.request(req)).expect("the engine answers")
    }

    fn execute_on(&self, host: Host, definition: &str, data: &str) -> ValueAnswer {
        let nonce = self.nonces.generate();
        let definition = match self.define(definition) {
            Ok(definition) => definition,
            Err(diagnostics) => {
                return ValueAnswer { nonce, result: Err(EngineError::DefinitionViolations { diagnostics }) };
            }
        };
        let req = ExecuteRequest {
            host,
            nonce,
            definition,
            data: data_value(data),
            timeout: None,
        };
        self.rt.block_on(self.engine.request(req)).expect("the engine answers")
    }

    fn execute(&self, definition: &str, data: &str) -> ValueAnswer {
        self.execute_on(Host::Local, definition, data)
    }

    fn rerun(&self, source: &str, data: &str) -> ValueAnswer {
        let req = RerunRequest {
            host: Host::Local,
            nonce: self.nonces.generate(),
            source: source.to_string(),
            data: data_value(data),
            timeout: None,
        };
        self.rt.block_on(self.engine.request(req)).expect("the engine answers")
    }

    fn call(&self, namepath: &str, data: &str) -> ValueAnswer {
        let req = CallRequest {
            host: Host::Local,
            nonce: self.nonces.generate(),
            namepath: namepath.to_string(),
            data: data_value(data),
            timeout: None,
        };
        self.rt.block_on(self.engine.request(req)).expect("the engine answers")
    }

    fn control(&self, command: EngineCommand) -> ControlAnswer {
        self.rt
            .block_on(self.engine.control(command.clone()))
            .unwrap_or_else(|e| panic!("{command:?} refused: {e:?}"))
    }

    /// The await jobs running, as `interface await list` answers them.
    fn awaits(&self) -> Vec<NuVal> {
        match self.control(EngineCommand::AwaitList) {
            ControlAnswer::Value(NuVal::List(rows)) => rows,
            other => panic!("await list answered {other:?}"),
        }
    }

    /// Polls the await list until `done` holds of it, or five seconds pass.
    fn wait_for(&self, done: impl Fn(&[NuVal]) -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if done(&self.awaits()) {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// An equip's source under the binary's root: `equip/<author>/<name>`.
    fn equip_source(&self, equip: &str) -> PathBuf {
        self.root.join("src").join("equip").join(equip)
    }

    /// An await call's answer: its suite prepared on its data and its job started.
    fn await_suite(
        &self,
        module: &str,
        data: &str,
        timeouts: Option<NuVal>,
        heartbeats: Option<NuVal>,
    ) -> ValueAnswer {
        let req = AwaitRequest {
            host: Host::Local,
            nonce: self.nonces.generate(),
            module: module.to_string(),
            data: data_value(data),
            timeouts,
            heartbeats,
        };
        self.rt.block_on(self.engine.request(req)).expect("the engine answers")
    }
}

/// A record's field.
fn field<'a>(value: &'a NuVal, name: &str) -> Option<&'a NuVal> {
    match value {
        NuVal::Record(fields) => fields.iter().find(|(key, _)| key == name).map(|(_, v)| v),
        _ => None,
    }
}

/// Whether an await list's rows hold the job under `nonce`.
fn lists(rows: &[NuVal], nonce: &str) -> bool {
    let wanted = NuVal::String(nonce.to_string());
    rows.iter().any(|row| field(row, "await") == Some(&wanted))
}

/// A date's moment in nanoseconds since the epoch.
fn nanos(value: Option<&NuVal>) -> i128 {
    match value {
        Some(NuVal::Date(date)) => i128::from(date.seconds) * 1_000_000_000 + i128::from(date.seconds_fraction),
        other => panic!("a date, not {other:?}"),
    }
}

/// A call's await events, polled until `done` holds of them or ten seconds pass.
fn events_for(nonce: &str, done: impl Fn(&[AwaitEvent]) -> bool) -> Vec<AwaitEvent> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let wanted = NuVal::String(nonce.to_string());
    loop {
        let events: Vec<AwaitEvent> = EVENTS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|event| field(&event.data, "await") == Some(&wanted))
            .cloned()
            .collect();
        if done(&events) || Instant::now() >= deadline {
            return events;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// The PREPARED record the test suite's execute, status, and stop take.
const PREPARED: &str =
    "record<job: record<id: int, started: datetime>, prepared: record<n: int, nap: duration>>";

/// A suite whose job naps then doubles prepare's number, status declaring `status_data`.
fn suite_with(status_data: &str) -> String {
    format!(
        "export def prepare [data: record<x: int, nap: duration>]: nothing -> record<n: int, nap: duration> {{
    {{n: $data.x, nap: $data.nap}}
}}

export def execute [data: {PREPARED}, timeout?: duration]: nothing -> record<doubled: int> {{
    sleep $data.prepared.nap
    {{doubled: ($data.prepared.n * 2)}}
}}

export def status [data: {status_data}]: nothing -> record<beating: bool> {{
    {{beating: true}}
}}

export def stop [data: {PREPARED}]: nothing -> record<stopped: int> {{
    {{stopped: $data.job.id}}
}}
"
    )
}

/// The test suite, its four definitions holding to the contract.
fn suite() -> String {
    suite_with(PREPARED)
}

/// The first event of `model` among a call's, once it arrives.
fn event_of(nonce: &str, model: &str) -> (AwaitEvent, Vec<AwaitEvent>) {
    let events = events_for(nonce, |events| events.iter().any(|event| event.model == model));
    let event = events
        .iter()
        .find(|event| event.model == model)
        .cloned()
        .unwrap_or_else(|| panic!("no {model} event; got {events:?}"));
    (event, events)
}

#[test]
fn await_answers_its_prepared_record_then_its_result() {
    let e = TESTING.harness();
    let answer = e.await_suite(&suite(), "{x: 21, nap: 10ms}", None, None);
    let prepared = answer.result.clone().unwrap_or_else(|error| panic!("the job starts: {error:?}"));
    assert_eq!(
        field(&prepared, "prepared"),
        Some(&record(&[("n", NuVal::Int(21)), ("nap", NuVal::Duration(10_000_000))])),
    );
    let job = field(&prepared, "job").expect("the job's record");
    assert!(matches!(field(job, "id"), Some(NuVal::Int(_))), "got {job:?}");
    assert!(matches!(field(job, "started"), Some(NuVal::Date(_))), "got {job:?}");
    let nonce = answer.nonce.to_string();
    let (result, events) = event_of(&nonce, "mcp/await/Result");
    let models: Vec<&str> = events.iter().map(|event| event.model.as_str()).collect();
    assert_eq!(models, vec!["mcp/await/Result"], "the Result alone, no Start");
    let expected = record(&[
        ("await", NuVal::String(nonce.clone())),
        ("result", record(&[("doubled", NuVal::Int(42))])),
    ]);
    assert_eq!(result.data, expected, "the call's nonce as `await`, then execute's value");
    assert!(e.wait_for(|rows| !lists(rows, &nonce)), "the job leaves the list");
    let trace_file = e.root.join("cache").join("await").join(&nonce).join(TRACE_FILE);
    let trace = std::fs::read_to_string(&trace_file).expect("the job's trace");
    assert!(trace.lines().any(|line| line.contains("mcp/await/Result")), "the trace:\n{trace}");
}

#[test]
fn a_beat_runs_status_while_the_job_runs() {
    let e = TESTING.harness();
    let beats = NuVal::List(vec![NuVal::Duration(200_000_000), NuVal::Duration(200_000_000)]);
    let answer = e.await_suite(&suite(), "{x: 1, nap: 2sec}", None, Some(beats));
    let prepared = answer.result.clone().unwrap_or_else(|error| panic!("the job starts: {error:?}"));
    let started = nanos(field(&prepared, "job").and_then(|job| field(job, "started")));
    let nonce = answer.nonce.to_string();
    let (_, events) = event_of(&nonce, "mcp/await/Result");
    let status = events
        .iter()
        .find(|event| event.model == "mcp/await/Status")
        .unwrap_or_else(|| panic!("a beat while the job ran; got {events:?}"));
    let expected = record(&[
        ("await", NuVal::String(nonce.clone())),
        ("status", record(&[("beating", NuVal::Bool(true))])),
    ]);
    assert_eq!(status.data, expected, "the call's nonce as `await`, then status's value");
    let trace_file = e.root.join("cache").join("await").join(&nonce).join(TRACE_FILE);
    let trace = std::fs::read_to_string(&trace_file).expect("the job's trace");
    let line = trace
        .lines()
        .find(|line| line.contains("mcp/await/Status"))
        .unwrap_or_else(|| panic!("no Status in the trace:\n{trace}"));
    let traced = NuVal::from_nuon(line).unwrap_or_else(|e| panic!("a trace line is NUON: {e}\n{line}"));
    let gap = nanos(field(&traced, "at")) - started;
    assert!(gap >= 190_000_000, "the first Status comes a heartbeat after the job's start, not {gap} ns after");
}

#[test]
fn a_stop_by_hand_runs_stop_then_sends_its_stop() {
    let e = TESTING.harness();
    let answer = e.await_suite(&suite(), "{x: 1, nap: 30sec}", None, None);
    let nonce = answer.nonce.to_string();
    let job_id = answer
        .result
        .as_ref()
        .ok()
        .and_then(|prepared| field(prepared, "job"))
        .and_then(|job| field(job, "id"))
        .cloned()
        .unwrap_or_else(|| panic!("the job's id; got {:?}", answer.result));
    assert!(e.wait_for(|rows| lists(rows, &nonce)), "the job lists as running");
    assert_eq!(e.control(EngineCommand::AwaitStop { nonce: nonce.clone() }), ControlAnswer::Done);
    let (stopped, events) = event_of(&nonce, "mcp/await/Stop");
    let expected = record(&[
        ("await", NuVal::String(nonce.clone())),
        ("reason", NuVal::String("stop".to_string())),
        ("result", record(&[("stopped", job_id)])),
    ]);
    assert_eq!(stopped.data, expected, "the call's nonce as `await`, the reason, then stop's value");
    assert!(e.wait_for(|rows| !lists(rows, &nonce)), "the stopped job leaves the list");
    let events = events_for(&nonce, |_| false).into_iter().chain(events).collect::<Vec<_>>();
    assert!(
        events.iter().all(|event| event.model != "mcp/await/Result"),
        "a stopped job sends no Result; got {events:?}",
    );
}

#[test]
fn execute_past_its_timeout_runs_stop() {
    let e = TESTING.harness();
    let timeouts = record(&[
        ("prepare", NuVal::Nothing),
        ("execute", NuVal::Duration(500_000_000)),
        ("status", NuVal::Nothing),
        ("stop", NuVal::Nothing),
    ]);
    let answer = e.await_suite(&suite(), "{x: 1, nap: 30sec}", Some(timeouts), None);
    assert!(answer.result.is_ok(), "got {:?}", answer.result);
    let (stopped, _) = event_of(&answer.nonce.to_string(), "mcp/await/Stop");
    assert_eq!(field(&stopped.data, "reason"), Some(&NuVal::String("timeout".to_string())));
}

#[test]
fn the_await_commands_list_a_job_run_its_status_and_refuse_a_stranger() {
    let e = TESTING.harness();
    let answer = e.await_suite(&suite(), "{x: 1, nap: 30sec}", None, None);
    let prepared = answer.result.clone().unwrap_or_else(|error| panic!("the job starts: {error:?}"));
    let job = field(&prepared, "job").expect("the job's record");
    let nonce = answer.nonce.to_string();
    let rows = e.awaits();
    let row = rows
        .iter()
        .find(|row| field(row, "await") == Some(&NuVal::String(nonce.clone())))
        .unwrap_or_else(|| panic!("the job lists; got {rows:?}"));
    let expected = record(&[
        ("await", NuVal::String(nonce.clone())),
        ("job", field(job, "id").cloned().expect("the job's id")),
        ("started", field(job, "started").cloned().expect("the job's start")),
    ]);
    assert_eq!(row, &expected, "a row is the job's await, its id, and its start");
    let ControlAnswer::Value(status) = e.control(EngineCommand::AwaitStatus { nonce: nonce.clone() }) else {
        panic!("await status answered otherwise");
    };
    let beating = record(&[
        ("await", NuVal::String(nonce.clone())),
        ("status", record(&[("beating", NuVal::Bool(true))])),
    ]);
    assert_eq!(status, beating, "status run now, answered as a Status packet's data");
    let events = events_for(&nonce, |_| false);
    assert!(events.is_empty(), "a status asked for sends no packet; got {events:?}");
    let stranger = EngineCommand::AwaitStop {
        nonce: "noSuchAwait".to_string(),
    };
    let refused = e.rt.block_on(e.engine.control(stranger));
    assert!(matches!(refused, Err(EngineError::AwaitUnknown { .. })), "got {refused:?}");
    assert_eq!(e.control(EngineCommand::AwaitStop { nonce: nonce.clone() }), ControlAnswer::Done);
    let status = EngineCommand::AwaitStatus { nonce: nonce.clone() };
    assert!(e.wait_for(|rows| !lists(rows, &nonce)), "the stopped job leaves the list");
    let refused = e.rt.block_on(e.engine.control(status));
    assert!(matches!(refused, Err(EngineError::AwaitUnknown { .. })), "a stopped job is gone; got {refused:?}");
}

#[test]
fn a_suite_breaking_its_contract_is_refused() {
    let e = TESTING.harness();
    let refused = |module: &str| -> Vec<String> {
        match e.await_suite(module, "{x: 1, nap: 10ms}", None, None).result {
            Err(EngineError::SuiteViolations { diagnostics }) => {
                diagnostics.into_iter().map(|row| row.message).collect()
            }
            other => panic!("a suite violation; got {other:?}"),
        }
    };
    let alone = "export def prepare [data: nothing]: nothing -> nothing { }";
    let messages = refused(alone);
    assert!(messages.iter().any(|m| m == "a suite exports `execute`"), "got {messages:?}");
    let messages = refused(&format!("{}\nexport def helper [] {{ }}\n", suite()));
    assert!(messages.iter().any(|m| m.contains("`helper` stays unexported")), "got {messages:?}");
    let narrow = "record<job: record<id: int, started: datetime>, prepared: record<n: int>>";
    let messages = refused(&suite_with(narrow));
    assert!(
        messages.iter().any(|m| m.starts_with("`status`'s `data` is the PREPARED record")),
        "got {messages:?}",
    );
    let messages = refused(&suite().replace("export def stop", "export def --env stop"));
    assert!(messages.iter().any(|m| m.contains("`--env` is refused")), "got {messages:?}");
}

#[test]
fn await_refuses_timeouts_and_heartbeats_off_their_shapes() {
    let e = TESTING.harness();
    let one = NuVal::List(vec![NuVal::Duration(1_000_000_000)]);
    let answer = e.await_suite(&suite(), "{x: 1, nap: 10ms}", None, Some(one));
    assert!(matches!(answer.result, Err(EngineError::HeartbeatsInvalid { .. })), "got {:?}", answer.result);
    let stray = record(&[("prepare", NuVal::Nothing), ("after", NuVal::Nothing)]);
    let answer = e.await_suite(&suite(), "{x: 1, nap: 10ms}", Some(stray), None);
    assert!(matches!(answer.result, Err(EngineError::TimeoutInvalid { .. })), "got {:?}", answer.result);
    let zero = record(&[
        ("prepare", NuVal::Duration(0)),
        ("execute", NuVal::Nothing),
        ("status", NuVal::Nothing),
        ("stop", NuVal::Nothing),
    ]);
    let answer = e.await_suite(&suite(), "{x: 1, nap: 10ms}", Some(zero), None);
    assert!(
        matches!(answer.result, Err(EngineError::TimeoutInvalid { .. })),
        "only execute takes 0ns; got {:?}",
        answer.result,
    );
}

/// A record's value from its fields.
fn record(fields: &[(&str, NuVal)]) -> NuVal {
    NuVal::Record(fields.iter().map(|(k, v)| (k.to_string(), v.clone())).collect())
}

const DOUBLER: &str =
    "def execute [data: record<x: int>]: nothing -> record<y: int> {\n    { y: ($data.x * 2) }\n}";

#[test]
fn execute_answers_its_value() {
    let e = TESTING.harness();
    let answer = e.execute(DOUBLER, "{x: 21}");
    assert_eq!(answer.result, Ok(record(&[("y", NuVal::Int(42))])));
}

#[test]
fn rerun_runs_the_cached_definition_on_fresh_data() {
    let e = TESTING.harness();
    let first = e.execute(DOUBLER, "{x: 2}");
    assert_eq!(first.result, Ok(record(&[("y", NuVal::Int(4))])));
    let again = e.rerun(&first.nonce.to_string(), "{x: 5}");
    assert_eq!(again.result, Ok(record(&[("y", NuVal::Int(10))])));
    assert_ne!(again.nonce, first.nonce, "a rerun answers under its own nonce");
}

#[test]
fn a_definition_breaking_the_contract_is_refused() {
    let e = TESTING.harness();
    let answer = e.execute(
        "def execute [data: record<>]: nothing -> record<y: int> {\n    { y: 1 }\n}",
        "{}",
    );
    assert!(
        matches!(answer.result, Err(EngineError::DefinitionViolations { .. })),
        "got {:?}",
        answer.result,
    );
}

#[test]
fn a_rerun_of_an_unknown_nonce_is_refused() {
    let e = TESTING.harness();
    let answer = e.rerun("noSuchNonce1", "{x: 1}");
    assert!(
        matches!(answer.result, Err(EngineError::RerunBodyMissing { .. })),
        "got {:?}",
        answer.result,
    );
}

#[test]
fn a_remote_host_is_refused() {
    let e = TESTING.harness();
    let peer = Host::Remote(datum::Nom::from("peer"));
    let answer = e.execute_on(peer, DOUBLER, "{x: 1}");
    assert!(
        matches!(answer.result, Err(EngineError::HostUnserved { .. })),
        "got {:?}",
        answer.result,
    );
    let control = e.rt.block_on(e.engine.request(ControlRequest {
        host: peer,
        command: EngineCommand::AwaitList,
    }));
    assert!(
        matches!(control, Ok(Err(EngineError::HostUnserved { .. }))),
        "got {control:?}",
    );
}

#[test]
fn a_body_send_reaches_the_host() {
    let e = TESTING.harness();
    let answer = e.execute(
        "def execute [data: nothing]: nothing -> record<event_id: string> {\n    grmr channel send 'test/engine' {n: 1}\n}",
        "null",
    );
    assert_eq!(
        answer.result,
        Ok(record(&[("event_id", NuVal::String("sent-test-engine".to_string()))])),
    );
    let sent = SENT.lock().unwrap_or_else(|e| e.into_inner());
    assert!(sent.iter().any(|m| m == "test/engine"), "the host saw {sent:?}");
}

#[test]
fn call_runs_a_committed_rig_function() {
    let e = TESTING.harness();
    let source = e.equip_source("sourcetrait/enginecalc");
    let created = e.control(EngineCommand::EquipNew {
        source: source.to_string_lossy().into_owned(),
    });
    assert!(matches!(created, ControlAnswer::Created(_)), "got {created:?}");
    let scaffolded = e.control(EngineCommand::EquipScaffold {
        namepaths: vec!["sourcetrait/enginecalc:math:double".to_string()],
    });
    assert!(matches!(scaffolded, ControlAnswer::Created(_)), "got {scaffolded:?}");
    std::fs::write(
        source.join("rig/math/double/mod.nu"),
        "# Doubles x\nexport def main [data: record<x: int>]: nothing -> record<out: int> {\n    { out: ($data.x * 2) }\n}\n",
    )
    .expect("the function's source");
    let committed = e.control(EngineCommand::EquipCommit {
        namepath: "sourcetrait/enginecalc".to_string(),
    });
    assert!(matches!(committed, ControlAnswer::Committed(_)), "got {committed:?}");

    let answer = e.call("sourcetrait/enginecalc:math:double", "{x: 7}");
    assert_eq!(answer.result, Ok(record(&[("out", NuVal::Int(14))])));

    let ControlAnswer::Signatures(block) = e.control(EngineCommand::Signatures {
        patterns: vec!["*".to_string()],
    }) else {
        panic!("signatures answered otherwise");
    };
    assert!(block.contains("double <x:int> <out:int>"), "the block:\n{block}");
    let help = e.control(EngineCommand::Help {
        namepath: "sourcetrait/enginecalc:math:double".to_string(),
    });
    assert!(matches!(help, ControlAnswer::Help(_)), "got {help:?}");
}

#[test]
fn a_call_to_an_unregistered_rig_is_refused() {
    let e = TESTING.harness();
    let answer = e.call("sourcetrait/ghost:m:noop", "null");
    assert!(
        matches!(answer.result, Err(EngineError::EquipNotRegistered { .. })),
        "got {:?}",
        answer.result,
    );
}

#[test]
fn health_reports_the_eval_permits() {
    let e = TESTING.harness();
    let ControlAnswer::Health(health) = e.control(EngineCommand::Health) else {
        panic!("health answered otherwise");
    };
    assert!(health.permits_cap > 0, "got {health:?}");
    assert!(health.permits_held <= health.permits_cap, "got {health:?}");
}

#[test]
fn a_second_system_starts_and_stops() {
    let e = TESTING.harness();
    let second = e.rt.block_on(EngineHandle::start(TestHost)).expect("a second system");
    let answer = e.rt.block_on(second.control(EngineCommand::AwaitList));
    assert!(matches!(answer, Ok(ControlAnswer::Value(_))), "got {answer:?}");
    e.rt.block_on(second.stop()).expect("the system stops");
}
