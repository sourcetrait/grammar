//! In-process test-support harness driving the real tool handlers.

use crate::*;

use std::sync::Once;

pub use nuval::NuVal;

static TEST_ENV: Once = Once::new();

const INPROC_PREFIX: &str = "grammar_inproc_";

/// Remove the temp namespaces left by test processes that have since exited.
fn sweep_dead_test_namespaces(temp: &std::path::Path) {
    if sourcetrait_process::process(process::id()).is_none() {
        return;
    }
    let Ok(entries) = fs::read_dir(temp) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(pid) = name
            .strip_prefix(INPROC_PREFIX)
            .and_then(|p| p.parse::<u32>().ok())
        else {
            continue;
        };
        if pid == process::id() || sourcetrait_process::process(pid).is_some() {
            continue;
        }
        let _ = fs::remove_dir_all(entry.path());
    }
}

/// This test binary's own root under the temporary directory.
fn test_root() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("{INPROC_PREFIX}{}", process::id()))
}

/// The test namespace's config, its homes under the binary's root.
fn namespace_config(root: &std::path::Path) -> Config {
    Config {
        id: "test".to_string(),
        namespace: "default".to_string(),
        work_dir: root.join("work"),
        remotes_file: root.join(".grammar").join(REMOTES_FILE),
        ..Config::default()
    }
}

/// Point the homes at the binary's root and start the engine on them, once.
fn ensure_test_env() {
    TEST_ENV.call_once(|| {
        sweep_dead_test_namespaces(&std::env::temp_dir());
        let root = test_root();
        let _ = fs::create_dir_all(root.join("data"));
        let _ = fs::create_dir_all(root.join("cache"));
        let _ = fs::create_dir_all(root.join("tmp"));
        let _ = fs::create_dir_all(root.join("run"));
        unsafe {
            std::env::set_var("XDG_DATA_HOME", root.join("data"));
            std::env::set_var("XDG_CACHE_HOME", root.join("cache"));
            std::env::set_var("UENV_USR_DATA", root.join("data"));
            std::env::set_var("UENV_USR_CACHE", root.join("cache"));
            std::env::set_var("UENV_USR_TMP", root.join("tmp"));
            std::env::set_var("UENV_USR_TRANSIENT", root.join("run"));
        }
        init_engine(&namespace_config(&root)).expect("the engine starts for the tests");
    });
}

/// The test namespace's config, its environment and engine made first.
fn test_config() -> Arc<Config> {
    ensure_test_env();
    Arc::new(namespace_config(&test_root()))
}

/// An in-process handle to the grammar tool surface.
pub struct TestServer {
    rt: tokio::runtime::Runtime,
    nush: NuSh,
}

impl Default for TestServer {
    fn default() -> Self {
        Self::new()
    }
}

impl TestServer {
    pub fn new() -> Self {
        let config = test_config();
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_stack_size(EVAL_STACK_SIZE)
            .build()
            .expect("tokio runtime");
        rt.block_on(ensure_substrate()).expect("ensure_substrate");
        let controller = rt
            .block_on(McpController::start(config.clone(), false, None))
            .expect("mcp's controller starts its systems");
        let nush = NuSh::new(controller, &config.deny);
        Self { rt, nush }
    }

    /// A tool's envelope, read from its NUON text.
    fn envelope(r: Result<mcp::CallToolResult, mcp::ErrorData>) -> NuVal {
        let ctr = r.unwrap_or_else(|e| panic!("unexpected rmcp-boundary error: {e:?}"));
        match ctr.content.first() {
            Some(mcp::ContentBlock::Text(text)) => nuon(&text.text),
            other => panic!("a result without its envelope text: {other:?}"),
        }
    }

    /// `execute` on a definition and its data's NUON text.
    pub fn execute(
        &self,
        definition: &str,
        data: &str,
    ) -> NuVal {
        self.execute_timeout(definition, data, None)
    }

    /// `execute` with a timeout, a Nu duration's text.
    pub fn execute_timeout(
        &self,
        definition: &str,
        data: &str,
        timeout: Option<&str>,
    ) -> NuVal {
        self.execute_on(LOCAL_HOST, definition, data, timeout)
    }

    /// `execute` on a host alias.
    pub fn execute_on(
        &self,
        host: &str,
        definition: &str,
        data: &str,
        timeout: Option<&str>,
    ) -> NuVal {
        let p = ExecuteParams {
            host: host.to_string(),
            definition: definition.to_string(),
            data: data.to_string(),
            timeout: timeout.map(str::to_string),
        };
        Self::envelope(self.rt.block_on(self.nush.execute(mcp::Parameters(p))))
    }

    /// `rerun` of an earlier `execute` call with fresh data.
    pub fn rerun(
        &self,
        nonce: &str,
        data: &str,
    ) -> NuVal {
        self.rerun_timeout(nonce, data, None)
    }

    /// `rerun` with a timeout, a Nu duration's text.
    pub fn rerun_timeout(
        &self,
        nonce: &str,
        data: &str,
        timeout: Option<&str>,
    ) -> NuVal {
        self.rerun_on(LOCAL_HOST, nonce, data, timeout)
    }

    /// `rerun` on a host alias.
    pub fn rerun_on(
        &self,
        host: &str,
        nonce: &str,
        data: &str,
        timeout: Option<&str>,
    ) -> NuVal {
        let p = RerunParams {
            host: host.to_string(),
            nonce: nonce.to_string(),
            data: data.to_string(),
            timeout: timeout.map(str::to_string),
        };
        Self::envelope(self.rt.block_on(self.nush.rerun(mcp::Parameters(p))))
    }

    /// `call` of a committed function on its data's NUON text.
    pub fn call(
        &self,
        namepath: &str,
        data: &str,
    ) -> NuVal {
        self.call_timeout(namepath, data, None)
    }

    /// `call` with a timeout, a Nu duration's text.
    pub fn call_timeout(
        &self,
        namepath: &str,
        data: &str,
        timeout: Option<&str>,
    ) -> NuVal {
        self.call_on(LOCAL_HOST, namepath, data, timeout)
    }

    /// `call` on a host alias.
    pub fn call_on(
        &self,
        host: &str,
        namepath: &str,
        data: &str,
        timeout: Option<&str>,
    ) -> NuVal {
        let p = CallParams {
            host: host.to_string(),
            namepath: namepath.to_string(),
            data: data.to_string(),
            timeout: timeout.map(str::to_string),
        };
        Self::envelope(self.rt.block_on(self.nush.call(mcp::Parameters(p))))
    }

    /// A nonce minted as the interface call would mint one.
    fn interface_nonce<P: ser::Serialize>(&self, p: &P) -> datum::Nonce {
        self.nush.mint(INTERFACE_TOOL, p).to_nonce()
    }

    /// One `interface` command, through the tool's parse and dispatch.
    pub fn interface(&self, command: &str) -> NuVal {
        self.interface_on(LOCAL_HOST, command)
    }

    /// One `interface` command on a host alias.
    pub fn interface_on(
        &self,
        host: &str,
        command: &str,
    ) -> NuVal {
        let p = InterfaceParams {
            host: host.to_string(),
            command: command.to_string(),
        };
        Self::envelope(self.rt.block_on(self.nush.interface(mcp::Parameters(p))))
    }

    pub fn commit(&self, namepath: &str) -> NuVal {
        let p = CommitParams {
            namepath: namepath.to_string(),
        };
        let nonce = self.interface_nonce(&p);
        Self::envelope(self.rt.block_on(self.nush.equip_commit(nonce, &Route::Local, p)))
    }

    /// An `interface equip` action on a source; an unknown one goes to the parse.
    pub fn equip(&self, action: &str, source_dir: &str) -> NuVal {
        let p = EquipParams {
            source: source_dir.to_string(),
        };
        let nonce = self.interface_nonce(&p);
        let result = match action {
            "new" => self.rt.block_on(self.nush.equip_new(nonce, &Route::Local, p)),
            "install" => self.rt.block_on(self.nush.equip_install(nonce, &Route::Local, p)),
            "check" => self.rt.block_on(self.nush.equip_check(nonce, &Route::Local, p)),
            "uninstall" => self.rt.block_on(self.nush.equip_uninstall(nonce, &Route::Local, p)),
            other => return self.interface(&format!("{INTERFACE} equip {other} '{source_dir}'")),
        };
        Self::envelope(result)
    }

    /// `interface equip new` at `root`'s source for `name`; its rig source.
    pub fn new_rig(
        &self,
        root: &std::path::Path,
        name: &str,
    ) -> std::path::PathBuf {
        let source = equip_source(root, name);
        let env = self.equip("new", &source.to_string_lossy());
        assert!(!has_error(&env), "equip new for {name} failed: {env}");
        source.join("rig")
    }

    pub fn scaffold(&self, namepaths: &[&str]) -> NuVal {
        let p = NewParams {
            namepaths: namepaths.iter().map(|s| s.to_string()).collect(),
        };
        let nonce = self.interface_nonce(&p);
        Self::envelope(self.rt.block_on(self.nush.equip_scaffold(nonce, &Route::Local, p)))
    }

    /// `interface help` on a namepath or a pattern.
    pub fn inspect(&self, namepath: &str) -> NuVal {
        let p = InspectParams {
            namepath: namepath.to_string(),
        };
        let nonce = self.interface_nonce(&p);
        Self::envelope(self.rt.block_on(self.nush.help(nonce, &Route::Local, p)))
    }

    /// `interface info`, the current view's.
    pub fn info(&self) -> NuVal {
        let nonce = self.interface_nonce(&"info");
        Self::envelope(self.rt.block_on(self.nush.info(nonce, &Route::Local)))
    }

    /// `interface purview list`.
    pub fn purviews(&self) -> NuVal {
        let nonce = self.interface_nonce(&"purview list");
        Self::envelope(self.rt.block_on(self.nush.purview_list(nonce, &Route::Local)))
    }

    /// Set a purview's namepath patterns; an EMPTY `namepaths` deletes it.
    pub fn purview_configure(
        &self,
        purview: &str,
        namepaths: &[&str],
    ) -> NuVal {
        let p = PurviewConfigureParams {
            purview: purview.to_string(),
            namepaths: namepaths.iter().map(|s| s.to_string()).collect(),
        };
        let nonce = self.interface_nonce(&p);
        Self::envelope(self.rt.block_on(self.nush.purview_configure(nonce, &Route::Local, p)))
    }

    pub fn purview_extend(
        &self,
        purviews: &[&str],
    ) -> NuVal {
        let p = PurviewExtendParams {
            purviews: purviews.iter().map(|s| s.to_string()).collect(),
        };
        let nonce = self.interface_nonce(&p);
        Self::envelope(self.rt.block_on(self.nush.purview_extend(nonce, &Route::Local, p)))
    }

    /// `interface purview set`.
    pub fn purview(
        &self,
        purviews: &[&str],
    ) -> NuVal {
        let p = PurviewParams {
            purviews: purviews.iter().map(|s| (*s).to_string()).collect(),
        };
        let nonce = self.interface_nonce(&p);
        Self::envelope(self.rt.block_on(self.nush.purview_set(nonce, &Route::Local, p)))
    }

    /// Where this namespace's purview table lands.
    pub fn purviews_path(&self) -> std::path::PathBuf {
        crate::purviews_path()
    }

    /// `interface process list`.
    pub fn processes(&self) -> NuVal {
        let nonce = self.interface_nonce(&"process list");
        Self::envelope(self.rt.block_on(self.nush.process_list(nonce, &Route::Local)))
    }

    /// `interface process kill`; its result is null on success.
    pub fn kill(&self, nonce: &str) -> NuVal {
        let p = KillParams {
            nonce: nonce.to_string(),
        };
        let call_nonce = self.interface_nonce(&p);
        Self::envelope(self.rt.block_on(self.nush.process_kill(call_nonce, &Route::Local, p)))
    }

    /// `interface channel ready`; its result is null.
    pub fn channel_verified(&self) -> NuVal {
        let nonce = self.interface_nonce(&"channel ready");
        Self::envelope(self.rt.block_on(self.nush.controller.face.channel_ready(nonce)))
    }

    /// A PARTIAL update; pass `None` for anything that should not move.
    pub fn config_channel(
        &self,
        warn_window_secs: Option<u64>,
        warn_rate: Option<u32>,
        error_window_secs: Option<u64>,
        error_rate: Option<u32>,
    ) -> NuVal {
        let p = ConfigChannelParams {
            spam_warn_window_secs: warn_window_secs,
            spam_warn_rate: warn_rate,
            spam_error_window_secs: error_window_secs,
            spam_error_rate: error_rate,
        };
        let nonce = self.interface_nonce(&p);
        Self::envelope(self.rt.block_on(self.nush.controller.face.channel_config(nonce, p)))
    }

    /// `interface channel close`; closing a closed channel succeeds.
    pub fn channel_close(&self) -> NuVal {
        let nonce = self.interface_nonce(&"channel close");
        Self::envelope(self.rt.block_on(self.nush.controller.face.channel_close(nonce)))
    }

    /// Each routed tool as `tools/list` carries it.
    pub fn tools(&self) -> Vec<json::Value> {
        self.nush
            .tool_router
            .list_all()
            .into_iter()
            .map(|tool| json::to_value(&tool).expect("a tool serializes"))
            .collect()
    }

    /// The server's instructions, as `initialize` carries them.
    pub fn instructions(&self) -> String {
        mcp::ServerHandler::get_info(&self.nush)
            .instructions
            .unwrap_or_default()
    }

    /// The in-process equip git repo dir.
    pub fn equip_root(&self) -> std::path::PathBuf {
        crate::equip_root()
    }

    /// The per-call log dir of an `execute` call's nonce.
    pub fn execute_log_dir(&self, nonce: &str) -> std::path::PathBuf {
        crate::definition_file(nonce)
            .parent()
            .expect("a definition path always has a parent dir")
            .to_path_buf()
    }

    /// The committed rig of an equip by its compound `author/name`.
    pub fn rig_dir(&self, name: &str) -> std::path::PathBuf {
        crate::rig_dir(name)
    }

    /// The committed lib of an equip by its compound `author/name`.
    pub fn lib_dir(&self, name: &str) -> std::path::PathBuf {
        crate::lib_dir(name)
    }

    /// A rig's committed index, decoded and carried as a NuVal.
    pub fn rig_index(&self, name: &str) -> NuVal {
        let path = crate::rig_meta_path(name);
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read index {}: {e}", path.display()));
        let index = crate::index_from_nuon(&text)
            .unwrap_or_else(|e| panic!("decode index {}: {e}", path.display()));
        nuval::to_nuval(&index).expect("the index converts")
    }
}

/// mcp's controller on the test namespace as the serve path starts it.
pub struct SupervisedSystem {
    rt: tokio::runtime::Runtime,
    controller: Arc<McpController>,
}

impl SupervisedSystem {
    /// The namespace's lock taken, then the controller started, supervising.
    pub fn start() -> Self {
        let config = test_config();
        let lock = lock_namespace(&config).expect("the test namespace's lock");
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_stack_size(EVAL_STACK_SIZE)
            .build()
            .expect("tokio runtime");
        rt.block_on(ensure_substrate()).expect("ensure_substrate");
        let controller = rt
            .block_on(McpController::start(config, true, Some(lock)))
            .expect("mcp's controller starts its systems");
        Self { rt, controller }
    }

    /// Whether the engine answers a process list through mcp's controller.
    pub fn engine_answers(&self) -> bool {
        matches!(
            self.rt.block_on(self.controller.control(EngineCommand::Processes)),
            Ok(ControlAnswer::Processes(_)),
        )
    }

    /// Whether the namespace's lock is held, tried without blocking.
    pub fn host_lock_held(&self) -> bool {
        let lock = self.controller.config.data_home().join(LOCK_FILE);
        let Ok(file) = fs::File::open(lock) else {
            return false;
        };
        nix::fcntl::Flock::lock(file, nix::fcntl::FlockArg::LockExclusiveNonblock).is_err()
    }

    /// The systems stopped in order.
    pub fn shutdown(&self) {
        self.rt.block_on(self.controller.shutdown());
    }
}

/// Drop every config pin.
pub fn clear_config_pins() {
    clear_pins();
}

/// A NUON text as a NuVal, panicking on text that isn't NUON.
pub fn nuon(text: &str) -> NuVal {
    NuVal::from_nuon(text).unwrap_or_else(|e| panic!("NUON `{text}`: {e}"))
}

/// True when the envelope is the failure shape, `{nonce, error}`.
pub fn has_error(env: &NuVal) -> bool {
    env.get("error").is_some()
}

/// The envelope's error rows, none on a success.
fn error_rows(env: &NuVal) -> &[NuVal] {
    env.get("error").and_then(NuVal::as_list).unwrap_or_default()
}

/// The first error row's `kind`, if any.
pub fn error_kind(env: &NuVal) -> Option<&str> {
    error_rows(env).first()?.get("kind")?.as_str()
}

/// Every error row's `kind`.
pub fn error_kinds(env: &NuVal) -> Vec<String> {
    error_rows(env)
        .iter()
        .filter_map(|row| row.get("kind").and_then(NuVal::as_str))
        .map(str::to_string)
        .collect()
}

pub fn has_kind(env: &NuVal, kind: &str) -> bool {
    error_kinds(env).iter().any(|k| k == kind)
}

/// Every error row's `message`.
pub fn error_messages(env: &NuVal) -> Vec<String> {
    error_rows(env)
        .iter()
        .filter_map(|row| row.get("message").and_then(NuVal::as_str))
        .map(str::to_string)
        .collect()
}

/// The whole error table as its raw NUON.
pub fn error_text(env: &NuVal) -> String {
    env.get("error").map(|e| e.to_string()).unwrap_or_default()
}

/// One rig's slice of an `interface info` signature block.
pub fn rig_block(
    signatures: &str,
    leaf: &str,
) -> String {
    let head = format!(" {leaf}");
    let mut out: Vec<&str> = Vec::new();
    for line in signatures.lines() {
        if out.is_empty() {
            if line == head || line.starts_with(&format!("{head} ")) {
                out.push(line);
            }
            continue;
        }
        let indent = line.len() - line.trim_start_matches(' ').len();
        if indent <= 1 {
            break;
        }
        out.push(line);
    }
    assert!(
        !out.is_empty(),
        "rig `{leaf}` is absent from the block:\n{signatures}",
    );
    format!("{}\n", out.join("\n"))
}

/// An equip's source directory under `root`: `root/equip/<name>`.
pub fn equip_source(
    root: &std::path::Path,
    name: &str,
) -> std::path::PathBuf {
    root.join("equip").join(name)
}

pub fn write_source(dir: &std::path::Path, rel: &str, contents: &str) {
    let target = dir.join(rel);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).expect("mkdir");
    }
    fs::write(&target, contents).expect("write source");
}

pub fn write_tree(dir: &std::path::Path, files: &[(&str, &str)]) {
    for (rel, contents) in files {
        write_source(dir, rel, contents);
    }
}

/// A call's `main` taking a record of `data_fields`, answering `result_fields`.
pub fn valid_function_source(data_fields: &str, result_fields: &str, body: &str) -> String {
    format!(
        "export def main [data: record<{data_fields}>]: nothing -> record<{result_fields}> {{\n{body}\n}}\n",
    )
}

/// Lint a definition as `execute`'s contract does, each diagnostic a NuVal.
pub fn lint_definition(definition: &str) -> Vec<NuVal> {
    ensure_test_env();
    let engine = ParseEngine::new_full();
    let mut ws = nu::StateWorkingSet::new(engine.engine_state());
    let block = nu::parse(&mut ws, Some(LINT_FILE), definition.as_bytes(), false);
    let start = file_start(&ws, LINT_FILE);
    crate::lint_block(&block, &ws, definition, start)
        .iter()
        .map(|d| nuval::to_nuval(d).expect("a diagnostic converts"))
        .collect()
}

/// The name `lint_definition` parses a definition under.
const LINT_FILE: &str = "definition.nu";

/// The execute contract's verdict on a definition: its summary, or its rows.
pub fn analyze_definition(definition: &str) -> Result<NuVal, Vec<NuVal>> {
    ensure_test_env();
    let engine = ParseEngine::new_full();
    match crate::analyze_definition(&engine, definition, true) {
        Ok(def) => Ok(NuVal::Record(vec![
            ("lane".to_string(), NuVal::from(def.lane.as_str())),
            ("data_type".to_string(), NuVal::from(typedef_text(&def.data_type))),
            ("result_type".to_string(), NuVal::from(typedef_text(&def.result_type))),
            ("takes_timeout".to_string(), NuVal::from(def.takes_timeout)),
        ])),
        Err(rows) => Err(rows
            .iter()
            .map(|d| nuval::to_nuval(d).expect("a diagnostic converts"))
            .collect()),
    }
}

/// Are two `current()` handles off ONE `LintEngine` the same instance?
pub fn lint_engine_reuses_until_the_registry_moves() -> bool {
    ensure_test_env();
    let engine = LintEngine::new();
    let first = engine.current();
    let second = engine.current();
    Arc::ptr_eq(&first, &second)
}

/// True if `src` parses clean on a full-shell ParseEngine.
pub fn parses_clean(src: &str) -> bool {
    ensure_test_env();
    let engine = ParseEngine::new_full();
    let mut ws = nu::StateWorkingSet::new(engine.engine_state());
    let _ = nu::parse(&mut ws, Some("golden.nu"), src.as_bytes(), false);
    ws.parse_errors.is_empty()
}

/// The synthesized execute() source for a definition and its literals.
pub fn build_execute_source(
    definition: &str,
    data_literal: &str,
    timeout_literal: Option<&str>,
) -> String {
    crate::build_execute_source(definition, data_literal, timeout_literal)
}

/// The synthesized call() source for a function and its literals.
pub fn build_call_source(
    equip_root: &std::path::Path,
    rig: &str,
    module_path: &str,
    name: &str,
    data_literal: &str,
    timeout_literal: Option<&str>,
) -> Result<String, String> {
    crate::build_call_source(equip_root, rig, module_path, name, data_literal, timeout_literal)
}
