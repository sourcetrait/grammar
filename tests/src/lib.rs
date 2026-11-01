//! System-test harness for `grammar-mcp`.
//!
//! These tests spawn the REAL host binary - the server, MCP over stdio on its
//! in-process engine, or the one-shot `cli` subcommand - so they are SYSTEM
//! tests and live outside the mcp crate to keep its own `cargo test` fast. The
//! binary is located from the test binary's own path (`current_exe()` -> the
//! `target/<profile>/grammar-mcp` sibling of `deps/`), so it must be built
//! first, in the same target directory: `cargo build -p sourcetrait_grammar_mcp`,
//! with `--features test-hooks` for the lifecycle tests.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

pub use nuval::NuVal;

pub mod remote;
pub use remote::*;

#[cfg(unix)]
pub mod sshd;
#[cfg(unix)]
pub use sshd::*;

/// The `grammar-mcp` host binary, beside the running test binary's `deps/`.
///
/// The test binary's own path with its file name and `deps/` popped: the
/// profile's directory, where the binary is built.
pub fn grammar_mcp_bin() -> PathBuf {
    let mut dir = std::env::current_exe().expect("current_exe");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    dir.join("grammar-mcp")
}

/// Run the binary once as a subprocess, its homes under `scratch`.
///
/// For the `cli` surface or a check of a failed start; the output is captured,
/// the `cli` surface printing its envelope's NUON text.
pub fn run_output(scratch: &Path, args: &[&str]) -> Output {
    run_output_env(scratch, args, &[])
}

/// `run_output` with extra env vars, for a start its environment decides.
pub fn run_output_env(scratch: &Path, args: &[&str], envs: &[(&str, &str)]) -> Output {
    let data = scratch.join("data");
    let cache = scratch.join("cache");
    std::fs::create_dir_all(&data).expect("mkdir data");
    std::fs::create_dir_all(&cache).expect("mkdir cache");
    let (tmp, run) = scratch_bases(scratch);
    Command::new(grammar_mcp_bin())
        .args(args)
        .env("XDG_DATA_HOME", &data)
        .env("XDG_CACHE_HOME", &cache)
        .env("UENV_USR_DATA", &data)
        .env("UENV_USR_CACHE", &cache)
        .env("UENV_USR_TMP", &tmp)
        .env("UENV_USR_TRANSIENT", &run)
        .envs(envs.iter().copied())
        .output()
        .expect("run grammar-mcp")
}

/// A host's temporary and runtime bases under `scratch`, private to it.
///
/// What a host killed outright leaves behind stays in the test's directory.
fn scratch_bases(scratch: &Path) -> (PathBuf, PathBuf) {
    let tmp = scratch.join("tmp");
    let run = scratch.join("run");
    for base in [&tmp, &run] {
        std::fs::create_dir_all(base).expect("mkdir a scratch base");
        #[cfg(unix)]
        std::fs::set_permissions(base, std::fs::Permissions::from_mode(0o700))
            .expect("a scratch base's mode");
    }
    (tmp, run)
}

pub fn stdout_str(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The envelope the `cli` surface printed, read from its NUON text.
pub fn stdout_envelope(out: &Output) -> NuVal {
    nuon(stdout_str(out).trim())
}

/// A `grammar-mcp` server child driven over JSON-RPC stdio, killed on drop.
pub struct Host {
    child: Child,
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
    next_id: u64,
    data_home: PathBuf,
    cache_home: PathBuf,
    /// Responses read while waiting for another id, kept by id.
    ///
    /// A caller can send several requests at once and read each back; `read_id`
    /// would otherwise drop a response whose id it is not waiting for.
    pending: HashMap<u64, Value>,
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Host {
    /// Spawn with its homes under `scratch/{data,cache}` and no extra CLI args.
    pub fn spawn(scratch: &Path) -> Self {
        Self::spawn_full(scratch, &[], &[])
    }

    /// Spawn with extra CLI args (e.g. `--deny`, `--id`, `--namespace`).
    pub fn spawn_args(scratch: &Path, args: &[&str]) -> Self {
        Self::spawn_full(scratch, args, &[])
    }

    /// Spawn with extra CLI args and extra env vars, such as `USER` or `HOME`.
    pub fn spawn_full(scratch: &Path, args: &[&str], envs: &[(&str, &str)]) -> Self {
        Self::spawn_core(scratch, args, envs, None, None)
    }

    /// `spawn_full` with the host's own stderr written to `stderr_log`.
    pub fn spawn_logged(
        scratch: &Path,
        args: &[&str],
        envs: &[(&str, &str)],
        stderr_log: &Path,
    ) -> Self {
        Self::spawn_core(scratch, args, envs, None, Some(stderr_log))
    }

    /// Spawn with extra CLI args in a working directory.
    ///
    /// For a host whose `<cwd>/.grammar/config.toml`, its identity and remotes,
    /// must resolve.
    pub fn spawn_in(scratch: &Path, args: &[&str], cwd: &Path) -> Self {
        Self::spawn_core(scratch, args, &[], Some(cwd), None)
    }

    /// A host with its stderr in `stderr_log` and nothing sent.
    ///
    /// The test owns the first message, as a client probing before `initialize`
    /// does.
    pub fn spawn_uninitialized(scratch: &Path, stderr_log: &Path) -> Self {
        Self::start(scratch, &[], &[], None, Some(stderr_log))
    }

    /// `spawn_uninitialized` with extra CLI args.
    pub fn spawn_uninitialized_args(scratch: &Path, args: &[&str], stderr_log: &Path) -> Self {
        Self::start(scratch, args, &[], None, Some(stderr_log))
    }

    fn spawn_core(
        scratch: &Path,
        args: &[&str],
        envs: &[(&str, &str)],
        cwd: Option<&Path>,
        stderr_log: Option<&Path>,
    ) -> Self {
        let mut host = Self::start(scratch, args, envs, cwd, stderr_log);
        host.initialize();
        host
    }

    /// Spawn the binary on its bare entry, MCP over stdio.
    fn start(
        scratch: &Path,
        args: &[&str],
        envs: &[(&str, &str)],
        cwd: Option<&Path>,
        stderr_log: Option<&Path>,
    ) -> Self {
        let data_home = scratch.join("data");
        let cache_home = scratch.join("cache");
        std::fs::create_dir_all(&data_home).expect("mkdir data");
        std::fs::create_dir_all(&cache_home).expect("mkdir cache");
        let (tmp, run) = scratch_bases(scratch);
        let stderr = match stderr_log {
            Some(path) => Stdio::from(
                std::fs::File::create(path).expect("create the host's stderr log"),
            ),
            None => Stdio::inherit(),
        };
        let mut cmd = Command::new(grammar_mcp_bin());
        cmd.args(args)
            .env("XDG_DATA_HOME", &data_home)
            .env("XDG_CACHE_HOME", &cache_home)
            .env("UENV_USR_DATA", &data_home)
            .env("UENV_USR_CACHE", &cache_home)
            .env("UENV_USR_TMP", &tmp)
            .env("UENV_USR_TRANSIENT", &run)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(stderr);
        for (k, v) in envs {
            cmd.env(k, v);
        }
        if let Some(cwd) = cwd {
            cmd.current_dir(cwd);
        }
        let mut child = cmd.spawn().expect("spawn grammar-mcp");
        let stdin = child.stdin.take().expect("host stdin");
        let stdout = BufReader::new(child.stdout.take().expect("host stdout"));
        Self {
            child,
            stdin,
            stdout,
            next_id: 1,
            data_home,
            cache_home,
            pending: HashMap::new(),
        }
    }

    /// The spawned host's pid.
    ///
    /// For a test that inspects its process tree from `/proc`, such as that it
    /// reaps the orphans the subreaper adopts.
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// The data home the child was spawned with, the root of its namespace's tree.
    pub fn data_home(&self) -> &Path {
        &self.data_home
    }

    /// The cache home the child was spawned with.
    pub fn cache_home(&self) -> &Path {
        &self.cache_home
    }

    /// Signal the host and wait for it to exit, returning its status.
    ///
    /// The distinction this exists to measure: a host whose handler ran exits on its
    /// own terms (`code() == Some(128 + signo)`), while one that merely took the
    /// signal's default action is reported as killed by the signal (`code() == None`,
    /// `signal() == Some(signo)`). Only the former proves the shutdown sweep ran.
    #[cfg(unix)]
    pub fn signal_and_wait(
        &mut self,
        signal: nix::sys::signal::Signal,
        timeout: Duration,
    ) -> Option<std::process::ExitStatus> {
        let pid = nix::unistd::Pid::from_raw(self.child.id() as i32);
        nix::sys::signal::kill(pid, Some(signal)).expect("signal the host");
        let deadline = Instant::now() + timeout;
        loop {
            match self.child.try_wait().expect("try_wait") {
                Some(status) => return Some(status),
                None if Instant::now() >= deadline => return None,
                None => std::thread::sleep(Duration::from_millis(25)),
            }
        }
    }

    /// `read_id`, but tolerant of the host dying mid-request.
    ///
    /// `read_id` panics on EOF, which is the right default for a test whose host is
    /// expected to answer. A test asserting that some input does not kill the host
    /// needs the other behaviour: EOF, the host's stdout closed, is what a dead host
    /// looks like from here, the observation itself, and panicking on it throws
    /// away the chance to report what actually happened. Pair it with
    /// `wait_for_exit` and `describe_exit` to name the death mode.
    pub fn try_read_id(
        &mut self,
        expected_id: u64,
    ) -> Option<Value> {
        if let Some(msg) = self.pending.remove(&expected_id) {
            return Some(msg);
        }
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if Instant::now() >= deadline {
                return None;
            }
            let mut line = String::new();
            match self.stdout.read_line(&mut line) {
                Ok(0) | Err(_) => return None,
                Ok(_) => {}
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(msg) = serde_json::from_str::<Value>(trimmed) else {
                continue;
            };
            match msg.get("id").and_then(|v| v.as_u64()) {
                Some(id) if id == expected_id => return Some(msg),
                Some(id) => {
                    self.pending.insert(id, msg);
                }
                None => {}
            }
        }
    }

    /// `try_read_id` for any JSON-RPC id, a string one included.
    ///
    /// Such as Claude Code's `server-discover-probe-1`.
    pub fn try_read_value_id(
        &mut self,
        expected: &Value,
    ) -> Option<Value> {
        if let Some(id) = expected.as_u64() {
            return self.try_read_id(id);
        }
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if Instant::now() >= deadline {
                return None;
            }
            let mut line = String::new();
            match self.stdout.read_line(&mut line) {
                Ok(0) | Err(_) => return None,
                Ok(_) => {}
            }
            let Ok(msg) = serde_json::from_str::<Value>(line.trim()) else {
                continue;
            };
            match msg.get("id") {
                Some(id) if id == expected => return Some(msg),
                Some(id) => {
                    if let Some(id) = id.as_u64() {
                        self.pending.insert(id, msg);
                    }
                }
                None => {}
            }
        }
    }

    /// Wait for the child to exit on its own, returning its status; None on timeout.
    ///
    /// Unlike `signal_and_wait` this sends nothing - it observes a host that is
    /// already on its way out.
    pub fn wait_for_exit(
        &mut self,
        timeout: Duration,
    ) -> Option<std::process::ExitStatus> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.child.try_wait().expect("try_wait") {
                Some(status) => return Some(status),
                None if Instant::now() >= deadline => return None,
                None => std::thread::sleep(Duration::from_millis(25)),
            }
        }
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn send(&mut self, msg: &Value) {
        let line = msg.to_string();
        self.stdin.write_all(line.as_bytes()).expect("write line");
        self.stdin.write_all(b"\n").expect("write newline");
        self.stdin.flush().expect("flush");
    }

    /// The response to `expected_id`, waited for up to 15 seconds.
    ///
    /// A response to another request in flight is kept by id for a later
    /// `read_id` rather than dropped; a notification, which carries no id, is
    /// skipped.
    pub fn read_id(&mut self, expected_id: u64) -> Value {
        self.read_id_within(expected_id, Duration::from_secs(15))
    }

    /// The response to `expected_id`, waited for up to `wait`, as `read_id` reads it.
    pub fn read_id_within(&mut self, expected_id: u64, wait: Duration) -> Value {
        if let Some(msg) = self.pending.remove(&expected_id) {
            return msg;
        }
        let deadline = Instant::now() + wait;
        loop {
            if Instant::now() >= deadline {
                panic!("timed out waiting for response id {expected_id}");
            }
            let mut line = String::new();
            let n = self.stdout.read_line(&mut line).expect("read line");
            if n == 0 {
                panic!("EOF on host stdout waiting for id {expected_id}");
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let msg: Value = serde_json::from_str(trimmed)
                .unwrap_or_else(|e| panic!("parse JSON: {e} from {trimmed:?}"));
            match msg.get("id").and_then(|v| v.as_u64()) {
                Some(id) if id == expected_id => return msg,
                Some(id) => {
                    self.pending.insert(id, msg);
                }
                None => {}
            }
        }
    }

    /// The legacy handshake: `initialize`, then `notifications/initialized`; the
    /// `initialize` response, its server info and instructions.
    pub fn initialize(&mut self) -> Value {
        let id = self.next_id();
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "grammar-system-tests", "version": "0.0.1"}
            }
        }));
        let reply = self.read_id(id);
        self.send(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
        }));
        reply
    }

    /// Send a `tools/call` without reading its response; the request id.
    ///
    /// For requests in flight together, such as a kill mid-call; `read_id(id)`
    /// reads the response later. A bare rig name is author-prefixed, and the
    /// `local` host named unless the arguments name one.
    pub fn request(&mut self, tool: &str, args: Value) -> u64 {
        self.request_as_given(tool, author_prefixed(tool, hosted(args)))
    }

    /// Send a `tools/call` with its arguments exactly as given; the request id.
    pub fn request_as_given(&mut self, tool: &str, args: Value) -> u64 {
        let id = self.next_id();
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {"name": tool, "arguments": args}
        }));
        id
    }

    /// A `tools/call`, a bare rig name author-prefixed.
    ///
    /// A value already carrying an `author/name` passes unchanged.
    pub fn call(&mut self, tool: &str, args: Value) -> Value {
        let id = self.request(tool, args);
        self.read_id(id)
    }

    /// `execute` on a definition and its data's NUON text; JSON text is NUON too.
    pub fn execute(&mut self, definition: &str, data: &str) -> Value {
        self.call("execute", execute_args(definition, data))
    }

    /// `call` of a committed function on its data's NUON text.
    pub fn call_np(&mut self, namepath: &str, data: &str) -> Value {
        self.call("call", json!({"namepath": namepath, "data": data}))
    }

    /// One `interface` command, its arguments Nu literals.
    pub fn interface(&mut self, command: &str) -> Value {
        let id = self.request_interface(command);
        self.read_id(id)
    }

    /// Send an `interface` command without reading its response; the request id.
    pub fn request_interface(&mut self, command: &str) -> u64 {
        self.request("interface", json!({"command": command}))
    }

    /// `interface info`.
    pub fn info(&mut self) -> Value {
        self.interface("interface info")
    }

    /// `interface equip <action>` on a source directory.
    pub fn equip(&mut self, action: &str, source_dir: &str) -> Value {
        self.interface(&format!("interface equip {action} {}", nu_literal(source_dir)))
    }

    /// `interface equip new` for `name` under `root`, a bare name author-prefixed.
    ///
    /// Returns the response and the equip's rig source.
    pub fn equip_new(&mut self, name: &str, root: &Path) -> (Value, PathBuf) {
        let name = if name.contains('/') {
            name.to_string()
        } else {
            format!("sourcetrait/{name}")
        };
        let source = equip_source(root, &name);
        let resp = self.equip("new", source.to_str().unwrap());
        (resp, source.join("rig"))
    }

    /// `interface equip commit`, author-prefixing a bare name.
    pub fn commit(&mut self, name: &str) -> Value {
        self.interface(&format!("interface equip commit {}", nu_literal(&prefixed_equip(name))))
    }

    /// The tools advertised by `tools/list`, each as the client receives it.
    pub fn tools(&mut self) -> Vec<Value> {
        let id = self.next_id();
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": "tools/list"}));
        let resp = self.read_id(id);
        resp["result"]["tools"].as_array().expect("tools array").clone()
    }

    /// The tool names advertised by `tools/list`.
    pub fn tool_names(&mut self) -> Vec<String> {
        self.tools()
            .iter()
            .map(|t| t["name"].as_str().expect("tool name").to_string())
            .collect()
    }
}

/// An equip's source directory under `root`: `root/equip/<name>`.
pub fn equip_source(root: &Path, name: &str) -> PathBuf {
    root.join("equip").join(name)
}

/// A plugin directory under `root`, each built plugin under its kind's directory.
pub fn bundle_plugins(root: &Path, plugins: &[(&str, &str)]) -> PathBuf {
    let dir = root.join("plugin");
    for (kind, name) in plugins {
        let kind_dir = dir.join(kind);
        std::fs::create_dir_all(&kind_dir).expect("mkdir the plugin kind's dir");
        let built = grammar_mcp_bin().with_file_name(name);
        std::fs::copy(&built, kind_dir.join(name)).unwrap_or_else(|e| {
            panic!("copy {}: {e}; build it with `cargo build --bin {name}`", built.display())
        });
    }
    dir
}

/// The builtins' bare names, which the host resolves itself.
const BUILTINS: [&str; 2] = ["gram", "grammar"];

/// The host alias every tool call names for the host it runs on.
pub const LOCAL_HOST: &str = "local";

/// A tool's arguments naming the `local` host, unless they name one already.
pub fn hosted(mut args: Value) -> Value {
    if let Some(object) = args.as_object_mut() {
        object
            .entry("host")
            .or_insert_with(|| Value::String(LOCAL_HOST.to_string()));
    }
    args
}

/// Prepend `sourcetrait/` to a bare equip name in a `call`'s namepath.
///
/// Idempotent: a value already carrying `/`, an `author/name`, passes through
/// unchanged, as does a builtin's bare name.
pub fn author_prefixed(tool: &str, mut args: Value) -> Value {
    if tool == "call"
        && let Some(namepath) = args.get("namepath").and_then(|v| v.as_str())
    {
        args["namepath"] = Value::String(prefixed_namepath(namepath));
    }
    args
}

/// An equip name, `sourcetrait/` prepended when it is bare and not a builtin's.
pub fn prefixed_equip(name: &str) -> String {
    if name.is_empty() || name.contains('/') || BUILTINS.contains(&name) {
        name.to_string()
    } else {
        format!("sourcetrait/{name}")
    }
}

/// A namepath, `sourcetrait/` prepended when its equip is bare and not a builtin's.
pub fn prefixed_namepath(namepath: &str) -> String {
    let equip = namepath.split(':').next().unwrap_or(namepath);
    if equip.is_empty() || equip.contains('/') || BUILTINS.contains(&equip) {
        namepath.to_string()
    } else {
        format!("sourcetrait/{namepath}")
    }
}

/// A string as a Nu literal in an interface command.
///
/// Single-quoted, or raw when it holds a quote.
pub fn nu_literal(text: &str) -> String {
    if text.contains('\'') {
        assert!(!text.contains("'#"), "no raw string holds {text:?}");
        format!("r#'{text}'#")
    } else {
        format!("'{text}'")
    }
}

/// How a child ended, in a form a failure message can carry.
///
/// The distinction is the whole point: a host that exited chose to, and its code says
/// why; a host reported as killed by a signal was taken down by the kernel, and the
/// signal names the mechanism (SIGSEGV = a memory fault, which for a parser means a
/// stack overflow far more often than anything else).
pub fn describe_exit(status: &std::process::ExitStatus) -> String {
    match (status.code(), exit_signal(status)) {
        (Some(code), _) => format!("exited with code {code}"),
        (None, Some(sig)) => {
            let name = match sig {
                6 => " (SIGABRT)",
                9 => " (SIGKILL)",
                11 => " (SIGSEGV)",
                15 => " (SIGTERM)",
                _ => "",
            };
            format!("killed by signal {sig}{name}")
        }
        (None, None) => "ended for an unknown reason".to_string(),
    }
}

/// The signal that ended a child, on Unix.
#[cfg(unix)]
fn exit_signal(status: &std::process::ExitStatus) -> Option<i32> {
    status.signal()
}

/// Off Unix no signal ends a child.
#[cfg(not(unix))]
fn exit_signal(_status: &std::process::ExitStatus) -> Option<i32> {
    None
}

/// An `execute` call's arguments: its definition and its data's NUON text.
pub fn execute_args(definition: &str, data: &str) -> Value {
    json!({"definition": definition, "data": data})
}

/// A NUON text as a NuVal, `None` when the text is not NUON.
pub fn try_nuon(text: &str) -> Option<NuVal> {
    NuVal::from_nuon(text).ok()
}

/// A NUON text as a NuVal, panicking on text that is not NUON.
pub fn nuon(text: &str) -> NuVal {
    NuVal::from_nuon(text).unwrap_or_else(|e| panic!("not NUON: {text:?}: {e}"))
}

/// A `tools/call` response's envelope, read from its one NUON text content.
///
/// Nothing when it carries none, as a JSON-RPC error does.
pub fn envelope(resp: &Value) -> NuVal {
    resp["result"]["content"][0]["text"].as_str().map(nuon).unwrap_or(NuVal::Nothing)
}

/// The envelope's error table, when it holds one.
pub fn envelope_error(resp: &Value) -> Option<NuVal> {
    envelope(resp).get("error").cloned()
}

pub fn has_error_path(resp: &Value) -> bool {
    envelope_error(resp).is_some()
}

/// The first error row's `kind`.
pub fn envelope_error_kind(resp: &Value) -> Option<String> {
    error_kinds(resp).into_iter().next()
}

/// Every error row's `kind`.
pub fn error_kinds(resp: &Value) -> Vec<String> {
    envelope_error(resp)
        .map(|error| {
            error
                .as_list()
                .unwrap_or_default()
                .iter()
                .filter_map(|row| row.get("kind").and_then(NuVal::as_str).map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub fn has_kind(resp: &Value, kind: &str) -> bool {
    error_kinds(resp).iter().any(|k| k == kind)
}

/// The envelope's error table as its raw NUON, or the whole response's JSON
/// when it carries none.
pub fn error_message(resp: &Value) -> String {
    envelope_error(resp)
        .map(|e| e.to_string())
        .unwrap_or_else(|| resp.to_string())
}

/// The success envelope, or `None` when the response carries an `error` or none.
pub fn extract_envelope(resp: &Value) -> Option<NuVal> {
    let envelope = envelope(resp);
    if envelope.is_nothing() || envelope.get("error").is_some() {
        return None;
    }
    Some(envelope)
}

pub fn write_source(dir: &Path, rel: &str, contents: &str) {
    let target = dir.join(rel);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).expect("mkdir");
    }
    std::fs::write(&target, contents).expect("write source");
}

/// A call's `main` from a record of `data_fields` to one of `result_fields`.
pub fn valid_function_source(data_fields: &str, result_fields: &str, body: &str) -> String {
    format!(
        "export def main [data: record<{data_fields}>]: nothing -> record<{result_fields}> {{\n{body}\n}}\n",
    )
}

/// The on-disk namespace subtree for `(id, namespace)` under a data home.
pub fn namespace_dir(data_home: &Path, id: &str, namespace: &str) -> PathBuf {
    data_home
        .join("grammar")
        .join("mcp")
        .join(id)
        .join(namespace)
}
