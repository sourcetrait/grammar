//! OpenSSH's sshd on loopback as the test's own user, and the worker its sessions run.

use std::fs;
use std::io::Read;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use russh::keys::{Algorithm, PrivateKey, key::safe_rng, ssh_key::LineEnding};
use sourcetrait_grammar_client::Endpoint;

use crate::grammar_mcp_bin;

/// Where OpenSSH installs its server, on Linux and on macOS.
pub const SSHD: &str = "/usr/sbin/sshd";

/// The id a test worker runs as.
pub const WORKER_ID: &str = "fixture";

/// How long a started sshd may take to listen.
const LISTEN_DEADLINE: Duration = Duration::from_secs(10);

/// How many ports a start tries before it gives up.
const PORT_TRIES: usize = 5;

/// The `grammar-ssh` binary, beside the running test binary's `deps/`.
pub fn grammar_ssh_bin() -> PathBuf {
    grammar_mcp_bin().with_file_name("grammar-ssh")
}

/// The test's own user, by name.
pub fn current_user() -> String {
    nix::unistd::User::from_uid(nix::unistd::getuid())
        .expect("the user database")
        .expect("the test's user")
        .name
}

/// `path` written owner-only.
fn write_private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).expect("a private file");
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).expect("its mode");
}

/// A scratch tree, owner-only.
fn private_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a scratch tree");
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).expect("its mode");
    dir
}

/// A key pair of a test's own: its private key file and its public line.
pub struct TestKey {
    pub file: PathBuf,
    pub public: String,
}

impl TestKey {
    /// A fresh Ed25519 key, its private half written to `file` owner-only.
    pub fn new(file: &Path) -> Self {
        let key = PrivateKey::random(&mut safe_rng(), Algorithm::Ed25519).expect("a key");
        let private = key.to_openssh(LineEnding::LF).expect("its OpenSSH form");
        write_private(file, private.as_bytes());
        let public = key.public_key().to_openssh().expect("its public line");
        Self {
            file: file.to_path_buf(),
            public,
        }
    }

    /// The private key file's path as text.
    pub fn path(&self) -> String {
        self.file.display().to_string()
    }
}

/// A worker for an sshd's sessions: a wrapper giving it the test's homes, then `grammar-ssh`.
pub struct TestWorker {
    dir: tempfile::TempDir,
}

impl TestWorker {
    /// The worker's homes, its workdir, and its wrapper, made.
    pub fn new() -> Self {
        let dir = private_dir();
        let root = dir.path();
        for home in ["data", "cache", "tmp", "run", "work", "bin"] {
            fs::create_dir_all(root.join(home)).expect("a worker's home");
        }
        let worker_bin = grammar_ssh_bin();
        assert!(worker_bin.is_file(), "grammar-ssh is built beside grammar-mcp: {}", worker_bin.display());
        let exports: String = [
            ("UENV_USR_DATA", "data"),
            ("UENV_USR_CACHE", "cache"),
            ("UENV_USR_TMP", "tmp"),
            ("UENV_USR_TRANSIENT", "run"),
        ]
        .iter()
        .map(|(var, home)| format!("export {var}='{}'\n", root.join(home).display()))
        .collect();
        let wrapper = root.join("bin").join("grammar-ssh-worker");
        fs::write(&wrapper, format!("#!/bin/sh\n{exports}exec '{}' \"$@\"\n", worker_bin.display()))
            .expect("the wrapper");
        fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).expect("the wrapper's mode");
        Self { dir }
    }

    /// The worker's scratch tree.
    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    /// The wrapper an endpoint runs as its command.
    pub fn command(&self) -> PathBuf {
        self.root().join("bin").join("grammar-ssh-worker")
    }

    /// The worker's working directory.
    pub fn workdir(&self) -> PathBuf {
        self.root().join("work")
    }

    /// A home of the worker's, by its directory's name.
    pub fn home(&self, name: &str) -> PathBuf {
        self.root().join(name)
    }

    /// The worker scratch directories in its temporary home.
    pub fn scratch_dirs(&self) -> Vec<PathBuf> {
        fs::read_dir(self.home("tmp"))
            .expect("the temporary home")
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.starts_with("grammar-ssh.")))
            .collect()
    }

    /// The namespace lock's path for `namespace`.
    pub fn lock(&self, namespace: &str) -> PathBuf {
        self.home("data").join("grammar").join("mcp").join(WORKER_ID).join(namespace).join("host.lock")
    }
}

impl Default for TestWorker {
    fn default() -> Self {
        Self::new()
    }
}

/// OpenSSH's sshd on loopback as the test's own user, killed at its drop.
pub struct Sshd {
    dir: tempfile::TempDir,
    child: Child,
    pub port: u16,
    pub user: String,
    pub host_key: TestKey,
    /// The key the sshd authorizes.
    pub user_key: TestKey,
    /// A key the sshd doesn't authorize.
    pub stranger_key: TestKey,
}

impl Sshd {
    /// An sshd listening on a free loopback port, its keys and config its own.
    pub fn start() -> Self {
        assert!(Path::new(SSHD).is_file(), "OpenSSH's sshd is needed at {SSHD}");
        let dir = private_dir();
        let root = dir.path().to_path_buf();
        let host_key = TestKey::new(&root.join("host_ed25519"));
        let user_key = TestKey::new(&root.join("user_ed25519"));
        let stranger_key = TestKey::new(&root.join("stranger_ed25519"));
        write_private(&root.join("authorized_keys"), format!("{}\n", user_key.public).as_bytes());
        let mut failures = Vec::new();
        for _ in 0..PORT_TRIES {
            let port = free_port();
            let config = root.join("sshd_config");
            write_private(&config, config_text(&root, port).as_bytes());
            let log = fs::File::create(root.join("sshd.log")).expect("sshd's log");
            let mut child = Command::new(SSHD)
                .args(["-D", "-e", "-f"])
                .arg(&config)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(log)
                .spawn()
                .expect("sshd runs");
            if listening(&mut child, port) {
                return Self {
                    dir,
                    child,
                    port,
                    user: current_user(),
                    host_key,
                    user_key,
                    stranger_key,
                };
            }
            let _ = child.kill();
            let _ = child.wait();
            failures.push(read_log(&root.join("sshd.log")));
        }
        panic!("sshd never listened: {}", failures.join("\n---\n"));
    }

    /// sshd's log so far.
    pub fn log(&self) -> String {
        read_log(&self.dir.path().join("sshd.log"))
    }

    /// An endpoint of this sshd as `alias`, its command `worker`'s wrapper.
    pub fn endpoint(&self, alias: &str, worker: &TestWorker) -> Endpoint {
        Endpoint {
            alias: alias.to_string(),
            host: "127.0.0.1".to_string(),
            port: self.port,
            user: self.user.clone(),
            key: self.user_key.path(),
            host_keys: vec![self.host_key.public.clone()],
            command: worker.command().display().to_string(),
            id: WORKER_ID.to_string(),
            namespace: "default".to_string(),
            workdir: worker.workdir().display().to_string(),
        }
    }
}

impl Drop for Sshd {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// `endpoint` as a remotes file's `[[engine]]` table.
pub fn engine_toml(endpoint: &Endpoint) -> String {
    let keys: Vec<String> = endpoint.host_keys.iter().map(|key| format!("\"{key}\"")).collect();
    format!(
        "[[engine]]\nalias = \"{}\"\nhost = \"{}\"\nport = {}\nuser = \"{}\"\nkey = \"{}\"\nhost_keys = [{}]\ncommand = \"{}\"\nid = \"{}\"\nnamespace = \"{}\"\nworkdir = \"{}\"\n\n",
        endpoint.alias,
        endpoint.host,
        endpoint.port,
        endpoint.user,
        endpoint.key,
        keys.join(", "),
        endpoint.command,
        endpoint.id,
        endpoint.namespace,
        endpoint.workdir,
    )
}

/// A loopback proxy to a port whose connections a test cuts, as a dropped network does.
pub struct Cuttable {
    pub port: u16,
    cut: tokio::sync::watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}

impl Cuttable {
    /// A proxy on a free loopback port to `target`'s.
    pub async fn start(target: u16) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a loopback port");
        let port = listener.local_addr().expect("its address").port();
        let (cut, watch) = tokio::sync::watch::channel(false);
        let task = tokio::spawn(async move {
            while let Ok((inbound, _)) = listener.accept().await {
                let watch = watch.clone();
                tokio::spawn(async move {
                    let Ok(outbound) = tokio::net::TcpStream::connect(("127.0.0.1", target)).await else {
                        return;
                    };
                    let (mut in_read, mut in_write) = inbound.into_split();
                    let (mut out_read, mut out_write) = outbound.into_split();
                    tokio::select! {
                        _ = tokio::io::copy(&mut in_read, &mut out_write) => {}
                        _ = tokio::io::copy(&mut out_read, &mut in_write) => {}
                        _ = cut_signal(watch) => {}
                    }
                });
            }
        });
        Self { port, cut, task }
    }

    /// Every connection through the proxy dropped, both its sockets closed.
    pub fn cut(&self) {
        let _ = self.cut.send(true);
    }
}

/// Ready once the proxy is cut, or its cutter gone.
async fn cut_signal(mut watch: tokio::sync::watch::Receiver<bool>) {
    loop {
        let cut = *watch.borrow();
        if cut || watch.changed().await.is_err() {
            return;
        }
    }
}

impl Drop for Cuttable {
    fn drop(&mut self) {
        self.cut();
        self.task.abort();
    }
}

/// Whether process `pid` is still alive.
pub fn alive(pid: u32) -> bool {
    nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), None).is_ok()
}

/// Whether `check` holds within `wait`, polled on the calling thread.
pub fn eventually(wait: Duration, check: impl Fn() -> bool) -> bool {
    let deadline = Instant::now() + wait;
    while Instant::now() < deadline {
        if check() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    check()
}

/// Whether `check` holds within `wait`, polled.
pub async fn holds_within(wait: Duration, check: impl Fn() -> bool) -> bool {
    let deadline = Instant::now() + wait;
    while Instant::now() < deadline {
        if check() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    check()
}

/// A port free on loopback a moment ago.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0").expect("a loopback port").local_addr().expect("its address").port()
}

/// Whether `child` listens on `port` within the deadline, before it exits.
fn listening(child: &mut Child, port: u16) -> bool {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let deadline = Instant::now() + LISTEN_DEADLINE;
    while Instant::now() < deadline {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return false;
        }
        if TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

/// A log file's text, or why it has none.
fn read_log(path: &Path) -> String {
    let mut text = String::new();
    match fs::File::open(path).and_then(|mut file| file.read_to_string(&mut text)) {
        Ok(_) => text,
        Err(e) => format!("no log at {}: {e}", path.display()),
    }
}

/// The sshd's config: loopback, its own keys, public keys alone, nothing forwarded.
fn config_text(root: &Path, port: u16) -> String {
    let at = |name: &str| root.join(name).display().to_string();
    [
        format!("ListenAddress 127.0.0.1:{port}"),
        format!("HostKey {}", at("host_ed25519")),
        format!("PidFile {}", at("sshd.pid")),
        format!("AuthorizedKeysFile {}", at("authorized_keys")),
        "AuthenticationMethods publickey".to_string(),
        "PubkeyAuthentication yes".to_string(),
        "PasswordAuthentication no".to_string(),
        "KbdInteractiveAuthentication no".to_string(),
        "PermitRootLogin no".to_string(),
        "PermitUserRC no".to_string(),
        "PermitUserEnvironment no".to_string(),
        "AllowTcpForwarding no".to_string(),
        "AllowStreamLocalForwarding no".to_string(),
        "AllowAgentForwarding no".to_string(),
        "X11Forwarding no".to_string(),
        "PermitTunnel no".to_string(),
        "PrintMotd no".to_string(),
        "StrictModes no".to_string(),
        "UsePAM no".to_string(),
        "LogLevel VERBOSE".to_string(),
    ]
    .join("\n")
        + "\n"
}
