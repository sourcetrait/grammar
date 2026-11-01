use crate::*;

/// The builtins: equips central to the engine, each called by its bare name.
pub const BUILTINS: [(&str, &str); 2] = [
    ("gram", "sourcetrait"),
    ("grammar", "sourcetrait"),
];

/// The `<author>/<name>` a builtin's bare name stands for.
pub fn builtin_equip(name: &str) -> Option<String> {
    BUILTINS
        .iter()
        .find(|(builtin, _)| *builtin == name)
        .map(|(builtin, author)| format!("{author}/{builtin}"))
}

/// The bare name an equip goes by when it is a builtin.
pub fn builtin_name(equip: &str) -> Option<&'static str> {
    BUILTINS
        .iter()
        .find(|(name, author)| equip.split_once('/') == Some((*author, *name)))
        .map(|(name, _)| *name)
}

/// What an equip is shown and called by: a builtin's bare name, else itself.
pub fn display_equip(equip: &str) -> &str {
    builtin_name(equip).unwrap_or(equip)
}

/// Where a request runs: this process's engine, or a linked host's.
#[cereal::derived(Data, Copy, Eq)]
pub enum Host {
    Local,
    Remote(datum::Nom),
}

/// An `execute` call: its definition run on its data.
#[cereal::derived(Data)]
pub struct ExecuteRequest {
    pub host: Host,
    pub nonce: datum::Nonce,
    pub definition: String,
    /// The definition's data as NUON text.
    pub data: String,
    /// The call's timeout as a Nu duration's text; the default when absent.
    pub timeout: Option<String>,
}

/// A rerun: an earlier `execute` call's definition run on fresh data.
#[cereal::derived(Data)]
pub struct RerunRequest {
    pub host: Host,
    pub nonce: datum::Nonce,
    /// The earlier call's nonce, whose cached definition runs again.
    pub source: String,
    pub data: String,
    pub timeout: Option<String>,
}

/// A call: a committed rig function run on its data.
#[cereal::derived(Data)]
pub struct CallRequest {
    pub host: Host,
    pub nonce: datum::Nonce,
    pub namepath: String,
    /// The function's data as NUON text.
    pub data: String,
    /// The call's timeout as a Nu duration's text; the default when absent.
    pub timeout: Option<String>,
}

/// A command to the engine itself.
#[cereal::derived(Data)]
pub struct ControlRequest {
    pub host: Host,
    pub command: EngineCommand,
}

/// What the engine is asked of its calls, its health, its equips, its signatures.
#[cereal::derived(Data)]
pub enum EngineCommand {
    Processes,
    Kill { nonce: String },
    Health,
    Teardown,
    EquipNew { source: String },
    EquipInstall { source: String },
    EquipCheck { source: String },
    EquipUninstall { source: String },
    EquipScaffold { namepaths: Vec<String> },
    EquipCommit { namepath: String },
    Signatures { patterns: Vec<String> },
    Help { namepath: String },
}

/// A call's answer: its nonce, and its value or the engine's refusal.
#[cereal::derived(Data)]
pub struct ValueAnswer {
    pub nonce: datum::Nonce,
    pub result: EngineResult<NuVal>,
}

/// A command's answer.
#[cereal::derived(Data)]
pub enum ControlAnswer {
    Done,
    Processes(Vec<ProcessRow>),
    Health(EngineHealth),
    Created(Vec<String>),
    Installed { equip: String, result: CommitResult },
    Checked(CheckSummary),
    /// The equip uninstalled, `None` when none was installed.
    Uninstalled { equip: Option<String> },
    Committed(CommitResult),
    Signatures(String),
    Help(InspectDoc),
}

/// One call in flight.
#[cereal::derived(Data)]
pub struct ProcessRow {
    pub nonce: String,
    pub tool: String,
    pub started_at: u64,
    pub args: NuVal,
    pub source_nonce: Option<String>,
    pub path: Option<String>,
}

/// The lane an eval runs on: the stateless pool, or the stateful engine.
#[cereal::derived(Data, Copy, Eq)]
pub enum Lane {
    Stateless,
    Stateful,
}

impl Lane {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stateless => "stateless",
            Self::Stateful => "stateful",
        }
    }
}

/// A cancelled eval whose thread has not ended, watched for a hang.
#[cereal::derived(Data)]
pub struct HungEval {
    pub nonce: String,
    pub tool: String,
    pub lane: Lane,
    pub started_at: u64,
    pub cancelled_at: u64,
}

/// The engine's health as its supervisor samples it.
#[cereal::derived(Data)]
pub struct EngineHealth {
    pub hung: Vec<HungEval>,
    pub permits_held: u64,
    pub permits_cap: u64,
    pub jobs: u64,
}

/// An equip check's verdict: `ok` when no row is an error.
#[cereal::derived(Data)]
pub struct CheckSummary {
    pub ok: bool,
    pub errors: Vec<Diagnostic>,
    pub warnings: Vec<Diagnostic>,
}

/// A commit's changed paths, grouped by kind, and the validator's warning rows.
#[cereal::derived(Data)]
pub struct CommitResult {
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub removed: Vec<String>,
    pub warnings: Vec<Diagnostic>,
}

impl CommitResult {
    /// Whether the commit changed no path.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.removed.is_empty()
    }
}

/// A rig's documentation; `srcdir` is the COMMITTED CANONICAL directory.
#[cereal::derived(Data)]
pub struct RigDoc {
    pub srcdir: String,
    pub summary: String,
    pub details: String,
}

/// A module's documentation; `src` is its committed `mod.nu`.
#[cereal::derived(Data)]
pub struct ModuleDoc {
    pub src: String,
    pub summary: String,
    pub details: String,
}

/// A call's documentation; `src` is the committed `mod.nu` holding its `main`.
#[cereal::derived(Data)]
pub struct CallDoc {
    pub src: String,
    pub signature: String,
    pub details: String,
}

/// A pattern's `interface help`: the `interface info` block at the pattern.
#[cereal::derived(Data)]
pub struct SignaturesDoc {
    pub signatures: String,
}

/// What `interface help` returns - one exact namepath, or a pattern's block.
#[cereal::derived(Data)]
#[serde(untagged)]
pub enum InspectDoc {
    Rig(RigDoc),
    Module(ModuleDoc),
    Call(CallDoc),
    Signatures(SignaturesDoc),
}

#[cereal::derived(Data, Copy, Eq)]
#[derive(Default)]
pub enum Severity {
    #[default]
    Error,
    Warning,
}

/// A diagnostic's location; `path` is equip-repo-relative, or null in a body.
#[cereal::derived(Data)]
pub struct Source {
    pub path: Option<String>,
    pub position: [u64; 2],
}

/// A byte offset's 1-based line and column in its source text.
pub fn span_to_line_col(source: &str, byte_offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, b) in source.bytes().enumerate() {
        if i >= byte_offset {
            return (line, col);
        }
        if b == b'\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// The most rows one lint reports.
pub const LINT_VIOLATION_CAP: usize = 3;

/// One agent-facing diagnostic row.
#[cereal::derived(Data)]
pub struct Diagnostic {
    pub kind: String,
    #[serde(skip)]
    pub severity: Severity,
    pub source: Option<Source>,
    pub message: String,
}

impl Diagnostic {
    pub fn error(
        kind: impl Into<String>,
        source: Option<Source>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            severity: Severity::Error,
            source,
            message: message.into(),
        }
    }

    pub fn warning(
        kind: impl Into<String>,
        source: Option<Source>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            severity: Severity::Warning,
            source,
            message: message.into(),
        }
    }

    pub fn bucket(diagnostics: Vec<Diagnostic>) -> (Vec<Diagnostic>, Vec<Diagnostic>) {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        for d in diagnostics {
            match d.severity {
                Severity::Error => errors.push(d),
                Severity::Warning => warnings.push(d),
            }
        }
        (errors, warnings)
    }
}

/// What the engine failed at, and why.
#[cereal::derived(Data)]
pub enum EngineError {
    /// An eval's failure, in nushell's words.
    Eval { reason: String },
    /// A module-resolution cycle, the files it ran through.
    CircularImport { files: String },
    /// The engine's start, and why it could not.
    Init { reason: String },
    EquipNotRegistered {
        equip: String,
    },
    EquipAlreadyRegistered {
        equip: String,
    },
    EquipInvalidName {
        equip: String,
        reason: String,
    },
    EquipNameDenied {
        equip: String,
    },
    EquipInvalidSource {
        source_dir: String,
        reason: String,
    },
    EquipSourceMissing {
        path: String,
    },
    EquipSourcePathMismatch {
        equip: String,
        passed: String,
        registered: String,
    },
    EquipNoKinds {
        source_dir: String,
    },
    EquipNoRig {
        equip: String,
    },
    EquipViolations {
        diagnostics: Vec<Diagnostic>,
    },
    RigInvalidName {
        name: String,
        reason: String,
    },
    RigInvalidModulePath {
        module_path: String,
        reason: String,
    },
    FunctionNotDefined {
        rig: String,
        module_path: String,
        name: String,
    },
    NamepathInvalid {
        namepath: String,
        reason: String,
    },
    DefinitionViolations {
        diagnostics: Vec<Diagnostic>,
    },
    DataInvalid {
        reason: String,
    },
    ResultInvalid {
        reason: String,
    },
    TimeoutInvalid {
        reason: String,
    },
    RerunInvalidNonce {
        nonce: String,
        reason: String,
    },
    RerunBodyMissing {
        nonce: String,
    },
    Dispatch {
        reason: String,
    },
    Timeout {
        timeout_ms: u64,
    },
    /// A request for a host whose engine this process doesn't serve.
    HostUnserved {
        host: String,
    },
    Internal {
        phase: String,
        reason: String,
    },
}

/// The engine's result.
pub type EngineResult<T> = Result<T, EngineError>;

impl EngineError {
    /// The kind a single-condition error reports under.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Eval { .. } => "thread::returned_error",
            Self::CircularImport { .. } => "module::circular_import",
            Self::Init { .. } | Self::Internal { .. } => "internal",
            Self::EquipNotRegistered { .. } => "equip::not_registered",
            Self::EquipAlreadyRegistered { .. } => "equip::already_registered",
            Self::EquipInvalidName { .. } => "equip::invalid_name",
            Self::EquipNameDenied { .. } => "equip::name_denied",
            Self::EquipInvalidSource { .. } => "equip::invalid_source",
            Self::EquipSourceMissing { .. } => "equip::source_missing",
            Self::EquipSourcePathMismatch { .. } => "equip::source_path_mismatch",
            Self::EquipNoKinds { .. } => "equip::no_kinds",
            Self::EquipNoRig { .. } => "equip::no_rig",
            Self::RigInvalidName { .. } => "rig::invalid_name",
            Self::RigInvalidModulePath { .. } => "rig::invalid_module_path",
            Self::FunctionNotDefined { .. } => "function::not_defined",
            Self::NamepathInvalid { .. } => "namepath::invalid",
            Self::DataInvalid { .. } => "data::invalid",
            Self::ResultInvalid { .. } => "result::invalid",
            Self::TimeoutInvalid { .. } => "timeout::invalid",
            Self::RerunInvalidNonce { .. } => "rerun::invalid_nonce",
            Self::RerunBodyMissing { .. } => "rerun::body_missing",
            Self::Dispatch { .. } => "thread::dispatch",
            Self::Timeout { .. } => "thread::timeout",
            Self::HostUnserved { .. } => "host::unserved",
            Self::EquipViolations { .. } | Self::DefinitionViolations { .. } => {
                unreachable!("a violations error renders through its rows, not a kind")
            }
        }
    }

    /// The message a single-condition error carries.
    pub fn message(&self) -> String {
        match self {
            Self::Eval { reason } | Self::Init { reason } => reason.clone(),
            Self::CircularImport { files } => files.clone(),
            Self::Internal { phase, reason } => format!("internal error [{phase}]: {reason}"),
            Self::EquipNotRegistered { equip } => {
                format!("equip `{}` is not registered", display_equip(equip))
            }
            Self::EquipAlreadyRegistered { equip } => {
                format!("equip `{}` is already registered", display_equip(equip))
            }
            Self::EquipInvalidName { equip, reason } => {
                format!("invalid equip name `{equip}`: {reason}")
            }
            Self::EquipNameDenied { equip } => {
                format!("equip name `{equip}` is reserved and cannot be used")
            }
            Self::EquipInvalidSource { source_dir, reason } => {
                format!("invalid equip source `{source_dir}`: {reason}")
            }
            Self::EquipSourceMissing { path } => {
                format!("equip source path is missing or not a directory: {path}")
            }
            Self::EquipSourcePathMismatch {
                equip,
                passed,
                registered,
            } => format!(
                "source_dir mismatch for `{}`: passed `{passed}`, registered `{registered}`",
                display_equip(equip),
            ),
            Self::EquipNoKinds { source_dir } => {
                format!("equip source `{source_dir}` holds neither a lib/ nor a rig/")
            }
            Self::EquipNoRig { equip } => format!("equip `{}` has no rig", display_equip(equip)),
            Self::RigInvalidName { name, reason } => {
                format!("invalid name `{name}`: {reason}")
            }
            Self::RigInvalidModulePath {
                module_path,
                reason,
            } => format!("invalid module path `{module_path}`: {reason}"),
            Self::FunctionNotDefined {
                rig,
                module_path,
                name,
            } => format!(
                "function `{name}` is not defined in `{}:{module_path}`",
                display_equip(rig),
            ),
            Self::NamepathInvalid { namepath, reason } => {
                format!("invalid namepath `{namepath}`: {reason}")
            }
            Self::DataInvalid { reason }
            | Self::ResultInvalid { reason }
            | Self::TimeoutInvalid { reason } => reason.clone(),
            Self::RerunInvalidNonce { nonce, reason } => {
                format!("invalid nonce `{nonce}`: {reason}")
            }
            Self::RerunBodyMissing { nonce } => {
                format!("no cached definition for the nonce `{nonce}`; rerun takes an execute call's nonce")
            }
            Self::Dispatch { reason } => format!("eval dispatch failed: {reason}"),
            Self::Timeout { timeout_ms } => format!("eval timed out after {timeout_ms} ms"),
            Self::HostUnserved { host } => format!("no engine is served on the host `{host}`"),
            Self::EquipViolations { .. } | Self::DefinitionViolations { .. } => {
                unreachable!("a violations error renders through its rows, not a message")
            }
        }
    }

    /// The error rows a failure reports: a violations error's own, else one.
    pub fn rows(self) -> Vec<Diagnostic> {
        match self {
            Self::EquipViolations { diagnostics } | Self::DefinitionViolations { diagnostics } => {
                Diagnostic::bucket(diagnostics).0
            }
            single => vec![Diagnostic::error(single.kind(), None, single.message())],
        }
    }
}

impl From<String> for EngineError {
    fn from(reason: String) -> Self {
        Self::Eval { reason }
    }
}

impl From<io::Error> for EngineError {
    fn from(e: io::Error) -> Self {
        Self::Internal {
            phase: "io".to_string(),
            reason: e.to_string(),
        }
    }
}

impl fmt::Display for EngineError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Self::EquipViolations { diagnostics } | Self::DefinitionViolations { diagnostics } => {
                let messages: Vec<&str> =
                    diagnostics.iter().map(|d| d.message.as_str()).collect();
                f.write_str(&messages.join("; "))
            }
            single => f.write_str(&single.message()),
        }
    }
}

impl std::error::Error for EngineError {}

#[cereal::derived(Data)]
pub enum ToEngineSys {
    Execute(ExecuteRequest),
    Rerun(RerunRequest),
    Call(CallRequest),
    Control(ControlRequest),
}

#[cereal::derived(Data)]
pub enum FromEngineSys {
    Value(ValueAnswer),
    Control(EngineResult<ControlAnswer>),
}

impl From<ExecuteRequest> for ToEngineSys {
    fn from(v: ExecuteRequest) -> Self { Self::Execute(v) }
}

impl From<RerunRequest> for ToEngineSys {
    fn from(v: RerunRequest) -> Self { Self::Rerun(v) }
}

impl From<CallRequest> for ToEngineSys {
    fn from(v: CallRequest) -> Self { Self::Call(v) }
}

impl From<ControlRequest> for ToEngineSys {
    fn from(v: ControlRequest) -> Self { Self::Control(v) }
}

impl TryFrom<FromEngineSys> for ValueAnswer {
    type Error = subsys::SubsysError;

    fn try_from(v: FromEngineSys) -> subsys::SubsysResult<Self> {
        match v {
            FromEngineSys::Value(answer) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

impl TryFrom<FromEngineSys> for EngineResult<ControlAnswer> {
    type Error = subsys::SubsysError;

    fn try_from(v: FromEngineSys) -> subsys::SubsysResult<Self> {
        match v {
            FromEngineSys::Control(answer) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}
