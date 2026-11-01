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
    CallRequest, ControlAnswer, ControlRequest, EVAL_STACK_SIZE, EngineCommand, EngineError,
    EngineHandle, EngineParams, EngineSystem, ExecuteRequest, FromEngine, Host, HostAnswer,
    HostAsk, ProcessRow, RerunRequest, ToEngine, ValueAnswer, ensure_equip_store, init,
};

static TESTING: testing::ModuleWith<TestEngine> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|module| TestEngine::new(module.temp_dir()))
});

/// The model of each Channel send the host was handed.
static SENT: Mutex<Vec<String>> = Mutex::new(Vec::new());

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
            FromEngine::Wire(_) | FromEngine::Purview(_) => None,
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
            mcp_nom: "testnom".to_string(),
            data_home: root.join("data"),
            cache_home: root.join("cache"),
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

    fn execute_on(&self, host: Host, definition: &str, data: &str) -> ValueAnswer {
        let req = ExecuteRequest {
            host,
            nonce: self.nonces.generate(),
            definition: definition.to_string(),
            data: data.to_string(),
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
            data: data.to_string(),
            timeout: None,
        };
        self.rt.block_on(self.engine.request(req)).expect("the engine answers")
    }

    fn call(&self, namepath: &str, data: &str) -> ValueAnswer {
        let req = CallRequest {
            host: Host::Local,
            nonce: self.nonces.generate(),
            namepath: namepath.to_string(),
            data: data.to_string(),
            timeout: None,
        };
        self.rt.block_on(self.engine.request(req)).expect("the engine answers")
    }

    fn control(&self, command: EngineCommand) -> ControlAnswer {
        self.rt
            .block_on(self.engine.control(command.clone()))
            .unwrap_or_else(|e| panic!("{command:?} refused: {e:?}"))
    }

    fn processes(&self) -> Vec<ProcessRow> {
        match self.control(EngineCommand::Processes) {
            ControlAnswer::Processes(rows) => rows,
            other => panic!("processes answered {other:?}"),
        }
    }

    /// Polls the process list until `done` holds of it, or five seconds pass.
    fn wait_for(&self, done: impl Fn(&[ProcessRow]) -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if done(&self.processes()) {
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
        command: EngineCommand::Processes,
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
        "def execute [data: nothing]: nothing -> record<event_id: string> {\n    grimm channel send 'test/engine' {n: 1}\n}",
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
fn kill_ends_a_call_in_flight() {
    let e = TESTING.harness();
    let nonce = e.nonces.generate();
    let key = nonce.to_string();
    let engine = e.engine.clone();
    let pending = e.rt.spawn(async move {
        engine
            .request(ExecuteRequest {
                host: Host::Local,
                nonce,
                definition: "def execute [data: nothing]: nothing -> record<out: int> {\n    sleep 30sec\n    { out: 0 }\n}".to_string(),
                data: "null".to_string(),
                timeout: None,
            })
            .await
    });
    assert!(
        e.wait_for(|rows| rows.iter().any(|r| r.nonce == key && r.tool == "execute")),
        "the call lists in flight",
    );
    assert_eq!(e.control(EngineCommand::Kill { nonce: key.clone() }), ControlAnswer::Done);
    let answer = e
        .rt
        .block_on(pending)
        .expect("the request's task")
        .expect("the engine answers");
    assert!(answer.result.is_err(), "a killed call answers an error; got {:?}", answer.result);
    assert!(
        e.wait_for(|rows| rows.iter().all(|r| r.nonce != key)),
        "the killed call leaves the list",
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
    let answer = e.rt.block_on(second.control(EngineCommand::Processes));
    assert!(matches!(answer, Ok(ControlAnswer::Processes(_))), "got {answer:?}");
    e.rt.block_on(second.stop()).expect("the system stops");
}
