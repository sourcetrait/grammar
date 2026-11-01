//! A worker over pipes: its hello, its calls and asks, its refusals, and its end.

mod support;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use sourcetrait_common::datum;
use sourcetrait_grammar_client::{Session, SessionEvent};
use sourcetrait_grammar_protocol::{
    ControlAnswer, ControlRequest, EngineCommand, EngineRequest, EngineResponse, ExecuteRequest, FromEngineSys,
    HelloReply, Host, HostAnswer, Instance, LIMITS_DEFAULT, Limits, McpFrame, NuVal, SessionAsk, SessionClose,
    ToEngineSys, TransportError, ValueAnswer, WorkerFrame,
};
use sourcetrait_grammar_scratch::NamespaceLock;
use support::{RawSession, TestInstance, exit_within, hello, open, runtime, spawn_worker, stderr_of};

/// A definition that sleeps past any test's wait, until it is cancelled.
const SLEEP: &str = "def execute [data: nothing]: nothing -> nothing { sleep 30sec }";

/// How long a worker may take to exit once its session ends.
const EXIT_DEADLINE: Duration = Duration::from_secs(30);

/// `definition` run on null data under `nonce`.
fn execute(nonce: datum::Nonce, definition: &str) -> EngineRequest {
    EngineRequest::Wire(ToEngineSys::Execute(ExecuteRequest {
        host: Host::Local,
        nonce,
        definition: definition.to_string(),
        data: "null".to_string(),
        timeout: None,
    }))
}

/// A control command's answer, its failure a panic.
async fn control(session: &Session, command: EngineCommand) -> ControlAnswer {
    let request = EngineRequest::Wire(ToEngineSys::Control(ControlRequest { host: Host::Local, command }));
    match session.request(request).await.expect("a response") {
        EngineResponse::Wire(FromEngineSys::Control(Ok(answer))) => answer,
        other => panic!("a control answer, not {other:?}"),
    }
}

/// A call's value, its failure a panic.
async fn value(session: &Session, request: EngineRequest) -> NuVal {
    match session.request(request).await.expect("a response") {
        EngineResponse::Wire(FromEngineSys::Value(ValueAnswer { result: Ok(value), .. })) => value,
        other => panic!("a value, not {other:?}"),
    }
}

/// A record of `fields`, each a name and its value.
fn record(fields: &[(&str, NuVal)]) -> NuVal {
    NuVal::Record(fields.iter().map(|(name, value)| (name.to_string(), value.clone())).collect())
}

/// The first call the engine lists in flight, waited for.
async fn first_in_flight(session: &Session) -> String {
    for _ in 0..100 {
        if let ControlAnswer::Processes(rows) = control(session, EngineCommand::Processes).await
            && let Some(row) = rows.first()
        {
            return row.nonce.clone();
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("no call in flight in 10 s");
}

#[test]
fn a_session_serves_info_and_a_call() {
    let instance = TestInstance::new("fixture");
    runtime().block_on(async {
        let (mut child, opened) = open(&instance, "default", LIMITS_DEFAULT).await;
        let (session, _events) = opened.expect("the session opens");
        assert_eq!(session.worker().instance, Instance::new("fixture", "default").unwrap());
        assert_eq!(session.worker().effective, LIMITS_DEFAULT);
        match session.request(EngineRequest::Info).await.expect("info") {
            EngineResponse::Info(info) => {
                assert_eq!((info.id.as_str(), info.namespace.as_str()), ("fixture", "default"));
                assert_eq!(info.work_dir, instance.workdir().display().to_string());
                assert_eq!(info.nom, session.worker().nom);
                assert!(Path::new(&info.tmp).is_dir() && Path::new(&info.transient).is_dir());
            }
            other => panic!("info, not {other:?}"),
        }
        let nonces = datum::NonceGenerator::new();
        let answer = value(&session, execute(nonces.generate(), "def execute [data: nothing]: nothing -> record<n: int> { {n: 42} }")).await;
        assert_eq!(answer, record(&[("n", NuVal::Int(42))]));
        session.close();
        let status = exit_within(&mut child, EXIT_DEADLINE).await;
        let stderr = stderr_of(&mut child).await;
        assert!(status.success(), "{status:?}: {stderr}");
        assert!(stderr.contains("the MCP closed the session"), "{stderr}");
    });
}

#[test]
fn a_kill_cancels_a_call_in_flight() {
    let instance = TestInstance::new("fixture");
    runtime().block_on(async {
        let (_child, opened) = open(&instance, "default", LIMITS_DEFAULT).await;
        let session = Arc::new(opened.expect("the session opens").0);
        let nonce = datum::NonceGenerator::new().generate();
        let call = tokio::spawn({
            let session = session.clone();
            async move { session.request(execute(nonce, SLEEP)).await }
        });
        let listed = first_in_flight(&session).await;
        assert_eq!(listed, nonce.to_string(), "the engine lists the call by its nonce");
        control(&session, EngineCommand::Kill { nonce: listed }).await;
        let answer = tokio::time::timeout(Duration::from_secs(10), call).await.expect("the call ends").unwrap();
        assert!(
            matches!(answer, Ok(EngineResponse::Wire(FromEngineSys::Value(ValueAnswer { result: Err(_), .. })))),
            "the cancelled call answers its failure: {answer:?}",
        );
    });
}

#[test]
fn calls_past_the_sessions_bound_are_refused_busy() {
    let instance = TestInstance::new("fixture");
    runtime().block_on(async {
        let requested = Limits { calls: 1, ..LIMITS_DEFAULT };
        let (_child, opened) = open(&instance, "default", requested).await;
        let session = Arc::new(opened.expect("the session opens").0);
        assert_eq!(session.worker().effective.calls, 1);
        let nonces = datum::NonceGenerator::new();
        let first = nonces.generate();
        let held = tokio::spawn({
            let session = session.clone();
            async move { session.request(execute(first, SLEEP)).await }
        });
        first_in_flight(&session).await;
        let refused = session.request(execute(nonces.generate(), SLEEP)).await.expect("a response");
        assert!(matches!(refused, EngineResponse::Refused(TransportError::Busy)), "{refused:?}");
        control(&session, EngineCommand::Kill { nonce: first.to_string() }).await;
        let _ = tokio::time::timeout(Duration::from_secs(10), held).await;
    });
}

#[test]
fn a_result_past_the_frame_is_refused_in_its_place() {
    let instance = TestInstance::new("fixture");
    runtime().block_on(async {
        let requested = Limits { frame: 64 * 1024, ..LIMITS_DEFAULT };
        let (_child, opened) = open(&instance, "default", requested).await;
        let (session, _events) = opened.expect("the session opens");
        let big = "def execute [data: nothing]: nothing -> record<s: string> { {s: ('' | fill --character 'x' --width 200000)} }";
        let answer = session.request(execute(datum::NonceGenerator::new().generate(), big)).await.expect("a response");
        assert!(
            matches!(answer, EngineResponse::Refused(TransportError::ResultTooLarge { limit: 65536 })),
            "{answer:?}",
        );
        assert!(matches!(session.request(EngineRequest::Info).await, Ok(EngineResponse::Info(_))), "the session goes on");
    });
}

#[test]
fn an_evals_asks_reach_the_mcp_and_its_answers_come_back() {
    let instance = TestInstance::new("fixture");
    instance.write_config("[channel]\nspam_warn_rate = 3\n\n[supervisor]\ncpu_warn_fraction = 0.5\n");
    runtime().block_on(async {
        let (_child, opened) = open(&instance, "default", LIMITS_DEFAULT).await;
        let (session, mut events) = opened.expect("the session opens");
        let mcp_config = record(&[
            ("channel", record(&[("spam_warn_rate", NuVal::Int(10))])),
            ("supervisor", record(&[("cpu_warn_fraction", NuVal::Float(0.9))])),
            ("identity", NuVal::Nothing),
        ]);
        tokio::spawn(async move {
            while let Some(event) = events.recv().await {
                if let SessionEvent::Ask { ask, responder } = event {
                    responder.answer(match ask {
                        SessionAsk::ChannelSend(data) if data.model == "test/Model" => HostAnswer::Sent("sent1".to_string()),
                        SessionAsk::Config => HostAnswer::Config(mcp_config.clone()),
                        other => HostAnswer::Refused(format!("{other:?}")),
                    });
                }
            }
        });
        let nonces = datum::NonceGenerator::new();
        let send = "def execute [data: nothing]: nothing -> record<event_id: string> { grimm channel send test/Model {x: 1} }";
        assert_eq!(value(&session, execute(nonces.generate(), send)).await, record(&[("event_id", NuVal::String("sent1".to_string()))]));
        let config = "def execute [data: nothing]: nothing -> record<cpu: float, rate: int> { let c = (grimm get_config_all); {cpu: $c.supervisor.cpu_warn_fraction, rate: $c.channel.spam_warn_rate} }";
        assert_eq!(
            value(&session, execute(nonces.generate(), config)).await,
            record(&[("cpu", NuVal::Float(0.5)), ("rate", NuVal::Int(10))]),
            "the MCP's config, the worker's own supervisor lines in it",
        );
    });
}

#[test]
fn a_second_worker_on_a_namespace_is_refused() {
    let instance = TestInstance::new("fixture");
    runtime().block_on(async {
        let (_first, opened) = open(&instance, "default", LIMITS_DEFAULT).await;
        let (_session, _events) = opened.expect("the first session opens");
        let (mut second, refused) = open(&instance, "default", LIMITS_DEFAULT).await;
        let refusal = refused.expect_err("the second is refused");
        assert_eq!(refusal.kind(), "remote::engine::unavailable", "{refusal:?}");
        assert!(refusal.message().contains("runtime::namespace_held"), "{}", refusal.message());
        let status = exit_within(&mut second, EXIT_DEADLINE).await;
        assert_eq!(status.code(), Some(75), "{status:?}");
        assert_eq!(TestInstance::worker_dirs(&instance.home("tmp")).len(), 1, "the refused worker made no scratch");
    });
}

#[test]
fn the_end_of_stdin_ends_the_session_its_calls_and_its_files() {
    let instance = TestInstance::new("fixture");
    runtime().block_on(async {
        let mut child = spawn_worker(&instance, "default");
        let (mut raw, reply) = RawSession::open(&mut child, hello(&instance, "default", LIMITS_DEFAULT)).await;
        assert!(matches!(reply, HelloReply::Accepted(_)), "{reply:?}");
        let nonce = datum::NonceGenerator::new().generate();
        raw.send(&McpFrame::Request { id: 1, request: execute(nonce, SLEEP) }).await;
        let cache = instance.load("default").paths.cache.join("execute").join(nonce.to_string());
        for _ in 0..100 {
            if cache.is_dir() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(cache.is_dir(), "the call's cache entry, made as it runs");
        drop(raw.sink);
        let status = exit_within(&mut child, EXIT_DEADLINE).await;
        let stderr = stderr_of(&mut child).await;
        assert!(status.success(), "{status:?}: {stderr}");
        assert!(stderr.contains("the MCP's stream ended"), "{stderr}");
        assert!(!cache.exists(), "the session's call cache removed at its end");
        assert!(TestInstance::worker_dirs(&instance.home("tmp")).is_empty(), "its scratch removed");
        NamespaceLock::acquire(&instance.load("default").paths.lock).expect("its lock released");
    });
}

#[test]
fn a_session_with_no_frame_in_its_window_ends() {
    let instance = TestInstance::new("fixture");
    runtime().block_on(async {
        let mut child = spawn_worker(&instance, "default");
        let (mut raw, reply) = RawSession::open(&mut child, hello(&instance, "default", LIMITS_DEFAULT)).await;
        assert!(matches!(reply, HelloReply::Accepted(_)), "{reply:?}");
        let frame = tokio::time::timeout(Duration::from_secs(45), raw.next()).await.expect("a frame in the window's time");
        assert!(
            matches!(frame, Some(WorkerFrame::Close(SessionClose { error: Some(TransportError::SessionLost { .. }) }))),
            "{frame:?}",
        );
        let status = exit_within(&mut child, EXIT_DEADLINE).await;
        let stderr = stderr_of(&mut child).await;
        assert!(status.success(), "{status:?}: {stderr}");
        assert!(stderr.contains("no frame from the MCP in 30 s"), "{stderr}");
    });
}

#[test]
fn a_hello_for_another_namespace_is_refused() {
    let instance = TestInstance::new("fixture");
    runtime().block_on(async {
        let mut child = spawn_worker(&instance, "default");
        let (_raw, reply) = RawSession::open(&mut child, hello(&instance, "other", LIMITS_DEFAULT)).await;
        assert!(matches!(reply, HelloReply::Refused(TransportError::WrongWorker { .. })), "{reply:?}");
        let status = exit_within(&mut child, EXIT_DEADLINE).await;
        assert_eq!(status.code(), Some(76), "{status:?}");
        assert!(TestInstance::worker_dirs(&instance.home("tmp")).is_empty(), "its scratch removed");
    });
}
