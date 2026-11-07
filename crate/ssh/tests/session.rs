//! grammar-ssh over pipes: its hello, its calls and asks, its refusals, and its end.

mod support;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use sourcetrait_common::datum;
use sourcetrait_grammar_client::{Session, SessionEvent};
use sourcetrait_grammar_protocol::{
    AWAIT_RESULT_MODEL, AWAIT_STOP_MODEL, AwaitRequest, CallAnswer, EngineRequest, EngineResponse,
    ExecuteDefinition,
    ExecuteRequest, HelloReply, Host,
    HostAnswer, Instance, InterfaceRequest, LIMITS_DEFAULT, Limits, McpFrame, NuType, NuVal, SessionAsk,
    SessionClose, SshFrame, TransportError,
};
use sourcetrait_grammar_scratch::NamespaceLock;
use support::{RawSession, TestInstance, exit_within, hello, open, runtime, spawn_ssh, stderr_of};

/// A definition on `data: nothing`, its result typed `result` and its body `body`.
fn definition(result: &str, body: &str) -> ExecuteDefinition {
    ExecuteDefinition {
        is_env: false,
        data: NuType::Nothing,
        result: NuType::from_text(result).unwrap_or_else(|e| panic!("`{result}` is a type: {e}")),
        timeout: false,
        body: body.to_string(),
    }
}

/// A definition that sleeps past any test's wait, until it is cancelled.
fn sleep() -> ExecuteDefinition {
    definition("nothing", " sleep 30sec ")
}

/// How long grammar-ssh may take to exit once its session ends.
const EXIT_DEADLINE: Duration = Duration::from_secs(30);

/// `definition` run on null data under `nonce`.
fn execute(nonce: datum::Nonce, definition: ExecuteDefinition) -> EngineRequest {
    EngineRequest::Execute(ExecuteRequest {
        host: Host::Local,
        nonce,
        definition,
        data: NuVal::Nothing,
        timeout: None,
    })
}

/// An interface call's subcommand, as the MCP sends it on.
fn interface(subcommand: &str) -> EngineRequest {
    EngineRequest::Interface(InterfaceRequest {
        nonce: datum::NonceGenerator::new().generate(),
        subcommand: subcommand.to_string(),
    })
}

/// A call's value, its failure a panic.
async fn value(session: &Session, request: EngineRequest) -> NuVal {
    match session.request(request).await.expect("a response") {
        EngineResponse::Answer(CallAnswer { result: Ok(value), .. }) => value,
        other => panic!("a value, not {other:?}"),
    }
}

/// A record of `fields`, each a name and its value.
fn record(fields: &[(&str, NuVal)]) -> NuVal {
    NuVal::Record(fields.iter().map(|(name, value)| (name.to_string(), value.clone())).collect())
}

/// A record's field, its absence a panic.
fn field<'a>(value: &'a NuVal, name: &str) -> &'a NuVal {
    match value {
        NuVal::Record(fields) => fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
            .unwrap_or_else(|| panic!("no `{name}` in {value:?}")),
        other => panic!("a record, not {other:?}"),
    }
}

/// A string's text, anything else a panic.
fn text(value: &NuVal) -> &str {
    match value {
        NuVal::String(text) => text,
        other => panic!("a string, not {other:?}"),
    }
}

#[test]
fn a_session_serves_info_and_a_call() {
    let instance = TestInstance::new("fixture");
    runtime().block_on(async {
        let (mut child, opened) = open(&instance, "default", LIMITS_DEFAULT).await;
        let (session, _events) = opened.expect("the session opens");
        assert_eq!(session.ssh().instance, Instance::new("fixture", "default").unwrap());
        assert_eq!(session.ssh().effective, LIMITS_DEFAULT);
        let info = value(&session, interface("interface info")).await;
        assert_eq!((text(field(&info, "id")), text(field(&info, "namespace"))), ("fixture", "default"));
        assert_eq!(text(field(&info, "work_dir")), instance.workdir().display().to_string());
        assert_eq!(text(field(&info, "nom")), session.ssh().nom);
        let transient = field(&info, "transient");
        assert!(Path::new(text(field(&info, "tmp"))).is_dir() && Path::new(text(field(transient, "grammar"))).is_dir());
        assert_eq!(
            text(field(transient, "ai")),
            instance.home("run").join("ai").join("fixture").display().to_string(),
            "an agent's scratch under the runtime home by the id",
        );
        let view = field(&info, "purview");
        assert!(matches!(view, NuVal::List(rows) if !rows.is_empty()), "its signatures' view, its own session's: {info:?}");
        text(field(&info, "signatures"));
        let nonces = datum::NonceGenerator::new();
        let answer = value(&session, execute(nonces.generate(), definition("record<n: int>", " {n: 42} "))).await;
        assert_eq!(answer, record(&[("n", NuVal::Int(42))]));
        let nom = value(&session, execute(nonces.generate(), definition("record<nom: string>", " {nom: $env.EQUIP_MCP_NOM} "))).await;
        assert_eq!(nom, record(&[("nom", NuVal::String(session.ssh().nom.clone()))]), "an eval reads its engine's nom");
        session.close();
        let status = exit_within(&mut child, EXIT_DEADLINE).await;
        let stderr = stderr_of(&mut child).await;
        assert!(status.success(), "{status:?}: {stderr}");
        assert!(stderr.contains("the MCP closed the session"), "{stderr}");
    });
}

#[test]
fn an_await_stop_over_the_session_stops_the_job() {
    let instance = TestInstance::new("fixture");
    runtime().block_on(async {
        let (_child, opened) = open(&instance, "default", LIMITS_DEFAULT).await;
        let (session, mut events) = opened.expect("the session opens");
        let (seen_tx, mut seen_rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            while let Some(event) = events.recv().await {
                if let SessionEvent::Ask { ask, responder } = event {
                    responder.answer(match ask {
                        SessionAsk::Await(event) => {
                            let _ = seen_tx.send(event);
                            HostAnswer::Sent("event1".to_string())
                        }
                        other => HostAnswer::Refused(format!("{other:?}")),
                    });
                }
            }
        });
        let nonce = datum::NonceGenerator::new().generate();
        let request = EngineRequest::Await(AwaitRequest {
            host: Host::Local,
            nonce,
            module: slow_suite(),
            data: NuVal::Nothing,
            timeouts: None,
            heartbeats: None,
        });
        value(&session, request).await;
        let listed = value(&session, interface("await list")).await;
        let NuVal::List(rows) = &listed else {
            panic!("a table, not {listed:?}");
        };
        assert!(
            rows.iter().any(|row| text(field(row, "await")) == nonce.to_string()),
            "the job lists by its await: {listed:?}",
        );
        let stopped = value(&session, interface(&format!("await stop '{nonce}'"))).await;
        assert_eq!(stopped, NuVal::Nothing);
        let event = tokio::time::timeout(Duration::from_secs(10), seen_rx.recv())
            .await
            .expect("a Stop within 10 s")
            .expect("the Stop");
        assert_eq!(event.model, AWAIT_STOP_MODEL);
        assert_eq!(text(field(&event.data, "await")), nonce.to_string());
        assert_eq!(text(field(&event.data, "reason")), "stop");
    });
}

#[test]
fn calls_past_the_sessions_bound_are_refused_busy() {
    let instance = TestInstance::new("fixture");
    runtime().block_on(async {
        let requested = Limits { calls: 1, ..LIMITS_DEFAULT };
        let (_child, opened) = open(&instance, "default", requested).await;
        let (session, mut events) = opened.expect("the session opens");
        let session = Arc::new(session);
        assert_eq!(session.ssh().effective.calls, 1);
        let (asked_tx, mut asked_rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            while let Some(event) = events.recv().await {
                if let SessionEvent::Ask { ask, responder } = event {
                    let _ = asked_tx.send(());
                    responder.answer(match ask {
                        SessionAsk::ChannelSend(_) => HostAnswer::Sent("held1".to_string()),
                        other => HostAnswer::Refused(format!("{other:?}")),
                    });
                }
            }
        });
        let nonces = datum::NonceGenerator::new();
        let first = nonces.generate();
        let hold = definition("record<done: bool>", " grmr channel send test/Held {x: 1} | ignore; sleep 2sec; {done: true} ");
        let held = tokio::spawn({
            let session = session.clone();
            async move { session.request(execute(first, hold)).await }
        });
        tokio::time::timeout(Duration::from_secs(10), asked_rx.recv())
            .await
            .expect("the first call runs within 10 s")
            .expect("its ask");
        let refused = session.request(execute(nonces.generate(), sleep())).await.expect("a response");
        assert!(matches!(refused, EngineResponse::Refused(TransportError::Busy)), "{refused:?}");
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
        let big = definition("record<s: string>", " {s: ('' | fill --character 'x' --width 200000)} ");
        let answer = session.request(execute(datum::NonceGenerator::new().generate(), big)).await.expect("a response");
        assert!(
            matches!(answer, EngineResponse::Refused(TransportError::ResultTooLarge { limit: 65536 })),
            "{answer:?}",
        );
        let info = value(&session, interface("info")).await;
        assert_eq!(text(field(&info, "name")), "grammar", "the session goes on");
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
        let send = definition("record<event_id: string>", " grmr channel send test/Model {x: 1} ");
        assert_eq!(value(&session, execute(nonces.generate(), send)).await, record(&[("event_id", NuVal::String("sent1".to_string()))]));
        let config = definition(
            "record<cpu: float, rate: int>",
            " let c = (grmr config get [supervisor.cpu_warn_fraction channel.spam_warn_rate]); {cpu: ($c | get 0.value), rate: ($c | get 1.value)} ",
        );
        assert_eq!(
            value(&session, execute(nonces.generate(), config)).await,
            record(&[("cpu", NuVal::Float(0.5)), ("rate", NuVal::Int(10))]),
            "the MCP's config, grammar-ssh's own supervisor lines in it",
        );
    });
}

/// The PREPARED record of a suite whose prepare answers nothing.
const NOTHING_PREPARED: &str = "record<job: record<id: int, started: datetime>, prepared: nothing>";

/// A suite on `data: nothing` whose job naps past any test's wait unless stopped.
fn slow_suite() -> String {
    format!(
        "export def prepare [data: nothing]: nothing -> nothing {{ }}
export def execute [data: {NOTHING_PREPARED}]: nothing -> record<done: bool> {{ sleep 30sec; {{done: true}} }}
export def status [data: {NOTHING_PREPARED}]: nothing -> nothing {{ }}
export def stop [data: {NOTHING_PREPARED}]: nothing -> nothing {{ }}
"
    )
}

/// A suite on `data: nothing` whose job answers `{done: true}` at once.
fn quick_suite() -> String {
    format!(
        "export def prepare [data: nothing]: nothing -> nothing {{ }}
export def execute [data: {NOTHING_PREPARED}]: nothing -> record<done: bool> {{ {{done: true}} }}
export def status [data: {NOTHING_PREPARED}]: nothing -> nothing {{ }}
export def stop [data: {NOTHING_PREPARED}]: nothing -> nothing {{ }}
"
    )
}

#[test]
fn an_await_job_answers_its_record_and_its_events_ride_the_ask() {
    let instance = TestInstance::new("fixture");
    runtime().block_on(async {
        let (_child, opened) = open(&instance, "default", LIMITS_DEFAULT).await;
        let (session, mut events) = opened.expect("the session opens");
        let (seen_tx, mut seen_rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            while let Some(event) = events.recv().await {
                if let SessionEvent::Ask { ask, responder } = event {
                    responder.answer(match ask {
                        SessionAsk::Await(event) => {
                            let _ = seen_tx.send(event);
                            HostAnswer::Sent("event1".to_string())
                        }
                        other => HostAnswer::Refused(format!("{other:?}")),
                    });
                }
            }
        });
        let nonce = datum::NonceGenerator::new().generate();
        let request = EngineRequest::Await(AwaitRequest {
            host: Host::Local,
            nonce,
            module: quick_suite(),
            data: NuVal::Nothing,
            timeouts: None,
            heartbeats: None,
        });
        let prepared = value(&session, request).await;
        assert_eq!(field(&prepared, "prepared"), &NuVal::Nothing);
        let event = tokio::time::timeout(Duration::from_secs(10), seen_rx.recv())
            .await
            .expect("an event within 10 s")
            .expect("the event");
        assert_eq!(event.model, AWAIT_RESULT_MODEL, "the Result first, no Start");
        let expected = record(&[
            ("await", NuVal::String(nonce.to_string())),
            ("result", record(&[("done", NuVal::Bool(true))])),
        ]);
        assert_eq!(event.data, expected, "the call's nonce as `await`, then execute's value");
    });
}

#[test]
fn a_second_grammar_ssh_on_a_namespace_is_refused() {
    let instance = TestInstance::new("fixture");
    runtime().block_on(async {
        let (_first, opened) = open(&instance, "default", LIMITS_DEFAULT).await;
        let (_session, _events) = opened.expect("the first session opens");
        let (mut second, refused) = open(&instance, "default", LIMITS_DEFAULT).await;
        let refusal = refused.expect_err("the second is refused");
        assert_eq!(refusal.kind(), "remote::engine::unavailable", "{refusal:?}");
        assert!(refusal.message().contains("namespace::held"), "{}", refusal.message());
        let status = exit_within(&mut second, EXIT_DEADLINE).await;
        assert_eq!(status.code(), Some(75), "{status:?}");
        assert_eq!(TestInstance::scratch_dirs(&instance.home("tmp")).len(), 1, "the refused one made no scratch");
    });
}

#[test]
fn the_end_of_stdin_ends_the_session_and_removes_its_scratch_and_cache() {
    let instance = TestInstance::new("fixture");
    let cache_home = instance.load("default").cache_home();
    runtime().block_on(async {
        let mut child = spawn_ssh(&instance, "default");
        let (mut raw, reply) = RawSession::open(&mut child, hello(&instance, "default", LIMITS_DEFAULT)).await;
        assert!(matches!(reply, HelloReply::Accepted(_)), "{reply:?}");
        let nonce = datum::NonceGenerator::new().generate();
        raw.send(&McpFrame::Request { id: 1, request: execute(nonce, sleep()) }).await;
        let call = cache_home.join("execute").join(nonce.to_string());
        for _ in 0..100 {
            if call.is_dir() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(call.is_dir(), "the call's cache entry, made as it runs");
        drop(raw.sink);
        let status = exit_within(&mut child, EXIT_DEADLINE).await;
        let stderr = stderr_of(&mut child).await;
        assert!(status.success(), "{status:?}: {stderr}");
        assert!(stderr.contains("the MCP's stream ended"), "{stderr}");
        assert!(!cache_home.exists(), "its cache home removed at its end, its logs among it");
        assert!(TestInstance::scratch_dirs(&instance.home("tmp")).is_empty(), "its scratch removed");
        NamespaceLock::acquire(&instance.load("default").lock_file()).expect("its lock released");
    });
}

#[test]
fn a_session_with_no_frame_in_its_window_ends() {
    let instance = TestInstance::new("fixture");
    runtime().block_on(async {
        let mut child = spawn_ssh(&instance, "default");
        let (mut raw, reply) = RawSession::open(&mut child, hello(&instance, "default", LIMITS_DEFAULT)).await;
        assert!(matches!(reply, HelloReply::Accepted(_)), "{reply:?}");
        let frame = tokio::time::timeout(Duration::from_secs(45), raw.next()).await.expect("a frame in the window's time");
        assert!(
            matches!(frame, Some(SshFrame::Close(SessionClose { error: Some(TransportError::SessionLost { .. }) }))),
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
        let mut child = spawn_ssh(&instance, "default");
        let (_raw, reply) = RawSession::open(&mut child, hello(&instance, "other", LIMITS_DEFAULT)).await;
        assert!(matches!(reply, HelloReply::Refused(TransportError::WrongNamespace { .. })), "{reply:?}");
        let status = exit_within(&mut child, EXIT_DEADLINE).await;
        assert_eq!(status.code(), Some(76), "{status:?}");
        assert!(TestInstance::scratch_dirs(&instance.home("tmp")).is_empty(), "its scratch removed");
    });
}
