use crate::*;

/// The kind of a nushell `CompileError`, one per variant.
#[cereal::derived(Data, Copy, Eq, Hash)]
pub enum CompileErrorKind {
    RegisterOverflow,
    RegisterUninitialized,
    RegisterUninitializedWhilePushingInstruction,
    DataOverflow,
    FileOverflow,
    InvalidRedirectMode,
    Garbage,
    UnsupportedOperatorExpression,
    AccessEnvByInt,
    InvalidKeywordCall,
    SetBranchTargetOfNonBranchInstruction,
    RunExternalNotFound,
    AssignmentRequiresVar,
    AssignmentRequiresMutableVar,
    AutomaticEnvVarSetManually,
    CannotReplaceEnv,
    UnexpectedExpression,
    MissingRequiredDeclaration,
    InvalidLiteral,
    NotInATry,
    NotInALoop,
    IncoherentLoopState,
    UndefinedLabel,
}

impl CompileErrorKind {
    /// The diagnostic code nushell gives the kind, where it gives one.
    pub const fn code(self) -> Option<&'static str> {
        match self {
            Self::RegisterOverflow => Some("nu::compile::register_overflow"),
            Self::RegisterUninitialized => Some("nu::compile::register_uninitialized"),
            Self::RegisterUninitializedWhilePushingInstruction => Some("nu::compile::register_uninitialized"),
            Self::DataOverflow => Some("nu::compile::data_overflow"),
            Self::FileOverflow => Some("nu::compile::register_overflow"),
            Self::InvalidRedirectMode => Some("nu::compile::invalid_redirect_mode"),
            Self::Garbage => Some("nu::compile::garbage"),
            Self::UnsupportedOperatorExpression => Some("nu::compile::unsupported_operator_expression"),
            Self::AccessEnvByInt => Some("nu::compile::access_env_by_int"),
            Self::InvalidKeywordCall => Some("nu::compile::invalid_keyword_call"),
            Self::SetBranchTargetOfNonBranchInstruction => Some("nu::compile::set_branch_target_of_non_branch_instruction"),
            Self::RunExternalNotFound => Some("nu::compile::run_external_not_found"),
            Self::AssignmentRequiresVar => Some("nu::compile::assignment_requires_variable"),
            Self::AssignmentRequiresMutableVar => Some("nu::compile::assignment_requires_mutable_variable"),
            Self::AutomaticEnvVarSetManually => Some("nu::compile::automatic_env_var_set_manually"),
            Self::CannotReplaceEnv => Some("nu::compile::cannot_replace_env"),
            Self::UnexpectedExpression => Some("nu::compile::unexpected_expression"),
            Self::MissingRequiredDeclaration => Some("nu::compile::missing_required_declaration"),
            Self::InvalidLiteral => Some("nu::compile::invalid_literal"),
            Self::NotInATry => Some("nu::compile::not_in_a_try"),
            Self::NotInALoop => Some("nu::compile::not_in_a_loop"),
            Self::IncoherentLoopState => Some("nu::compile::incoherent_loop_state"),
            Self::UndefinedLabel => Some("nu::compile::undefined_label"),
        }
    }

    /// The kind's message, the same for every error of the kind.
    pub const fn message(self) -> &'static str {
        match self {
            Self::RegisterOverflow => "Register overflow.",
            Self::RegisterUninitialized => "A register was uninitialized when used, possibly reused.",
            Self::RegisterUninitializedWhilePushingInstruction => "A register was uninitialized when used, possibly reused.",
            Self::DataOverflow => "Block contains too much string data: maximum 4 GiB exceeded.",
            Self::FileOverflow => "Block contains too many files.",
            Self::InvalidRedirectMode => "Invalid redirect mode: File should not be specified by commands.",
            Self::Garbage => "Encountered garbage, likely due to parse error.",
            Self::UnsupportedOperatorExpression => "Unsupported operator expression.",
            Self::AccessEnvByInt => "Attempted access of $env by integer path.",
            Self::InvalidKeywordCall => "Encountered an invalid keyword call.",
            Self::SetBranchTargetOfNonBranchInstruction => "Attempted to set branch target of non-branch instruction.",
            Self::RunExternalNotFound => "External calls are not supported.",
            Self::AssignmentRequiresVar => "Assignment operations require a variable.",
            Self::AssignmentRequiresMutableVar => "Assignment to an immutable variable.",
            Self::AutomaticEnvVarSetManually => "An automatic environment variable cannot be set manually.",
            Self::CannotReplaceEnv => "Cannot replace environment.",
            Self::UnexpectedExpression => "Unexpected expression.",
            Self::MissingRequiredDeclaration => "Missing required declaration.",
            Self::InvalidLiteral => "Invalid literal",
            Self::NotInATry => "Not in a try block.",
            Self::NotInALoop => "Not in a loop.",
            Self::IncoherentLoopState => "Incoherent loop state: the loop that ended was not the one we were expecting.",
            Self::UndefinedLabel => "Undefined label.",
        }
    }
}

#[cfg(feature = "nushell")]
impl From<&nu::CompileError> for CompileErrorKind {
    fn from(v: &nu::CompileError) -> Self {
        match v {
            nu::CompileError::RegisterOverflow { .. } => Self::RegisterOverflow,
            nu::CompileError::RegisterUninitialized { .. } => Self::RegisterUninitialized,
            nu::CompileError::RegisterUninitializedWhilePushingInstruction { .. } => Self::RegisterUninitializedWhilePushingInstruction,
            nu::CompileError::DataOverflow { .. } => Self::DataOverflow,
            nu::CompileError::FileOverflow { .. } => Self::FileOverflow,
            nu::CompileError::InvalidRedirectMode { .. } => Self::InvalidRedirectMode,
            nu::CompileError::Garbage { .. } => Self::Garbage,
            nu::CompileError::UnsupportedOperatorExpression { .. } => Self::UnsupportedOperatorExpression,
            nu::CompileError::AccessEnvByInt { .. } => Self::AccessEnvByInt,
            nu::CompileError::InvalidKeywordCall { .. } => Self::InvalidKeywordCall,
            nu::CompileError::SetBranchTargetOfNonBranchInstruction { .. } => Self::SetBranchTargetOfNonBranchInstruction,
            nu::CompileError::RunExternalNotFound { .. } => Self::RunExternalNotFound,
            nu::CompileError::AssignmentRequiresVar { .. } => Self::AssignmentRequiresVar,
            nu::CompileError::AssignmentRequiresMutableVar { .. } => Self::AssignmentRequiresMutableVar,
            nu::CompileError::AutomaticEnvVarSetManually { .. } => Self::AutomaticEnvVarSetManually,
            nu::CompileError::CannotReplaceEnv { .. } => Self::CannotReplaceEnv,
            nu::CompileError::UnexpectedExpression { .. } => Self::UnexpectedExpression,
            nu::CompileError::MissingRequiredDeclaration { .. } => Self::MissingRequiredDeclaration,
            nu::CompileError::InvalidLiteral { .. } => Self::InvalidLiteral,
            nu::CompileError::NotInATry { .. } => Self::NotInATry,
            nu::CompileError::NotInALoop { .. } => Self::NotInALoop,
            nu::CompileError::IncoherentLoopState { .. } => Self::IncoherentLoopState,
            nu::CompileError::UndefinedLabel { .. } => Self::UndefinedLabel,
        }
    }
}
