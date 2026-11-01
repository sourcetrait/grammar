//! The client's SSH sessions against OpenSSH's sshd on loopback: trust, calls, kills, and ends.
#![cfg(unix)]

use std::sync::Arc;
use std::time::Duration;

use sourcetrait_common::{datum, subsys};
use sourcetrait_grammar_client::{
    Endpoint, FromRemote, RemoteHandle, RemoteSession, RemoteSysParams, RemoteSysPaths, RemoteSystem,
    SessionPhase, open,
};
use sourcetrait_grammar_protocol::{
    ControlAnswer, ControlRequest, EngineCommand, EngineRequest, EngineResponse, ExecuteRequest,
    FromEngineSys, Host, LIMITS_DEFAULT, NuVal, ToEngineSys, TransportError, ValueAnswer,
};
use sourcetrait_grammar_tests::{Cuttable, LOGIN_MARK, Sshd, TestKey, TestWorker, alive, holds_within};

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
    assert_eq!(refused.kind(), "remote::engine::host_key_rejected", "{}", refused.message());
    assert!(!sshd.log().contains("Accepted publickey"), "nothing authenticated: {}", sshd.log());
}

#[test]
fn a_key_the_host_does_not_authorize_is_refused() {
    let sshd = Sshd::start();
    let worker = TestWorker::new();
    let mut endpoint = sshd.endpoint("lab", &worker);
    endpoint.key = sshd.stranger_key.path();
    let refused = runtime().block_on(async { refusal(open(&endpoint, LIMITS_DEFAULT).await) });
    assert_eq!(refused.kind(), "remote::engine::authentication_failed", "{}", refused.message());
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
fn a_worker_runs_in_its_accounts_login_and_starts_where_the_session_does() {
    let sshd = Sshd::start();
    let worker = TestWorker::new();
    let mut endpoint = sshd.endpoint("lab", &worker);
    endpoint.workdir = None;
    runtime().block_on(async {
        let (session, _events) = open(&endpoint, LIMITS_DEFAULT).await.expect("a session");
        let definition = format!(
            "def execute [data: nothing]: nothing -> record<login: string> {{ {{login: ($env.{LOGIN_MARK}? | default '')}} }}"
        );
        match session.request(execute(datum::NonceGenerator::new().generate(), &definition)).await.expect("a response") {
            EngineResponse::Wire(FromEngineSys::Value(ValueAnswer { result: Ok(value), .. })) => {
                assert_eq!(value.get("login"), Some(&NuVal::String("1".to_string())), "the login shell's mark");
            }
            other => panic!("a value, not {other:?}"),
        }
        let home = nix::unistd::User::from_uid(nix::unistd::getuid())
            .expect("the user database")
            .expect("the test's user")
            .dir;
        match session.request(EngineRequest::Info).await.expect("info") {
            EngineResponse::Info(info) => {
                let start = std::fs::canonicalize(&info.work_dir).expect("the workdir");
                assert_eq!(start, std::fs::canonicalize(&home).expect("the home"), "the session's start");
            }
            other => panic!("info, not {other:?}"),
        }
        session.close().await;
    });
}

#[test]
fn a_worker_missing_from_its_host_is_command_not_found() {
    let sshd = Sshd::start();
    let worker = TestWorker::new();
    let mut endpoint = sshd.endpoint("lab", &worker);
    endpoint.command = worker.home("bin").join("absent").display().to_string();
    let refused = runtime().block_on(async { refusal(open(&endpoint, LIMITS_DEFAULT).await) });
    assert_eq!(refused.kind(), "remote::engine::unavailable", "{}", refused.message());
    assert!(
        refused.message().ends_with(&format!("command not found: {}", endpoint.command)),
        "{}",
        refused.message()
    );
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
async fn in_flight(remote: &RemoteHandle, endpoint: &Endpoint) -> bool {
    for _ in 0..100 {
        if let Ok(EngineResponse::Wire(FromEngineSys::Control(Ok(ControlAnswer::Processes(rows))))) =
            remote.request(endpoint.clone(), control(EngineCommand::Processes)).await
            && !rows.is_empty()
        {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

/// The worker's pid as an eval on it reads it, through the system.
async fn worker_pid(remote: &RemoteHandle, endpoint: &Endpoint) -> u32 {
    let definition = "def execute [data: nothing]: nothing -> record<pid: int> { {pid: $nu.pid} }";
    let request = execute(datum::NonceGenerator::new().generate(), definition);
    match remote.request(endpoint.clone(), request).await.expect("a response") {
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
        let params = RemoteSysParams { limits: LIMITS_DEFAULT };
        let remote = Arc::new(RemoteHandle::start(paths, params, Bare).await.expect("the system"));
        let pid = worker_pid(&remote, &endpoint).await;
        assert_eq!(worker_pid(&remote, &endpoint).await, pid, "one session serves both");
        let call = tokio::spawn({
            let remote = remote.clone();
            let endpoint = endpoint.clone();
            async move { remote.request(endpoint, execute(datum::NonceGenerator::new().generate(), SLEEP)).await }
        });
        assert!(in_flight(&remote, &endpoint).await, "the call runs on the worker");
        nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), nix::sys::signal::Signal::SIGKILL)
            .expect("the worker killed");
        let answer = tokio::time::timeout(END_WAIT, call).await.expect("the call ends").unwrap();
        match answer {
            Err(lost) => assert_eq!(lost.kind(), "remote::engine::session_lost", "{}", lost.message()),
            Ok(other) => panic!("the call's loss, not {other:?}"),
        }
        let next = worker_pid(&remote, &endpoint).await;
        assert_ne!(next, pid, "a new session, a new worker");
        remote.close().await;
        assert!(holds_within(END_WAIT, || !alive(next)).await, "the close ends the worker");
        RemoteHandle::stop(remote).await.expect("the system stops");
    });
}

#[test]
fn a_session_opens_on_its_entry_now_reports_its_state_and_closes_alone() {
    let sshd = Sshd::start();
    let worker = TestWorker::new();
    let endpoint = sshd.endpoint("lab", &worker);
    let mut stranger = sshd.endpoint("gate", &worker);
    stranger.key = sshd.stranger_key.path();
    runtime().block_on(async {
        let paths = RemoteSysPaths::new(worker.home("data"), worker.home("cache"));
        let remote = Arc::new(
            RemoteHandle::start(paths, RemoteSysParams { limits: LIMITS_DEFAULT }, Bare).await.expect("the system"),
        );
        assert!(remote.states().await.is_empty(), "no session before a request or a connect");
        let state = remote.connect(endpoint.clone()).await.expect("the session opens now");
        assert_eq!(state.phase, SessionPhase::Open, "{state:?}");
        let pid = state.worker_pid.expect("its worker's pid");
        assert_eq!(worker_pid(&remote, &endpoint).await, pid, "a request takes the open session");
        let refused = remote.connect(stranger).await.expect_err("an unauthorized key");
        assert_eq!(refused.kind(), "remote::engine::authentication_failed", "{}", refused.message());
        let states = remote.states().await;
        let failed = states.iter().find(|state| state.alias == "gate").expect("the failed open's alias");
        assert_eq!(failed.phase, SessionPhase::Closed, "{failed:?}");
        assert!(failed.failure.as_deref().is_some_and(|f| f.starts_with("remote::engine::authentication_failed")), "{failed:?}");
        assert!(remote.close_session("lab").await, "an open session closes");
        assert!(holds_within(END_WAIT, || !alive(pid)).await, "its worker ends with it");
        assert!(!remote.close_session("lab").await, "no session is left to close");
        assert!(remote.states().await.iter().all(|state| state.phase == SessionPhase::Closed), "nothing open");
        RemoteHandle::stop(remote).await.expect("the system stops");
    });
}
