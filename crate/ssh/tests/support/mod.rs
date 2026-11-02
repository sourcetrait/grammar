//! A test namespace on disk, its homes and its workdir, and grammar-ssh over pipes.
#![allow(dead_code, unused_imports)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use sourcetrait_grammar_client::{Session, SessionEvent};
use sourcetrait_grammar_protocol::{
    ClientHello, FrameCodec, HELLO_BUDGET, HelloReply, Instance, Limits, McpFrame, SshFrame,
    TransportError, decode_payload, payload,
};
use sourcetrait_grammar_ssh::CONFIG_FILE;
use sourcetrait_grammar_ssh::guts::{Config, SCRATCH_PREFIX};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::codec::{FramedRead, FramedWrite};

/// One id's homes and workdir under a scratch root, and the environment naming them.
pub struct TestInstance {
    pub root: tempfile::TempDir,
    pub id: String,
    pub env: BTreeMap<String, String>,
}

impl TestInstance {
    /// `id`'s homes and workdir made, and the environment naming them.
    pub fn new(id: &str) -> Self {
        let root = tempfile::tempdir().expect("a scratch root");
        let mut env = BTreeMap::new();
        for (var, dir) in [
            ("UENV_USR_DATA", "data"),
            ("UENV_USR_CACHE", "cache"),
            ("UENV_USR_TMP", "tmp"),
            ("UENV_USR_TRANSIENT", "run"),
        ] {
            let path = root.path().join(dir);
            std::fs::create_dir_all(&path).expect("a home");
            env.insert(var.to_string(), path.to_string_lossy().into_owned());
        }
        std::fs::create_dir_all(root.path().join("work")).expect("the workdir");
        env.insert("HOME".to_string(), root.path().to_string_lossy().into_owned());
        if let Ok(path) = std::env::var("PATH") {
            env.insert("PATH".to_string(), path);
        }
        // Windows gives every process its system variables; a program such as
        // ssh-keygen fails without them, so a cleared environment keeps them.
        #[cfg(windows)]
        for var in ["SystemRoot", "SystemDrive", "windir", "ComSpec", "PATHEXT", "TEMP", "TMP", "USERPROFILE", "APPDATA", "LOCALAPPDATA", "ProgramData"] {
            if let Ok(value) = std::env::var(var) {
                env.insert(var.to_string(), value);
            }
        }
        Self { root, id: id.to_string(), env }
    }

    /// The workdir, grammar-ssh's and its config file's.
    pub fn workdir(&self) -> PathBuf {
        self.root.path().join("work")
    }

    /// `text` as the workdir's config file.
    pub fn write_config(&self, text: &str) {
        let file = self.workdir().join(CONFIG_FILE);
        std::fs::create_dir_all(file.parent().expect("its directory")).expect("the config's directory");
        std::fs::write(file, text).expect("the config file");
    }

    /// `namespace`'s config, loaded as this process's user.
    pub fn load(&self, namespace: &str) -> Config {
        let uid = sourcetrait_grammar_scratch::current_uid();
        Config::load(uid, &self.id, namespace, &self.workdir(), &self.env).expect("the config loads")
    }

    /// A home under the scratch root.
    pub fn home(&self, name: &str) -> PathBuf {
        self.root.path().join(name)
    }

    /// grammar-ssh's scratch directories in `base`.
    pub fn scratch_dirs(base: &Path) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(base)
            .expect("the base")
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.starts_with(SCRATCH_PREFIX)))
            .collect();
        dirs.sort();
        dirs
    }
}

/// A runtime for a test's async half.
pub fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("a test runtime")
}

/// grammar-ssh serving `namespace` on `instance`'s environment, its stdio piped.
pub fn spawn_ssh(instance: &TestInstance, namespace: &str) -> Child {
    tokio::process::Command::new(env!("CARGO_BIN_EXE_grammar-ssh"))
        .args(["--id", &instance.id, "--namespace", namespace, "--workdir"])
        .arg(instance.workdir())
        .env_clear()
        .envs(&instance.env)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("grammar-ssh runs")
}

/// The hello for `instance`'s `namespace`, asking `requested`.
pub fn hello(instance: &TestInstance, namespace: &str, requested: Limits) -> ClientHello {
    ClientHello::new(Instance::new(&instance.id, namespace).expect("an instance"), requested)
}

/// grammar-ssh as a child and the session opened on it, or the session's refusal.
pub async fn open(
    instance: &TestInstance,
    namespace: &str,
    requested: Limits,
) -> (Child, Result<(Session, UnboundedReceiver<SessionEvent>), TransportError>) {
    let mut child = spawn_ssh(instance, namespace);
    let stdout = child.stdout.take().expect("its stdout");
    let stdin = child.stdin.take().expect("its stdin");
    let opened = Session::open(stdout, stdin, hello(instance, namespace, requested)).await;
    (child, opened)
}

/// `child`'s exit within `deadline`, killed past it.
pub async fn exit_within(child: &mut Child, deadline: Duration) -> ExitStatus {
    match tokio::time::timeout(deadline, child.wait()).await {
        Ok(status) => status.expect("grammar-ssh's exit"),
        Err(_) => {
            let _ = child.kill().await;
            panic!("grammar-ssh didn't exit in {} s", deadline.as_secs());
        }
    }
}

/// `child`'s stderr, read to its end.
pub async fn stderr_of(child: &mut Child) -> String {
    let mut text = String::new();
    if let Some(mut stderr) = child.stderr.take() {
        let _ = tokio::io::AsyncReadExt::read_to_string(&mut stderr, &mut text).await;
    }
    text
}

/// A session spoken frame by frame, for a test of the stream itself.
pub struct RawSession {
    pub frames: FramedRead<ChildStdout, FrameCodec>,
    pub sink: FramedWrite<ChildStdin, FrameCodec>,
}

impl RawSession {
    /// The hello sent on `child`'s pipes and grammar-ssh's reply read.
    pub async fn open(child: &mut Child, hello: ClientHello) -> (Self, HelloReply) {
        let mut frames = FramedRead::new(child.stdout.take().expect("its stdout"), FrameCodec::new());
        let mut sink = FramedWrite::new(child.stdin.take().expect("its stdin"), FrameCodec::new());
        sink.send(payload(&hello)).await.expect("the hello sent");
        let bytes = frames.next().await.expect("a reply").expect("its frame");
        let reply = decode_payload::<HelloReply>(&bytes, HELLO_BUDGET).expect("a reply decoded");
        if let HelloReply::Accepted(accepted) = &reply {
            frames.decoder_mut().set_ceiling(accepted.effective.frame);
            sink.encoder_mut().set_ceiling(accepted.effective.frame);
        }
        (Self { frames, sink }, reply)
    }

    /// One frame sent.
    pub async fn send(&mut self, frame: &McpFrame) {
        self.sink.send(payload(frame)).await.expect("a frame sent");
    }

    /// grammar-ssh's next frame, none at the stream's end.
    pub async fn next(&mut self) -> Option<SshFrame> {
        let bytes = self.frames.next().await?.ok()?;
        Some(decode_payload::<SshFrame>(&bytes, sourcetrait_grammar_protocol::LIMITS_DEFAULT.decode_budget()).expect("a frame decoded"))
    }
}
