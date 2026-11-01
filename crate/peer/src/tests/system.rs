//! The peer links' requests through their handle, on a test's own remotes file.
use crate::*;

/// What the links sent their controller unasked, as the controller heard it.
struct Heard {
    ups: tk::UnboundedSender<FromPeerSys>,
}

impl subsys::Handler<PeerSystem> for Heard {
    fn on_packet(&mut self, pkt: subsys::Packet<FromPeerSys>) -> impl Future<Output = Option<subsys::Packet<FromPeerSys>>> + 'static + Send {
        let _ = self.ups.send(pkt.take_msg());
        std::future::ready(None)
    }
}

/// A test's own directory, empty, its remotes file inside.
fn home(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("grammar_peer_{test}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a test home");
    dir
}

/// The links' system on `home`'s remotes file, with or without an identity.
async fn start(
    home: &Path,
    identity: Option<IdentityKeys>,
) -> PeerHandle {
    let (ups, _heard) = tk::unbounded_channel();
    let params = PeerSysParams {
        mcp_nom: "selfnom0001".to_string(),
        identity,
        config_file: ".grammar/config.toml".to_string(),
        remotes_file: home.join(REMOTES_FILE).to_string_lossy().into_owned(),
        expand: Vec::new(),
    };
    PeerHandle::start(PeerSysPaths::new(home.to_path_buf(), home.to_path_buf()), params, Heard { ups })
        .await
        .expect("the peer links' system starts")
}

fn set(alias: &str, cert: Option<&str>, address: Option<&str>) -> RemoteSetRequest {
    RemoteSetRequest {
        alias: alias.to_string(),
        address: address.map(str::to_string),
        listen: None,
        cert: cert.map(str::to_string),
    }
}

/// The reason a refusal names, or a panic for an answer that isn't one.
fn refusal(answer: Result<RemoteResult, RuntimeError>) -> RuntimeError {
    match answer {
        Err(e) => e,
        Ok(result) => panic!("a refusal, not {result:?}"),
    }
}

#[tokio::test]
async fn an_entry_is_set_read_and_unset_through_the_file() {
    let home = home("entry");
    let peers = start(&home, None).await;
    let rows = peers.request(RemoteListRequest).await.expect("list");
    assert_eq!(rows, Ok(RemoteResult::Rows(Vec::new())), "no file, no remotes");
    let made = peers
        .request(set("bob", Some("/k/bob.pem"), Some("127.0.0.1:9001")))
        .await
        .expect("set");
    let Ok(RemoteResult::Row(row)) = made else {
        panic!("a set answers its row: {made:?}");
    };
    assert_eq!(row.alias, "bob");
    assert_eq!(row.state, RemoteLinkState::Closed);
    assert_eq!(row.address.as_deref(), Some("127.0.0.1:9001"));
    let text = fs::read_to_string(home.join(REMOTES_FILE)).expect("the file written");
    assert!(text.contains("alias = \"bob\""), "{text}");
    let got = peers
        .request(RemoteGetRequest {
            alias: "bob".to_string(),
        })
        .await
        .expect("get");
    assert_eq!(got, Ok(RemoteResult::Row(row)));
    let unset = peers
        .request(RemoteUnsetRequest {
            alias: "bob".to_string(),
        })
        .await
        .expect("unset");
    assert_eq!(unset, Ok(RemoteResult::Done));
    let missing = peers
        .request(RemoteGetRequest {
            alias: "bob".to_string(),
        })
        .await
        .expect("get");
    assert!(matches!(refusal(missing), RuntimeError::RemoteInvalidParams { .. }));
    peers.stop().await.expect("stop");
    let _ = fs::remove_dir_all(&home);
}

#[tokio::test]
async fn a_set_refuses_what_the_file_would_not_hold() {
    let home = home("refused");
    let peers = start(&home, None).await;
    for req in [
        set("", Some("/k/a.pem"), None),
        set("bob", Some("/k/bob.pem"), Some("nowhere")),
        set("bob", Some(""), None),
        set("bob", None, Some("127.0.0.1:9001")),
    ] {
        let answer = peers.request(req.clone()).await.expect("set");
        assert!(
            matches!(refusal(answer), RuntimeError::RemoteInvalidParams { .. }),
            "{req:?} is refused",
        );
    }
    assert!(!home.join(REMOTES_FILE).exists(), "a refused set writes nothing");
    peers.stop().await.expect("stop");
    let _ = fs::remove_dir_all(&home);
}

#[tokio::test]
async fn an_open_names_what_it_lacks() {
    let home = home("open");
    let peers = start(&home, None).await;
    let unknown = peers
        .request(RemoteListenRequest {
            alias: "nobody".to_string(),
            on: None,
        })
        .await
        .expect("listen");
    let RuntimeError::RemoteInvalidParams { reason } = refusal(unknown) else {
        panic!("an unknown alias is refused as a parameter");
    };
    assert!(reason.contains("interface remote channel set"), "{reason}");
    let _ = peers.request(set("bob", Some("/k/bob.pem"), None)).await.expect("set");
    let keyless = peers
        .request(RemoteListenRequest {
            alias: "bob".to_string(),
            on: None,
        })
        .await
        .expect("listen");
    let RuntimeError::RemoteInvalidParams { reason } = refusal(keyless) else {
        panic!("a host with no identity has no key to present");
    };
    assert!(reason.contains(".grammar/config.toml"), "the config file is named: {reason}");
    let addressless = peers
        .request(RemoteConnectRequest {
            alias: "bob".to_string(),
            address: None,
        })
        .await
        .expect("connect");
    let RuntimeError::RemoteInvalidParams { reason } = refusal(addressless) else {
        panic!("a connect needs an address");
    };
    assert!(reason.contains("--address"), "{reason}");
    peers.stop().await.expect("stop");
    let _ = fs::remove_dir_all(&home);
}

#[tokio::test]
async fn a_listen_whose_keys_fail_to_load_fails_its_open() {
    let home = home("keys");
    let identity = IdentityKeys {
        cert: home.join("self.pem").to_string_lossy().into_owned(),
        key: home.join("self.key").to_string_lossy().into_owned(),
    };
    let peers = start(&home, Some(identity)).await;
    let _ = peers.request(set("bob", Some("/k/bob.pem"), None)).await.expect("set");
    let failed = peers
        .request(RemoteListenRequest {
            alias: "bob".to_string(),
            on: Some("127.0.0.1".to_string()),
        })
        .await
        .expect("listen");
    let RuntimeError::RemoteOpenFailed { alias, reason } = refusal(failed) else {
        panic!("a listen that can't load its keys fails its open");
    };
    assert_eq!(alias, "bob");
    assert!(reason.contains("load public key"), "{reason}");
    let rows = peers.request(RemoteListRequest).await.expect("list");
    let Ok(RemoteResult::Rows(rows)) = rows else {
        panic!("a list answers its rows");
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].state, RemoteLinkState::Closed, "a failed open leaves nothing live");
    peers.stop().await.expect("stop");
    let _ = fs::remove_dir_all(&home);
}

#[tokio::test]
async fn a_close_or_a_send_with_no_link_is_refused() {
    let home = home("unlinked");
    let peers = start(&home, None).await;
    let closed = peers
        .request(RemoteCloseRequest {
            alias: "bob".to_string(),
        })
        .await
        .expect("close");
    assert_eq!(
        closed,
        Err(RuntimeError::RemoteNotOpen {
            alias: "bob".to_string()
        })
    );
    let send = |model: &str| RemoteSendRequest {
        mcp_nom: "peernom0001".to_string(),
        model: model.to_string(),
        event_nuon: "{a: 1}".to_string(),
        payloads: Vec::new(),
    };
    let unlinked = peers.request(send("proj/thing/Changed")).await.expect("send");
    assert_eq!(unlinked, Err("no open remote link to mcp_nom `peernom0001`".to_string()));
    let reserved = peers.request(send("remote/channel/Sent")).await.expect("send");
    assert!(
        reserved.as_ref().is_err_and(|e| e.contains("reserved")),
        "a body never stamps a host's model: {reserved:?}",
    );
    let inbox = Some(home.to_string_lossy().into_owned());
    let held = peers.request(InboxRequest(inbox.clone())).await.expect("inbox");
    assert_eq!(held, inbox, "the links hold the inbox they were told");
    peers.stop().await.expect("stop");
    let _ = fs::remove_dir_all(&home);
}
