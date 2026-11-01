use crate::*;

/// The kind of a nushell `JobError`, one per variant.
#[cereal::derived(Data, Copy, Eq, Hash)]
pub enum JobErrorKind {
    NotFound,
    NoneToUnfreeze,
    CannotUnfreeze,
    AlreadyFrozen,
    RecvTimeout,
}

impl JobErrorKind {
    /// The diagnostic code nushell gives the kind, where it gives one.
    pub const fn code(self) -> Option<&'static str> {
        match self {
            Self::NotFound => Some("nu::shell::job::not_found"),
            Self::NoneToUnfreeze => Some("nu::shell::job::none_to_unfreeze"),
            Self::CannotUnfreeze => Some("nu::shell::job::cannot_unfreeze"),
            Self::AlreadyFrozen => Some("nu::shell::job::already_frozen"),
            Self::RecvTimeout => Some("nu::shell::job::recv_timeout"),
        }
    }

    /// The kind's message, the same for every error of the kind.
    pub const fn message(self) -> &'static str {
        match self {
            Self::NotFound => "Job not found",
            Self::NoneToUnfreeze => "No frozen job to unfreeze",
            Self::CannotUnfreeze => "Job is not frozen",
            Self::AlreadyFrozen => "The job is frozen",
            Self::RecvTimeout => "No message was received in the requested time interval",
        }
    }
}

#[cfg(feature = "nushell")]
impl From<&nu::JobError> for JobErrorKind {
    fn from(v: &nu::JobError) -> Self {
        match v {
            nu::JobError::NotFound { .. } => Self::NotFound,
            nu::JobError::NoneToUnfreeze { .. } => Self::NoneToUnfreeze,
            nu::JobError::CannotUnfreeze { .. } => Self::CannotUnfreeze,
            nu::JobError::AlreadyFrozen { .. } => Self::AlreadyFrozen,
            nu::JobError::RecvTimeout { .. } => Self::RecvTimeout,
        }
    }
}
