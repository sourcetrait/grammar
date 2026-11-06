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
    CallAnswer, EngineRequest, EngineResponse, ExecuteDefinition, ExecuteRequest, Host, InterfaceRequest,
    LIMITS_DEFAULT, NuType, NuVal, TransportError,
};
use sourcetrait_grammar_tests::{Cuttable, LOGIN_MARK, Sshd, TestAccount, TestKey, alive, holds_within};

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

/// How long grammar-ssh may take to end once its session does.
const END_WAIT: Duration = Duration::from_secs(30);

/// A runtime for a test's async half.
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("a test runtime")
}

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

/// A call's value, anything else a panic.
fn value(response: Result<EngineResponse, TransportError>) -> NuVal {
    match response {
        Ok(EngineResponse::Answer(CallAnswer { result: Ok(value), .. })) => value,
        other => panic!("a value, not {other:?}"),
    }
}

/// A string field's text, anything else a panic.
fn text<'a>(value: &'a NuVal, name: &str) -> &'a str {
    match value.get(name) {
        Some(NuVal::String(text)) => text,
        other => panic!("a string `{name}`, not {other:?} in {value:?}"),
    }
}

/// The nonces of the calls a process list's value shows in flight.
fn listed(processes: &NuVal) -> Vec<String> {
    match processes.get("processes") {
        Some(NuVal::List(rows)) => rows.iter().map(|row| text(row, "nonce").to_string()).collect(),
        other => panic!("a process list, not {other:?}"),
    }
}

/// The session's refusal, its success a panic.
fn refusal(opened: Result<(RemoteSession, impl Sized), TransportError>) -> TransportError {
    match opened {
        Ok(_) => panic!("the session opened"),
        Err(refusal) => refusal,
    }
}

/// The first call grammar-ssh lists in flight, waited for.
async fn first_in_flight(session: &RemoteSession) -> String {
    for _ in 0..100 {
        if let Some(nonce) = listed(&value(session.request(interface("process list")).await)).first() {
            return nonce.clone();
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("no call in flight in 10 s");
}

#[test]
fn a_host_key_not_pinned_is_refused_before_authentication() {
    let sshd = Sshd::start();
    let account = TestAccount::new();
    let stray = TestKey::new(&account.root().join("stray_ed25519"));
    let mut endpoint = sshd.endpoint("lab", &account);
    endpoint.host_keys = vec![stray.public.clone()];
    let refused = runtime().block_on(async { refusal(open(&endpoint, LIMITS_DEFAULT).await) });
    assert_eq!(refused.kind(), "remote::engine::host_key_rejected", "{}", refused.message());
    assert!(!sshd.log().contains("Accepted publickey"), "nothing authenticated: {}", sshd.log());
}

#[test]
fn a_key_the_host_does_not_authorize_is_refused() {
    let sshd = Sshd::start();
    let account = TestAccount::new();
    let mut endpoint = sshd.endpoint("lab", &account);
    endpoint.key = sshd.stranger_key.path();
    let refused = runtime().block_on(async { refusal(open(&endpoint, LIMITS_DEFAULT).await) });
    assert_eq!(refused.kind(), "remote::engine::authentication_failed", "{}", refused.message());
    assert!(account.scratch_dirs().is_empty(), "no grammar-ssh started");
}

#[test]
fn a_session_serves_grammar_ssh_s_info_a_call_and_a_kill() {
    let sshd = Sshd::start();
    let account = TestAccount::new();
    let endpoint = sshd.endpoint("lab", &account);
    runtime().block_on(async {
        let (session, _events) = open(&endpoint, LIMITS_DEFAULT).await.expect("a session");
        let session = Arc::new(session);
        let info = value(session.request(interface("interface info")).await);
        assert_eq!((text(&info, "id"), text(&info, "namespace")), ("fixture", "default"));
        assert_eq!(text(&info, "work_dir"), account.workdir().display().to_string());
        assert_eq!(text(&info, "nom"), session.ssh().nom);
        let nonces = datum::NonceGenerator::new();
        let answer = value(session.request(execute(nonces.generate(), definition("record<n: int>", " {n: 42} "))).await);
        assert_eq!(answer, NuVal::Record(vec![("n".to_string(), NuVal::Int(42))]));
        let nonce = nonces.generate();
        let call = tokio::spawn({
            let session = session.clone();
            async move { session.request(execute(nonce, sleep())).await }
        });
        let listed = first_in_flight(&session).await;
        assert_eq!(listed, nonce.to_string(), "grammar-ssh lists the call by its nonce");
        value(session.request(interface(&format!("process kill '{listed}'"))).await);
        let answer = tokio::time::timeout(Duration::from_secs(10), call).await.expect("the call ends").unwrap();
        assert!(
            matches!(answer, Ok(EngineResponse::Answer(CallAnswer { result: Err(_), .. }))),
            "the killed call answers its failure: {answer:?}",
        );
        let pid = session.ssh().pid;
        session.close().await;
        assert!(holds_within(END_WAIT, || !alive(pid)).await, "grammar-ssh ends with its session");
    });
}

#[test]
fn grammar_ssh_runs_in_its_accounts_login_and_starts_where_the_session_does() {
    let sshd = Sshd::start();
    let account = TestAccount::new();
    let mut endpoint = sshd.endpoint("lab", &account);
    endpoint.workdir = None;
    runtime().block_on(async {
        let (session, _events) = open(&endpoint, LIMITS_DEFAULT).await.expect("a session");
        let body = format!(" {{login: ($env.{LOGIN_MARK}? | default '')}} ");
        let marked = value(
            session
                .request(execute(datum::NonceGenerator::new().generate(), definition("record<login: string>", &body)))
                .await,
        );
        assert_eq!(marked.get("login"), Some(&NuVal::String("1".to_string())), "the login shell's mark");
        let home = nix::unistd::User::from_uid(nix::unistd::getuid())
            .expect("the user database")
            .expect("the test's user")
            .dir;
        let info = value(session.request(interface("info")).await);
        let start = std::fs::canonicalize(text(&info, "work_dir")).expect("the workdir");
        assert_eq!(start, std::fs::canonicalize(&home).expect("the home"), "the session's start");
        session.close().await;
    });
}

#[test]
fn grammar_ssh_missing_from_its_host_is_command_not_found() {
    let sshd = Sshd::start();
    let account = TestAccount::new();
    let mut endpoint = sshd.endpoint("lab", &account);
    endpoint.command = account.home("bin").join("absent").display().to_string();
    let refused = runtime().block_on(async { refusal(open(&endpoint, LIMITS_DEFAULT).await) });
    assert_eq!(refused.kind(), "remote::engine::unavailable", "{}", refused.message());
    assert!(
        refused.message().ends_with(&format!("command not found: {}", endpoint.command)),
        "{}",
        refused.message()
    );
}

#[test]
fn a_dropped_connection_ends_its_grammar_ssh_and_frees_its_namespace() {
    let sshd = Sshd::start();
    let account = TestAccount::new();
    runtime().block_on(async {
        let proxy = Cuttable::start(sshd.port).await;
        let mut endpoint = sshd.endpoint("lab", &account);
        endpoint.port = proxy.port;
        let (session, _events) = open(&endpoint, LIMITS_DEFAULT).await.expect("a session");
        let pid = session.ssh().pid;
        assert!(alive(pid));
        assert_eq!(account.scratch_dirs().len(), 1, "grammar-ssh's scratch");
        proxy.cut();
        assert!(holds_within(END_WAIT, || !alive(pid)).await, "grammar-ssh ends with its connection");
        assert!(account.scratch_dirs().is_empty(), "its scratch removed");
        assert!(session.has_ended(), "the session ended with its connection");
        let endpoint = sshd.endpoint("lab", &account);
        let (again, _events) = open(&endpoint, LIMITS_DEFAULT).await.expect("the namespace freed");
        assert_ne!(again.ssh().pid, pid, "a new grammar-ssh");
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

/// Whether grammar-ssh lists a call in flight within 10 s, through the system.
async fn in_flight(remote: &RemoteHandle, endpoint: &Endpoint) -> bool {
    for _ in 0..100 {
        if !listed(&value(remote.request(endpoint.clone(), interface("process list")).await)).is_empty() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

/// grammar-ssh's pid as an eval on it reads it, through the system.
async fn ssh_pid(remote: &RemoteHandle, endpoint: &Endpoint) -> u32 {
    let request = execute(datum::NonceGenerator::new().generate(), definition("record<pid: int>", " {pid: $nu.pid} "));
    match value(remote.request(endpoint.clone(), request).await).get("pid") {
        Some(NuVal::Int(pid)) => *pid as u32,
        other => panic!("a pid, not {other:?}"),
    }
}

#[test]
fn a_lost_grammar_ssh_answers_its_calls_lost_and_the_next_request_its_restart() {
    let sshd = Sshd::start();
    let account = TestAccount::new();
    let endpoint = sshd.endpoint("lab", &account);
    runtime().block_on(async {
        let paths = RemoteSysPaths::new(account.home("data"), account.home("cache"));
        let params = RemoteSysParams { limits: LIMITS_DEFAULT };
        let remote = Arc::new(RemoteHandle::start(paths, params, Bare).await.expect("the system"));
        let pid = ssh_pid(&remote, &endpoint).await;
        assert_eq!(ssh_pid(&remote, &endpoint).await, pid, "one session serves both");
        let call = tokio::spawn({
            let remote = remote.clone();
            let endpoint = endpoint.clone();
            async move { remote.request(endpoint, execute(datum::NonceGenerator::new().generate(), sleep())).await }
        });
        assert!(in_flight(&remote, &endpoint).await, "the call runs on grammar-ssh");
        nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), nix::sys::signal::Signal::SIGKILL)
            .expect("grammar-ssh killed");
        let answer = tokio::time::timeout(END_WAIT, call).await.expect("the call ends").unwrap();
        match answer {
            Err(lost) => assert_eq!(lost.kind(), "remote::engine::session_lost", "{}", lost.message()),
            Ok(other) => panic!("the call's loss, not {other:?}"),
        }
        let told = remote.request(endpoint.clone(), interface("info")).await.expect_err("the restart, nothing run");
        let TransportError::EngineRestart { nom, failure } = &told else {
            panic!("the engine's restart, not {told:?}");
        };
        assert!(failure.as_deref().is_some_and(|f| f.ends_with("grammar-ssh's stream ended")), "{failure:?}");
        let state = remote.states().await.into_iter().find(|state| state.alias == "lab").expect("its state");
        assert_eq!((state.phase, state.nom.as_deref()), (SessionPhase::Open, Some(nom.as_str())), "the rows show its nom");
        let kinds: Vec<String> = told.rows().into_iter().map(|row| row.kind).collect();
        assert_eq!(kinds, ["engine::fail", "engine::restart"]);
        let next = ssh_pid(&remote, &endpoint).await;
        assert_ne!(next, pid, "the call after the restart runs, on a new grammar-ssh");
        assert!(remote.close_session("lab").await, "the session closes");
        assert!(holds_within(END_WAIT, || !alive(next)).await, "the close ends grammar-ssh");
        let reopened = ssh_pid(&remote, &endpoint).await;
        assert_ne!(reopened, next, "a call after a close opens a session with no restart told");
        remote.close().await;
        assert!(holds_within(END_WAIT, || !alive(reopened)).await, "the close ends grammar-ssh");
        RemoteHandle::stop(remote).await.expect("the system stops");
    });
}

#[test]
fn a_session_opens_on_its_entry_now_reports_its_state_and_closes_alone() {
    let sshd = Sshd::start();
    let account = TestAccount::new();
    let endpoint = sshd.endpoint("lab", &account);
    let mut stranger = sshd.endpoint("gate", &account);
    stranger.key = sshd.stranger_key.path();
    runtime().block_on(async {
        let paths = RemoteSysPaths::new(account.home("data"), account.home("cache"));
        let remote = Arc::new(
            RemoteHandle::start(paths, RemoteSysParams { limits: LIMITS_DEFAULT }, Bare).await.expect("the system"),
        );
        assert!(remote.states().await.is_empty(), "no session before a request or a connect");
        let state = remote.connect(endpoint.clone()).await.expect("the session opens now");
        assert_eq!(state.phase, SessionPhase::Open, "{state:?}");
        let pid = state.pid.expect("its grammar-ssh's pid");
        assert_eq!(ssh_pid(&remote, &endpoint).await, pid, "a request takes the open session");
        let refused = remote.connect(stranger).await.expect_err("an unauthorized key");
        assert_eq!(refused.kind(), "remote::engine::authentication_failed", "{}", refused.message());
        let states = remote.states().await;
        let failed = states.iter().find(|state| state.alias == "gate").expect("the failed open's alias");
        assert_eq!(failed.phase, SessionPhase::Closed, "{failed:?}");
        assert!(failed.failure.as_deref().is_some_and(|f| f.starts_with("remote::engine::authentication_failed")), "{failed:?}");
        assert!(remote.close_session("lab").await, "an open session closes");
        assert!(holds_within(END_WAIT, || !alive(pid)).await, "its grammar-ssh ends with it");
        assert!(!remote.close_session("lab").await, "no session is left to close");
        assert!(remote.states().await.iter().all(|state| state.phase == SessionPhase::Closed), "nothing open");
        RemoteHandle::stop(remote).await.expect("the system stops");
    });
}
