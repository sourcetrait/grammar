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

/// How long the test host holds the answer to a Status whose value asks it to.
const HELD_SEND: Duration = Duration::from_millis(2500);

/// Whether an await event asks its send held: a Status whose value is `hold: true`.
fn holds(event: &AwaitEvent) -> bool {
    event.model == "mcp/await/Status"
        && field(&event.data, "status").and_then(|status| field(status, "hold")) == Some(&NuVal::Bool(true))
}

/// The engine's controller: each Channel send's model kept and answered an id,
/// a held Status's answer given once `HELD_SEND` has passed.
struct TestHost;

impl subsys::Handler<EngineSystem> for TestHost {
    fn on_packet(&mut self, pkt: subsys::Packet<FromEngine>) -> impl Future<Output = Option<subsys::Packet<FromEngine>>> + 'static + Send {
        std::future::ready(Some(pkt))
    }

    fn on_request(&mut self, pkt: subsys::Packet<FromEngine>) -> impl Future<Output = Option<ToEngine>> + 'static + Send {
        let mut held = false;
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
                held = holds(&event);
                EVENTS.lock().unwrap_or_else(|e| e.into_inner()).push(event);
                Some(HostAnswer::Sent(id))
            }
            FromEngine::Wire(_) | FromEngine::Purview(_) | FromEngine::Defined(_) => None,
        };
        async move {
            if held {
                tokio::time::sleep(HELD_SEND).await;
            }
            answer.map(ToEngine::Host)
        }
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
    events_within(nonce, Duration::from_secs(10), done)
}

/// A call's await events, polled until `done` holds of them or `limit` passes.
fn events_within(nonce: &str, limit: Duration, done: impl Fn(&[AwaitEvent]) -> bool) -> Vec<AwaitEvent> {
    let deadline = Instant::now() + limit;
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

/// A suite whose prepare takes `timeouts?` and `heartbeats?`, answering which were given.
fn suite_taking_both() -> String {
    let prepared = "record<job: record<id: int, started: datetime>, prepared: record<timeouts: bool, heartbeats: bool>>";
    format!(
        "export def prepare [data: record<x: int>, timeouts?: record<prepare: oneof<nothing, duration>, execute: oneof<nothing, duration>, status: oneof<nothing, duration>, stop: oneof<nothing, duration>>, heartbeats?: list<duration>]: nothing -> record<timeouts: bool, heartbeats: bool> {{
    {{timeouts: ($timeouts != null), heartbeats: ($heartbeats != null)}}
}}

export def execute [data: {prepared}]: nothing -> record<x: int> {{
    {{x: 1}}
}}

export def status [data: {prepared}]: nothing -> record<beating: bool> {{
    {{beating: true}}
}}

export def stop [data: {prepared}]: nothing -> record<stopped: bool> {{
    {{stopped: true}}
}}
"
    )
}

#[test]
fn prepare_takes_each_optional_parameter_as_given_or_null() {
    let e = TESTING.harness();
    let timeouts = record(&[
        ("prepare", NuVal::Nothing),
        ("execute", NuVal::Nothing),
        ("status", NuVal::Nothing),
        ("stop", NuVal::Nothing),
    ]);
    let beats = NuVal::List(vec![NuVal::Duration(200_000_000), NuVal::Duration(200_000_000)]);
    let cases = [
        (None, None),
        (Some(timeouts.clone()), None),
        (None, Some(beats.clone())),
        (Some(timeouts), Some(beats)),
    ];
    for (timeouts, heartbeats) in cases {
        let expected = record(&[
            ("timeouts", NuVal::Bool(timeouts.is_some())),
            ("heartbeats", NuVal::Bool(heartbeats.is_some())),
        ]);
        let answer = e.await_suite(&suite_taking_both(), "{x: 1}", timeouts, heartbeats);
        let prepared = answer.result.unwrap_or_else(|error| panic!("the job starts: {error:?}"));
        assert_eq!(field(&prepared, "prepared"), Some(&expected), "each as the call gave it, null where absent");
    }
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

/// The PREPARED record of a suite whose prepare answers nothing.
const PREPARED_NOTHING: &str = "record<job: record<id: int, started: datetime>, prepared: nothing>";

/// A suite of nothing's data: prepare's, execute's, and stop's bodies as given.
fn suite_of_nothing(prepare: &str, execute: &str, stop: &str) -> String {
    format!(
        "export def prepare [data: nothing]: nothing -> nothing {{ {prepare} }}
export def execute [data: {PREPARED_NOTHING}]: nothing -> nothing {{ {execute} }}
export def status [data: {PREPARED_NOTHING}]: nothing -> nothing {{ }}
export def stop [data: {PREPARED_NOTHING}]: nothing -> nothing {{ {stop} }}
"
    )
}

/// The moments a job's trace holds of `model`, in nanoseconds since its start.
fn traced_since(e: &TestEngine, nonce: &str, model: &str, started: i128) -> Vec<i128> {
    let trace_file = e.root.join("cache").join("await").join(nonce).join(TRACE_FILE);
    let trace = std::fs::read_to_string(&trace_file).unwrap_or_else(|err| panic!("the trace: {err}"));
    trace
        .lines()
        .filter(|line| line.contains(model))
        .map(|line| {
            let event = NuVal::from_nuon(line).unwrap_or_else(|err| panic!("a trace line is NUON: {err}"));
            nanos(field(&event, "at")) - started
        })
        .collect()
}

#[test]
fn beats_hold_their_schedule_past_a_slow_status() {
    let e = TESTING.harness();
    let slow = suite().replace("{beating: true}", "sleep 500ms; {beating: true}");
    let second = NuVal::Duration(1_000_000_000);
    let beats = NuVal::List(vec![second.clone(), second]);
    let answer = e.await_suite(&slow, "{x: 1, nap: 3600ms}", None, Some(beats));
    let prepared = answer.result.expect("the job starts");
    let started = nanos(field(field(&prepared, "job").expect("its job"), "started"));
    let nonce = answer.nonce.to_string();
    event_of(&nonce, "mcp/await/Result");
    let beats = traced_since(e, &nonce, "mcp/await/Status", started);
    assert_eq!(beats.len(), 3, "a beat at 1, 2, and 3 seconds, each status 500 ms: {beats:?}");
    for pair in beats.windows(2) {
        let gap = pair[1] - pair[0];
        assert!(
            (750_000_000..=1_250_000_000).contains(&gap),
            "a status sent a second after the last, not {gap} ns later: {beats:?}",
        );
    }
}

#[test]
fn a_job_send_reaches_its_executes_job_recv() {
    let e = TESTING.harness();
    let prepared_type = "record<job: record<id: int, started: datetime>, prepared: record<n: int>>";
    let module = format!(
        "export def prepare [data: record<x: int>]: nothing -> record<n: int> {{ {{n: $data.x}} }}
export def execute [data: {prepared_type}]: nothing -> record<got: int> {{
    let mail = job recv --timeout 10sec
    {{got: $mail.x}}
}}
export def status [data: {prepared_type}]: nothing -> nothing {{ }}
export def stop [data: {prepared_type}]: nothing -> nothing {{ }}
"
    );
    let answer = e.await_suite(&module, "{x: 1}", None, None);
    let prepared = answer.result.expect("the job starts");
    let id = match field(field(&prepared, "job").expect("its job"), "id") {
        Some(NuVal::Int(id)) => *id,
        other => panic!("a job id, not {other:?}"),
    };
    let nonce = answer.nonce.to_string();
    let sender = format!("def execute [data: nothing]: nothing -> nothing {{ {{x: 7}} | job send {id} }}");
    let sent = e.execute(&sender, "null");
    assert!(sent.result.is_ok(), "the send answers: {sent:?}");
    let (result, _) = event_of(&nonce, "mcp/await/Result");
    let expected = NuVal::Record(vec![
        ("await".to_string(), NuVal::String(nonce.clone())),
        ("result".to_string(), NuVal::Record(vec![("got".to_string(), NuVal::Int(7))])),
    ]);
    assert_eq!(result.data, expected, "the mail sent to the job's id is execute's job recv's");
}

#[test]
fn an_await_whose_prepare_ends_in_the_teardown_starts_no_job() {
    let e = TESTING.harness();
    let second = Arc::new(e.rt.block_on(EngineHandle::start(TestHost)).expect("a second system"));
    let request = |module: String| AwaitRequest {
        host: Host::Local,
        nonce: e.nonces.generate(),
        module,
        data: NuVal::Nothing,
        timeouts: None,
        heartbeats: None,
    };
    let first = request(suite_of_nothing("", "sleep 30sec", "sleep 3sec"));
    let first_nonce = first.nonce.to_string();
    let first = e.rt.block_on(second.request(first)).expect("the engine answers");
    assert!(first.result.is_ok(), "the first job starts: {first:?}");
    let late = request(suite_of_nothing("sleep 1500ms", "sleep 30sec", ""));
    let late_nonce = late.nonce.to_string();
    let answer = e
        .rt
        .block_on(async {
            let engine = second.clone();
            let call = tokio::spawn(async move { engine.request(late).await });
            tokio::time::sleep(Duration::from_millis(300)).await;
            let engine = second.clone();
            let teardown = tokio::spawn(async move { engine.control(EngineCommand::Teardown).await });
            let answer = call.await.expect("the call's task");
            teardown.await.expect("the teardown's task").expect("the teardown answers");
            answer
        })
        .expect("the engine answers");
    match &answer.result {
        Err(EngineError::Internal { phase, reason }) => {
            assert_eq!(phase, "await::start", "{reason}");
            assert!(reason.contains("stopping"), "{reason}");
        }
        other => panic!("an await whose prepare ends in the teardown is refused, not {other:?}"),
    }
    let late_trace = e.root.join("cache").join("await").join(&late_nonce).join(TRACE_FILE);
    assert!(!late_trace.exists(), "the refused await never became a job");
    let first_trace = e.root.join("cache").join("await").join(&first_nonce).join(TRACE_FILE);
    let trace = std::fs::read_to_string(&first_trace).expect("the first job's trace");
    assert!(trace.contains("mcp/await/Stop") && trace.contains("teardown"), "the first job's stop ran: {trace}");
    let second = Arc::try_unwrap(second).ok().expect("the one handle left");
    e.rt.block_on(second.stop()).expect("the system stops");
}

/// `to` by a path relative to the process's directory, which every eval's `PWD` is.
fn relative_to_cwd(to: &Path) -> PathBuf {
    let cwd = std::env::current_dir().expect("a working directory");
    let from: Vec<_> = cwd.components().collect();
    let to: Vec<_> = to.components().collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut relative = PathBuf::new();
    for _ in common..from.len() {
        relative.push("..");
    }
    for component in &to[common..] {
        relative.push(component);
    }
    relative
}

#[test]
fn a_suites_import_resolves_where_its_analysis_resolved_it() {
    let e = TESTING.harness();
    let mods = e.root.join("work").join("mods");
    std::fs::create_dir_all(&mods).expect("a module directory");
    let helper = mods.join("helper.nu");
    std::fs::write(&helper, "export def main []: nothing -> int { 20 }\n").expect("the helper");
    let import = relative_to_cwd(&helper);
    let module = format!(
        "use \"{}\"\n{}",
        import.display(),
        suite().replace("{n: $data.x, nap: $data.nap}", "{n: ($data.x + (helper)), nap: $data.nap}"),
    );
    let answer = e.await_suite(&module, "{x: 1, nap: 10ms}", None, None);
    let prepared = answer.result.expect("prepare resolves the import as the analysis did");
    assert_eq!(field(field(&prepared, "prepared").expect("prepare's value"), "n"), Some(&NuVal::Int(21)));
    let (result, _) = event_of(&answer.nonce.to_string(), "mcp/await/Result");
    let doubled = field(field(&result.data, "result").expect("execute's value"), "doubled");
    assert_eq!(doubled, Some(&NuVal::Int(42)), "execute resolves it too");
}

#[test]
fn an_import_resolving_beside_the_cache_alone_is_refused_at_the_analysis() {
    let e = TESTING.harness();
    let nonce = e.nonces.generate();
    let cached = e.root.join("cache").join("await").join(nonce.to_string());
    std::fs::create_dir_all(&cached).expect("the job's cache directory");
    std::fs::write(cached.join("beside.nu"), "export def main []: nothing -> int { 20 }\n").expect("a module beside it");
    let req = AwaitRequest {
        host: Host::Local,
        nonce,
        module: format!("use beside.nu\n{}", suite()),
        data: data_value("{x: 1, nap: 10ms}"),
        timeouts: None,
        heartbeats: None,
    };
    let answer = e.rt.block_on(e.engine.request(req)).expect("the engine answers");
    match &answer.result {
        Err(EngineError::SuiteViolations { diagnostics }) => {
            assert!(diagnostics.iter().any(|row| row.message.contains("beside.nu")), "{diagnostics:?}");
        }
        other => panic!("refused at the analysis, not {other:?}"),
    }
    assert!(!cached.join(TRACE_FILE).exists(), "the refused await never became a job");
}

#[test]
fn a_send_to_a_finished_jobs_id_is_refused() {
    let e = TESTING.harness();
    let answer = e.await_suite(&suite(), "{x: 1, nap: 10ms}", None, None);
    let prepared = answer.result.expect("the job starts");
    let id = match field(field(&prepared, "job").expect("its job"), "id") {
        Some(NuVal::Int(id)) => *id,
        other => panic!("a job id, not {other:?}"),
    };
    let nonce = answer.nonce.to_string();
    event_of(&nonce, "mcp/await/Result");
    assert!(e.wait_for(|rows| !lists(rows, &nonce)), "the finished job leaves the list");
    let sender = format!("def execute [data: nothing]: nothing -> nothing {{ {{x: 7}} | job send {id} }}");
    let deadline = Instant::now() + Duration::from_secs(2);
    let refusal = loop {
        let sent = e.execute(&sender, "null");
        match sent.result {
            Err(error) => break format!("{error:?}"),
            Ok(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(50)),
            Ok(value) => panic!("a send to a finished job's id is refused, not answered {value:?}"),
        }
    };
    assert!(refusal.contains("not found"), "nushell's own refusal of a job it doesn't hold: {refusal}");
}

#[test]
fn beats_after_a_slow_status_come_at_once_then_on_their_times() {
    let e = TESTING.harness();
    let dir = e.root.join("work").join("first-beat");
    std::fs::create_dir_all(&dir).expect("the status's marker directory");
    let prepared_type = "record<job: record<id: int, started: datetime>, prepared: record<dir: string, nap: duration>>";
    let module = format!(
        "export def prepare [data: record<dir: string, nap: duration>]: nothing -> record<dir: string, nap: duration> {{ $data }}
export def execute [data: {prepared_type}]: nothing -> nothing {{ sleep $data.prepared.nap }}
export def status [data: {prepared_type}]: nothing -> record<beating: bool> {{
    let first = ($data.prepared.dir | path join first)
    if not ($first | path exists) {{ touch $first; sleep 1500ms }}
    {{beating: true}}
}}
export def stop [data: {prepared_type}]: nothing -> nothing {{ }}
"
    );
    let second = NuVal::Duration(1_000_000_000);
    let beats = NuVal::List(vec![second.clone(), second]);
    let data = format!("{{dir: {:?}, nap: 4600ms}}", dir.display().to_string());
    let answer = e.await_suite(&module, &data, None, Some(beats));
    let prepared = answer.result.expect("the job starts");
    let started = nanos(field(field(&prepared, "job").expect("its job"), "started"));
    let nonce = answer.nonce.to_string();
    event_of(&nonce, "mcp/await/Result");
    let beats = traced_since(e, &nonce, "mcp/await/Status", started);
    assert_eq!(beats.len(), 4, "beats due at 1, 2, 3, and 4 seconds: {beats:?}");
    let within = |at: i128, from_ms: i128, to_ms: i128| (from_ms * 1_000_000..=to_ms * 1_000_000).contains(&at);
    assert!(within(beats[0], 2500, 3000), "the first beat's 1.5 s status ends past 2.5 s: {beats:?}");
    assert!(
        beats[1] >= beats[0] && beats[1] - beats[0] <= 500_000_000,
        "the beat due at 2 s, passed while the first ran, follows it at once: {beats:?}",
    );
    assert!(within(beats[2], 3000, 3500), "the beat due at 3 s keeps its time: {beats:?}");
    assert!(within(beats[3], 4000, 4500), "the beat due at 4 s keeps its time: {beats:?}");
}

/// A job's traced events, each its model and its data, in the order they were queued.
fn traced_events(e: &TestEngine, nonce: &str) -> Vec<(String, NuVal)> {
    let trace_file = e.root.join("cache").join("await").join(nonce).join(TRACE_FILE);
    let trace = std::fs::read_to_string(&trace_file).unwrap_or_else(|err| panic!("the trace: {err}"));
    trace
        .lines()
        .map(|line| {
            let event = NuVal::from_nuon(line).unwrap_or_else(|err| panic!("a trace line is NUON: {err}"));
            match (field(&event, "model"), field(&event, "data")) {
                (Some(NuVal::String(model)), Some(data)) => (model.clone(), data.clone()),
                other => panic!("a trace line's model and data, not {other:?}"),
            }
        })
        .collect()
}

#[test]
fn execute_runs_unbounded_where_the_call_names_no_bound() {
    let e = TESTING.harness();
    let prepared_type = "record<job: record<id: int, started: datetime>, prepared: record<nap: duration>>";
    let module = format!(
        "export def prepare [data: record<nap: duration>]: nothing -> record<nap: duration> {{ $data }}
export def execute [data: {prepared_type}, timeout?: duration]: nothing -> record<timeout: duration> {{
    sleep $data.prepared.nap
    {{timeout: $timeout}}
}}
export def status [data: {prepared_type}, timeout?: duration]: nothing -> record<timeout: duration> {{ {{timeout: $timeout}} }}
export def stop [data: {prepared_type}]: nothing -> nothing {{ }}
"
    );
    let tenth = NuVal::Duration(100_000_000);
    let beats = NuVal::List(vec![tenth.clone(), tenth]);
    let bounds = |timeouts: Option<NuVal>| -> (NuVal, NuVal) {
        let answer = e.await_suite(&module, "{nap: 400ms}", timeouts, Some(beats.clone()));
        assert!(answer.result.is_ok(), "the job starts: {answer:?}");
        let (result, events) = event_of(&answer.nonce.to_string(), "mcp/await/Result");
        let status = events
            .iter()
            .find(|event| event.model == "mcp/await/Status")
            .unwrap_or_else(|| panic!("a Status before the Result; got {events:?}"));
        let timeout = |data: &NuVal, holder: &str| {
            field(field(data, holder).unwrap_or_else(|| panic!("{holder} in {data:?}")), "timeout")
                .cloned()
                .unwrap_or_else(|| panic!("its timeout in {data:?}"))
        };
        (timeout(&status.data, "status"), timeout(&result.data, "result"))
    };
    let two_minutes = NuVal::Duration(120_000_000_000);
    let unbounded = NuVal::Duration(0);
    assert_eq!(bounds(None), (two_minutes.clone(), unbounded.clone()), "no timeouts: status 120sec, execute 0ns");
    let nulls = record(&[
        ("prepare", NuVal::Nothing),
        ("execute", NuVal::Nothing),
        ("status", NuVal::Nothing),
        ("stop", NuVal::Nothing),
    ]);
    assert_eq!(bounds(Some(nulls)), (two_minutes.clone(), unbounded), "a null field takes the same default");
    let five = NuVal::Duration(5_000_000_000);
    let given = record(&[
        ("prepare", NuVal::Nothing),
        ("execute", five.clone()),
        ("status", NuVal::Nothing),
        ("stop", NuVal::Nothing),
    ]);
    assert_eq!(bounds(Some(given)), (two_minutes, five), "a bound the call gives stands");
}

#[test]
fn held_status_sends_keep_the_beats_on_their_times_and_send_the_newest() {
    let e = TESTING.harness();
    let prepared_type = "record<job: record<id: int, started: datetime>, prepared: record<nap: duration>>";
    let module = format!(
        "export def prepare [data: record<nap: duration>]: nothing -> record<nap: duration> {{ $data }}
export def execute [data: {prepared_type}]: nothing -> nothing {{ sleep $data.prepared.nap }}
export def status [data: {prepared_type}]: nothing -> record<hold: bool, at: int> {{
    {{hold: true, at: (date now | into int)}}
}}
export def stop [data: {prepared_type}]: nothing -> nothing {{ }}
"
    );
    let second = NuVal::Duration(1_000_000_000);
    let beats = NuVal::List(vec![second.clone(), second]);
    let answer = e.await_suite(&module, "{nap: 6600ms}", None, Some(beats));
    let prepared = answer.result.expect("the job starts");
    let started = nanos(field(field(&prepared, "job").expect("its job"), "started"));
    let nonce = answer.nonce.to_string();
    let has_result = |events: &[AwaitEvent]| events.iter().any(|event| event.model == "mcp/await/Result");
    let delivered = events_within(&nonce, Duration::from_secs(25), has_result);
    let beats = traced_since(e, &nonce, "mcp/await/Status", started);
    assert_eq!(beats.len(), 6, "statuses at 1 to 6 seconds, each send held 2.5 s: {beats:?}");
    for (index, at) in beats.iter().enumerate() {
        let due = (index as i128 + 1) * 1_000_000_000;
        assert!(
            (due..=due + 500_000_000).contains(at),
            "the status due at {due} ns runs on its time, its send held: {beats:?}",
        );
    }
    let traced = traced_events(e, &nonce);
    let delivered: Vec<(String, NuVal)> = delivered.into_iter().map(|event| (event.model, event.data)).collect();
    let statuses = |events: &[(String, NuVal)]| events.iter().filter(|(model, _)| model == "mcp/await/Status").count();
    assert!(
        statuses(&delivered) < statuses(&traced),
        "a Status superseded while it waited is never sent: {delivered:?}",
    );
    let mut rest = traced.iter();
    assert!(
        delivered.iter().all(|sent| rest.any(|queued| queued == sent)),
        "every event sent was traced, in the trace's order: {delivered:?}",
    );
    let newest = traced.iter().rev().find(|(model, _)| model == "mcp/await/Status").expect("a traced Status");
    assert!(delivered.contains(newest), "the newest Status is sent: {delivered:?}");
    assert_eq!(delivered.last().map(|(model, _)| model.as_str()), Some("mcp/await/Result"), "the Result last");
}
