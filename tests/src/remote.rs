//! RemoteChannel's test harness: a simulated rmcp consumer, keys, and config files.
//!
//! The consumer is the existing stdio `Host` plus a real WSS Channel client that
//! trusts a one-off test CA; the keys are one-off, and the `.grammar/` files the
//! ones two linked hosts need.
//!
//! No test seam: the consumer connects to a host's Channel exactly as
//! claude-code's Monitor does (a real TLS + WebSocket client), so it settles the
//! observation question and is reusable to script any packet flow. Keys are
//! one-off and trusted directly via a private root store - never the OS trust
//! store, so the whole thing runs unattended with no OS-level install / sudo.
//!
//! Wire packets are NUON, read through nuval.

use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use srcert_lib as lib_cert;
use tokio_rustls::rustls::pki_types::CertificateDer;
use tokio_rustls::rustls::pki_types::pem::PemObject;

use crate::{Host, NuVal, envelope, has_error_path, try_nuon};

/// The key material a `generate` run leaves under `<dir>/certs`.
pub struct GeneratedKeys {
    /// The self-signed CA public key a WSS client trusts as its sole root.
    pub ca: PathBuf,
    /// A host's own public key, its `[identity]` certificate and a peer's known key.
    pub public_key: PathBuf,
    /// The matching private key.
    pub private_key: PathBuf,
}

fn key_config(name: &str) -> lib_cert::config::CertGenConfig {
    lib_cert::config::CertGenConfig {
        name: name.to_string(),
        organization: "SourceTrait".to_string(),
        validity_days: 3650,
        secret_data: None,
        authority: lib_cert::AuthorityConfig {
            common_name: format!("Test {name} CA"),
        },
        entity: lib_cert::EntityConfig {
            common_name: "localhost".to_string(),
            subject_alt_names: vec![
                "127.0.0.1".to_string(),
                "::1".to_string(),
                "localhost".to_string(),
            ],
            usages: vec![lib_cert::EntityUsage::Server, lib_cert::EntityUsage::Client],
        },
    }
}

/// A fresh self-signed CA and key pair named `name`, minted into `<dir>/certs`.
///
/// Its entity's SANs are loopback, so one pair serves a host's Channel and its
/// remote links alike; a peer knows it by byte match.
pub fn generate_keys(name: &str, dir: &Path) -> GeneratedKeys {
    let files = lib_cert::generate(&key_config(name), dir).expect("generate keys");
    GeneratedKeys {
        ca: files.authority_public,
        public_key: files.entity_public,
        private_key: files.entity_private,
    }
}

/// `<cwd>/.grammar/config.toml` written, `identity` its `[identity]`; its path.
///
/// `remote`, `[[channel]]` entries or nothing, goes to `remotes.toml` beside it.
/// Each path is a TOML literal string, so a Windows path's backslashes stay as
/// written.
pub fn write_host_config(cwd: &Path, identity: &GeneratedKeys, remote: &str) -> PathBuf {
    let dir = cwd.join(".grammar");
    std::fs::create_dir_all(&dir).expect("mkdir .grammar");
    let toml = format!(
        "[identity]\ncert = '{cert}'\nkey = '{key}'\n",
        cert = identity.public_key.display(),
        key = identity.private_key.display(),
    );
    let path = dir.join("config.toml");
    std::fs::write(&path, toml).expect("write .grammar/config.toml");
    if !remote.is_empty() {
        std::fs::write(dir.join("remotes.toml"), remote).expect("write .grammar/remotes.toml");
    }
    path
}

/// A `[[channel]]` entry, `peer`, its connect dialing `peer_addr`.
pub fn remote_connector(peer_addr: &str, peer_public: &Path) -> String {
    remote_connector_as("peer", peer_addr, peer_public)
}

/// A `[[channel]]` entry, `peer`, its listen binding `listen`.
pub fn remote_listener(listen: &str, peer_public: &Path) -> String {
    remote_listener_as("peer", listen, peer_public)
}

/// A `[[channel]]` entry named `alias`, its connect dialing `peer_addr`.
pub fn remote_connector_as(alias: &str, peer_addr: &str, peer_public: &Path) -> String {
    format!(
        "\n[[channel]]\nalias = \"{alias}\"\naddress = \"{peer_addr}\"\ncert = '{pk}'\n",
        pk = peer_public.display(),
    )
}

/// A `[[channel]]` entry named `alias`, its listen binding `listen`.
pub fn remote_listener_as(alias: &str, listen: &str, peer_public: &Path) -> String {
    format!(
        "\n[[channel]]\nalias = \"{alias}\"\nlisten = \"{listen}\"\ncert = '{pk}'\n",
        pk = peer_public.display(),
    )
}

/// A free loopback TCP port: bound at :0, read back, and released.
///
/// The tiny race is made negligible by the `--test-threads=1` serial run.
pub fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind probe port");
    listener.local_addr().expect("probe local_addr").port()
}

/// One Channel packet: its scalar envelope fields, and `event` as its raw NUON.
///
/// The raw NUON serves coarse `contains` assertions.
#[derive(Debug, Clone)]
pub struct Packet {
    pub id: String,
    pub from: String,
    pub model: String,
    /// The files in the packet's own directory, `<id>/`, each with its size.
    pub attached: Option<Vec<(String, i64)>>,
    pub event: String,
    /// The full raw NUON wire line, which a client renders as a notification.
    ///
    /// A client truncates it, so a spilled-pointer packet can be asserted under the
    /// cap.
    pub raw: String,
}

/// Parse one NUON wire line. Returns None when the line is not a packet.
pub fn parse_line(line: &str) -> Option<Packet> {
    let packet = try_nuon(line)?;
    let field = |k: &str| packet.get(k).and_then(NuVal::as_str).unwrap_or("").to_string();
    let model = field("model");
    if model.is_empty() {
        return None;
    }
    let attached = packet.get("attached").and_then(NuVal::as_list).map(|rows| {
        rows.iter()
            .filter_map(|row| {
                let file = row.get("file").and_then(NuVal::as_str)?.to_string();
                let NuVal::Filesize(size) = row.get("size")? else {
                    return None;
                };
                Some((file, *size))
            })
            .collect::<Vec<_>>()
    });
    Some(Packet {
        id: field("id"),
        from: field("from"),
        model,
        attached,
        event: packet.get("event").map_or_else(|| "{}".to_string(), |event| event.to_string()),
        raw: line.to_string(),
    })
}

fn wss_host_port(wss: &str) -> (String, u16) {
    let rest = wss.strip_prefix("wss://").unwrap_or(wss);
    let (host, port) = rest.rsplit_once(':').expect("wss host:port");
    (host.to_string(), port.parse().expect("wss port"))
}

fn client_config(ca_pem: &Path) -> Arc<tokio_rustls::rustls::ClientConfig> {
    let ca = CertificateDer::from_pem_file(ca_pem).expect("load test CA pem");
    let mut roots = tokio_rustls::rustls::RootCertStore::empty();
    roots.add(ca).expect("add test CA to root store");
    let provider = Arc::new(tokio_rustls::rustls::crypto::ring::default_provider());
    let config = tokio_rustls::rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("protocol versions")
        .with_root_certificates(roots)
        .with_no_client_auth();
    Arc::new(config)
}

/// A real TLS WebSocket client on `wss`, trusting `ca_pem` alone, lines to `tx`.
///
/// Every raw packet line is forwarded. The thread ends when the socket closes (the
/// host died) or the receiver is dropped. TLS is established before the WebSocket
/// handshake, so the handshake rides a `ws://` request.
fn spawn_ws_reader(wss: String, ca_pem: PathBuf, tx: Sender<String>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("ws reader runtime");
        rt.block_on(async move {
            let connector = tokio_rustls::TlsConnector::from(client_config(&ca_pem));
            let (host, port) = wss_host_port(&wss);
            let tcp = tokio::net::TcpStream::connect((host.as_str(), port))
                .await
                .expect("wss tcp connect");
            let ip: IpAddr = host.parse().expect("wss host is an ip");
            let server_name = tokio_rustls::rustls::pki_types::ServerName::IpAddress(ip.into());
            let tls = connector.connect(server_name, tcp).await.expect("wss tls");
            let request = format!("ws://{host}:{port}");
            let (mut ws, _resp) = tokio_tungstenite::client_async(request.as_str(), tls)
                .await
                .expect("wss handshake");
            while let Some(msg) = ws.next().await {
                match msg {
                    Ok(tokio_tungstenite::tungstenite::Message::Text(t)) => {
                        for line in t.as_str().lines() {
                            if !line.is_empty() && tx.send(line.to_string()).is_err() {
                                return;
                            }
                        }
                    }
                    Ok(tokio_tungstenite::tungstenite::Message::Close(_)) | Err(_) => return,
                    _ => {}
                }
            }
        });
    })
}

/// A simulated rmcp consumer: the stdio `Host` plus a live WSS Channel client.
pub struct Consumer {
    pub host: Host,
    /// The Channel's inbox, as the open reported it.
    pub inbox: PathBuf,
    /// The host's nom, as its Open packet carries it.
    pub nom: String,
    rx: Receiver<String>,
    _reader: JoinHandle<()>,
}

impl Consumer {
    /// The claude-code handshake: the open, a real WSS client on it, then the verify.
    ///
    /// The client trusts `ca_pem` and sees the `mcp/channel/Open` packet before the
    /// verify.
    pub fn attach(mut host: Host, ca_pem: &Path) -> Self {
        let open = host.interface("interface channel open async");
        let result = &envelope(&open)["result"];
        let wss = result["wss"]
            .as_str()
            .expect("`interface channel open async` returns a wss url")
            .to_string();
        let inbox = PathBuf::from(
            result["inbox"]
                .as_str()
                .expect("`interface channel open async` returns its inbox"),
        );
        let (tx, rx) = mpsc::channel();
        let reader = spawn_ws_reader(wss, ca_pem.to_path_buf(), tx);
        let first_raw = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the hub sends mcp/channel/Open on connect");
        let first = parse_line(&first_raw).expect("the Open packet should parse");
        assert_eq!(
            first.model, "mcp/channel/Open",
            "the first packet must be the handshake; got {first:?}",
        );
        let nom = NuVal::from_nuon(&first.event)
            .ok()
            .and_then(|event| event["mcp_nom"].as_str().map(str::to_string))
            .unwrap_or_else(|| panic!("the Open packet carries the host's nom; got {first:?}"));
        let verified = host.interface("interface channel ready");
        assert!(
            !has_error_path(&verified),
            "`interface channel ready` should succeed once claimed; got {verified}",
        );
        Self { host, inbox, nom, rx, _reader: reader }
    }

    /// Read a file the Channel left in its inbox, by its inbox-relative path.
    pub fn read_inbox(&self, rel: &str) -> String {
        let path = self.inbox.join(rel);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read inbox file {}: {e}", path.display()))
    }

    /// The first packet matching `pred` within `timeout`, or none.
    ///
    /// Each received line is parsed before the predicate sees it.
    pub fn await_packet(
        &mut self,
        pred: impl Fn(&Packet) -> bool,
        timeout: Duration,
    ) -> Option<Packet> {
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.checked_duration_since(Instant::now())?;
            let raw = self.rx.recv_timeout(remaining).ok()?;
            if let Some(packet) = parse_line(&raw)
                && pred(&packet)
            {
                return Some(packet);
            }
        }
    }
}
