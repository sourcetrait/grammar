//! The worker's failures, and the exit codes grammar-ssh reports them by.
use crate::*;

/// A worker operation's result.
pub type GrammarSshResult<T> = Result<T, GrammarSshError>;

/// What the worker refuses or fails at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrammarSshError {
    /// This process already started its one runtime.
    AlreadyStarted,
    /// An id or a namespace that isn't a name.
    NameInvalid {
        /// Which of the two it was given as.
        field: &'static str,
        name: String,
    },
    /// A workdir that isn't an absolute directory.
    WorkdirInvalid { path: PathBuf, reason: String },
    /// The workdir's config file, unreadable or not the worker's settings.
    ConfigInvalid { path: PathBuf, reason: String },
    /// A home the worker needs and cannot resolve to a directory.
    HomeUnavailable {
        /// The home, by its role.
        home: &'static str,
        reason: String,
    },
    /// A scratch directory the worker cannot make or remove.
    ScratchUnavailable { path: PathBuf, reason: String },
    /// The namespace lock another process holds, and that holder's own record.
    NamespaceHeld { path: PathBuf, holder: String },
    /// A namespace lock that cannot be taken or written for a reason of its own.
    LockUnavailable { path: PathBuf, reason: String },
    /// The engine's start, its parameters or its substrate, failing.
    EngineInit { reason: String },
    /// The worker's task runtime, its threads, that can't be made.
    TasksUnavailable { reason: String },
    /// A session that couldn't open: its hello refused, or its stream broken.
    Session { reason: String },
}

/// Payload-less mirror of `GrammarSshError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrammarSshErrorKind {
    AlreadyStarted,
    NameInvalid,
    WorkdirInvalid,
    ConfigInvalid,
    HomeUnavailable,
    ScratchUnavailable,
    NamespaceHeld,
    LockUnavailable,
    EngineInit,
    TasksUnavailable,
    Session,
}

impl GrammarSshErrorKind {
    /// The kind's name, as a report or a test names it.
    pub fn name(&self) -> &'static str {
        match self {
            Self::AlreadyStarted => "ssh::already_started",
            Self::NameInvalid => "instance::name_invalid",
            Self::WorkdirInvalid => "instance::workdir_invalid",
            Self::ConfigInvalid => "instance::config_invalid",
            Self::HomeUnavailable => "instance::home_unavailable",
            Self::ScratchUnavailable => "scratch::unavailable",
            Self::NamespaceHeld => "runtime::namespace_held",
            Self::LockUnavailable => "runtime::lock_unavailable",
            Self::EngineInit => "runtime::engine_init",
            Self::TasksUnavailable => "runtime::tasks_unavailable",
            Self::Session => "session::failed",
        }
    }
}

impl GrammarSshError {
    /// The failure's kind.
    pub fn kind(&self) -> GrammarSshErrorKind {
        match self {
            Self::AlreadyStarted => GrammarSshErrorKind::AlreadyStarted,
            Self::NameInvalid { .. } => GrammarSshErrorKind::NameInvalid,
            Self::WorkdirInvalid { .. } => GrammarSshErrorKind::WorkdirInvalid,
            Self::ConfigInvalid { .. } => GrammarSshErrorKind::ConfigInvalid,
            Self::HomeUnavailable { .. } => GrammarSshErrorKind::HomeUnavailable,
            Self::ScratchUnavailable { .. } => GrammarSshErrorKind::ScratchUnavailable,
            Self::NamespaceHeld { .. } => GrammarSshErrorKind::NamespaceHeld,
            Self::LockUnavailable { .. } => GrammarSshErrorKind::LockUnavailable,
            Self::EngineInit { .. } => GrammarSshErrorKind::EngineInit,
            Self::TasksUnavailable { .. } => GrammarSshErrorKind::TasksUnavailable,
            Self::Session { .. } => GrammarSshErrorKind::Session,
        }
    }

    /// The exit code grammar-ssh reports the failure by, sysexits' meanings.
    pub fn exit_code(&self) -> u8 {
        match self.kind() {
            GrammarSshErrorKind::AlreadyStarted | GrammarSshErrorKind::EngineInit => 70,
            GrammarSshErrorKind::NameInvalid => 64,
            GrammarSshErrorKind::WorkdirInvalid | GrammarSshErrorKind::ConfigInvalid | GrammarSshErrorKind::HomeUnavailable => 78,
            GrammarSshErrorKind::ScratchUnavailable | GrammarSshErrorKind::LockUnavailable => 73,
            GrammarSshErrorKind::NamespaceHeld => 75,
            GrammarSshErrorKind::TasksUnavailable => 71,
            GrammarSshErrorKind::Session => 76,
        }
    }
}

impl fmt::Display for GrammarSshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyStarted => write!(f, "this process already started its runtime"),
            Self::NameInvalid { field, name } => write!(
                f,
                "the {field} `{name}` is not a name: one to 64 of letters, digits, `.`, `_`, \
                 and `-`, a letter or a digit first",
            ),
            Self::WorkdirInvalid { path, reason } => write!(f, "the workdir {}: {reason}", path.display()),
            Self::ConfigInvalid { path, reason } => write!(f, "the config file {}: {reason}", path.display()),
            Self::HomeUnavailable { home, reason } => write!(f, "the {home} home: {reason}"),
            Self::ScratchUnavailable { path, reason } => {
                write!(f, "scratch under {}: {reason}", path.display())
            }
            Self::NamespaceHeld { path, holder } => {
                write!(f, "the namespace is held, {} naming {holder}", path.display())
            }
            Self::LockUnavailable { path, reason } => {
                write!(f, "the namespace lock {}: {reason}", path.display())
            }
            Self::EngineInit { reason } => write!(f, "the engine's start: {reason}"),
            Self::TasksUnavailable { reason } => write!(f, "the worker's task runtime: {reason}"),
            Self::Session { reason } => write!(f, "the session: {reason}"),
        }
    }
}

impl std::error::Error for GrammarSshError {}

impl From<LockError> for GrammarSshError {
    fn from(e: LockError) -> Self {
        match e {
            LockError::Held { path, holder } => Self::NamespaceHeld { path, holder },
            LockError::Unavailable { path, reason } => Self::LockUnavailable { path, reason },
        }
    }
}
