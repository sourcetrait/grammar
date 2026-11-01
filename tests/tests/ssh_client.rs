//! The client's SSH sessions against OpenSSH's sshd on loopback: trust, calls, kills, and ends.
#![cfg(unix)]

use std::sync::Arc;
use std::time::Duration;

use sourcetrait_common::{datum, subsys};
use sourcetrait_grammar_client::{
    FromRemote, RemoteHandle, RemoteSession, RemoteSysParams, RemoteSysPaths, RemoteSystem, open,
};
use sourcetrait_grammar_protocol::{
    ControlAnswer, ControlRequest, EngineCommand, EngineRequest, EngineResponse, ExecuteRequest,
    FromEngineSys, Host, LIMITS_DEFAULT, NuVal, ToEngineSys, TransportError, ValueAnswer,
};
use sourcetrait_grammar_tests::{Cuttable, Sshd, TestKey, TestWorker, alive, holds_within};

/// A definition that sleeps past any test's wait, until it is cancelled.
const SLEEP: &str = "def execute [data: nothing]: nothing -> nothing { sleep 30sec }";

/// How long a worker may take to end once its session does.
const END_WAIT: Duration = Duration::from_secs(30);

/// A runtime for a test's async half.
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("a test runtime")
}

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

/// A control command as a request.
fn control(command: EngineCommand) -> EngineRequest {
    EngineRequest::Wire(ToEngineSys::Control(ControlRequest { host: Host::Local, command }))
}

/// The session's refusal, its success a panic.
fn refusal(opened: Result<(RemoteSession, impl Sized), TransportError>) -> TransportError {
    match opened {
        Ok(_) => panic!("the session opened"),
        Err(refusal) => refusal,
    }
}

/// The first call the worker lists in flight, waited for.
async fn first_in_flight(session: &RemoteSession) -> String {
    for _ in 0..100 {
        if let Ok(EngineResponse::Wire(FromEngineSys::Control(Ok(ControlAnswer::Processes(rows))))) =
            session.request(control(EngineCommand::Processes)).await
            && let Some(row) = rows.first()
        {
            return row.nonce.clone();
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("no call in flight in 10 s");
}

#[test]
fn a_host_key_not_pinned_is_refused_before_authentication() {
    let sshd = Sshd::start();
    let worker = TestWorker::new();
    let stray = TestKey::new(&worker.root().join("stray_ed25519"));
    let mut endpoint = sshd.endpoint("lab", &worker);
    endpoint.host_keys = vec![stray.public.clone()];
    let refused = runtime().block_on(async { refusal(open(&endpoint, LIMITS_DEFAULT).await) });
    assert_eq!(refused.kind(), "daemon::host_key_rejected", "{}", refused.message());
    assert!(!sshd.log().contains("Accepted publickey"), "nothing authenticated: {}", sshd.log());
}

#[test]
fn a_key_the_host_does_not_authorize_is_refused() {
    let sshd = Sshd::start();
    let worker = TestWorker::new();
    let mut endpoint = sshd.endpoint("lab", &worker);
    endpoint.key = sshd.stranger_key.path();
    let refused = runtime().block_on(async { refusal(open(&endpoint, LIMITS_DEFAULT).await) });
    assert_eq!(refused.kind(), "daemon::authentication_failed", "{}", refused.message());
    assert!(worker.scratch_dirs().is_empty(), "no worker started");
}

#[test]
fn a_session_serves_its_workers_info_a_call_and_a_kill() {
    let sshd = Sshd::start();
    let worker = TestWorker::new();
    let endpoint = sshd.endpoint("lab", &worker);
    runtime().block_on(async {
        let (session, _events) = open(&endpoint, LIMITS_DEFAULT).await.expect("a session");
        let session = Arc::new(session);
        match session.request(EngineRequest::Info).await.expect("info") {
            EngineResponse::Info(info) => {
                assert_eq!((info.id.as_str(), info.namespace.as_str()), ("fixture", "default"));
                assert_eq!(info.work_dir, worker.workdir().display().to_string());
                assert_eq!(info.nom, session.worker().nom);
            }
            other => panic!("info, not {other:?}"),
        }
        let nonces = datum::NonceGenerator::new();
        let definition = "def execute [data: nothing]: nothing -> record<n: int> { {n: 42} }";
        match session.request(execute(nonces.generate(), definition)).await.expect("a response") {
            EngineResponse::Wire(FromEngineSys::Value(ValueAnswer { result: Ok(value), .. })) => {
                assert_eq!(value, NuVal::Record(vec![("n".to_string(), NuVal::Int(42))].into_iter().collect()));
            }
            other => panic!("a value, not {other:?}"),
        }
        let nonce = nonces.generate();
        let call = tokio::spawn({
            let session = session.clone();
            async move { session.request(execute(nonce, SLEEP)).await }
        });
        let listed = first_in_flight(&session).await;
        assert_eq!(listed, nonce.to_string(), "the worker lists the call by its nonce");
        session.request(control(EngineCommand::Kill { nonce: listed })).await.expect("the kill");
        let answer = tokio::time::timeout(Duration::from_secs(10), call).await.expect("the call ends").unwrap();
        assert!(
            matches!(answer, Ok(EngineResponse::Wire(FromEngineSys::Value(ValueAnswer { result: Err(_), .. })))),
            "the killed call answers its failure: {answer:?}",
        );
        let pid = session.worker().pid;
        session.close().await;
        assert!(holds_within(END_WAIT, || !alive(pid)).await, "the worker ends with its session");
    });
}

#[test]
fn a_dropped_connection_ends_its_worker_and_frees_its_namespace() {
    let sshd = Sshd::start();
    let worker = TestWorker::new();
    runtime().block_on(async {
        let proxy = Cuttable::start(sshd.port).await;
        let mut endpoint = sshd.endpoint("lab", &worker);
        endpoint.port = proxy.port;
        let (session, _events) = open(&endpoint, LIMITS_DEFAULT).await.expect("a session");
        let pid = session.worker().pid;
        assert!(alive(pid));
        assert_eq!(worker.scratch_dirs().len(), 1, "the worker's scratch");
        proxy.cut();
        assert!(holds_within(END_WAIT, || !alive(pid)).await, "the worker ends with its connection");
        assert!(worker.scratch_dirs().is_empty(), "its scratch removed");
        assert!(session.has_ended(), "the session ended with its connection");
        let endpoint = sshd.endpoint("lab", &worker);
        let (again, _events) = open(&endpoint, LIMITS_DEFAULT).await.expect("the namespace freed");
        assert_ne!(again.worker().pid, pid, "a new worker");
        again.close().await;
    });
}

/// A controller that holds no ask, for the system alone.
struct Bare;

impl subsys::Handler<RemoteSystem> for Bare {
    fn on_packet(
        &mut self,
        _pkt: subsys::Packet<FromRemote>,
    ) -> impl Future<Output = Option<subsys::Packet<FromRemote>>> + 'static + Send {
        std::future::ready(None)
    }
}

/// Whether the worker lists a call in flight within 10 s, through the system.
async fn in_flight(remote: &RemoteHandle) -> bool {
    for _ in 0..100 {
        if let Ok(EngineResponse::Wire(FromEngineSys::Control(Ok(ControlAnswer::Processes(rows))))) =
            remote.request("lab", control(EngineCommand::Processes)).await
            && !rows.is_empty()
        {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

/// The worker's pid as an eval on it reads it, through the system.
async fn worker_pid(remote: &RemoteHandle) -> u32 {
    let definition = "def execute [data: nothing]: nothing -> record<pid: int> { {pid: $nu.pid} }";
    let request = execute(datum::NonceGenerator::new().generate(), definition);
    match remote.request("lab", request).await.expect("a response") {
        EngineResponse::Wire(FromEngineSys::Value(ValueAnswer { result: Ok(value), .. })) => {
            match value.get("pid") {
                Some(NuVal::Int(pid)) => *pid as u32,
                other => panic!("a pid, not {other:?}"),
            }
        }
        other => panic!("a value, not {other:?}"),
    }
}

#[test]
fn a_lost_worker_answers_its_calls_lost_and_the_next_request_opens_a_session() {
    let sshd = Sshd::start();
    let worker = TestWorker::new();
    let endpoint = sshd.endpoint("lab", &worker);
    runtime().block_on(async {
        let paths = RemoteSysPaths::new(worker.home("data"), worker.home("cache"));
        let params = RemoteSysParams {
            endpoints: vec![endpoint],
            limits: LIMITS_DEFAULT,
        };
        let remote = Arc::new(RemoteHandle::start(paths, params, Bare).await.expect("the system"));
        let pid = worker_pid(&remote).await;
        assert_eq!(worker_pid(&remote).await, pid, "one session serves both");
        let call = tokio::spawn({
            let remote = remote.clone();
            async move { remote.request("lab", execute(datum::NonceGenerator::new().generate(), SLEEP)).await }
        });
        assert!(in_flight(&remote).await, "the call runs on the worker");
        nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), nix::sys::signal::Signal::SIGKILL)
            .expect("the worker killed");
        let answer = tokio::time::timeout(END_WAIT, call).await.expect("the call ends").unwrap();
        match answer {
            Err(lost) => assert_eq!(lost.kind(), "daemon::session_lost", "{}", lost.message()),
            Ok(other) => panic!("the call's loss, not {other:?}"),
        }
        let next = worker_pid(&remote).await;
        assert_ne!(next, pid, "a new session, a new worker");
        remote.close().await;
        assert!(holds_within(END_WAIT, || !alive(next)).await, "the close ends the worker");
        RemoteHandle::stop(remote).await.expect("the system stops");
    });
}
