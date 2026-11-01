use crate::*;

/// The kind of a nushell `ConfigError`, one per variant that isn't a wrapper.
#[cereal::derived(Data, Copy, Eq, Hash)]
pub enum ConfigErrorKind {
    TypeMismatch,
    InvalidValue,
    UnknownOption,
    MissingRequiredColumn,
    Deprecated,
    LockedAfterStartup,
}

impl ConfigErrorKind {
    /// The diagnostic code nushell gives the kind, where it gives one.
    pub const fn code(self) -> Option<&'static str> {
        match self {
            Self::TypeMismatch => Some("nu::shell::type_mismatch"),
            Self::InvalidValue => Some("nu::shell::invalid_value"),
            Self::UnknownOption => Some("nu::shell::unknown_config_option"),
            Self::MissingRequiredColumn => Some("nu::shell::missing_required_column"),
            Self::Deprecated => Some("nu::shell::deprecated_config_option"),
            Self::LockedAfterStartup => Some("nu::shell::config_option_locked_after_startup"),
        }
    }

    /// The kind's message, the same for every error of the kind.
    pub const fn message(self) -> &'static str {
        match self {
            Self::TypeMismatch => "Type mismatch in the config",
            Self::InvalidValue => "Invalid value in the config",
            Self::UnknownOption => "Unknown config option",
            Self::MissingRequiredColumn => "A config option requires a column",
            Self::Deprecated => "A config option is deprecated",
            Self::LockedAfterStartup => "A config option cannot be changed after Nushell has started",
        }
    }
}

#[cfg(feature = "nushell")]
impl ConfigErrorKind {
    /// The kind of an error, none for a variant wrapping another type.
    pub(crate) fn of(v: &nu::ConfigError) -> Option<Self> {
        match v {
            nu::ConfigError::TypeMismatch { .. } => Some(Self::TypeMismatch),
            nu::ConfigError::InvalidValue { .. } => Some(Self::InvalidValue),
            nu::ConfigError::UnknownOption { .. } => Some(Self::UnknownOption),
            nu::ConfigError::MissingRequiredColumn { .. } => Some(Self::MissingRequiredColumn),
            nu::ConfigError::Deprecated { .. } => Some(Self::Deprecated),
            nu::ConfigError::LockedAfterStartup { .. } => Some(Self::LockedAfterStartup),
            nu::ConfigError::ShellError(..) => None,
        }
    }
}

/// The kind of a nushell `ConfigWarning`, one per variant.
#[cereal::derived(Data, Copy, Eq, Hash)]
pub enum ConfigWarningKind {
    IncompatibleOptions,
    SharedKeybindingName,
}

impl ConfigWarningKind {
    /// The diagnostic code nushell gives the kind, where it gives one.
    pub const fn code(self) -> Option<&'static str> {
        match self {
            Self::IncompatibleOptions => Some("nu::shell::incompatible_options"),
            Self::SharedKeybindingName => Some("nu::shell::shared_keybindings_name"),
        }
    }

    /// The kind's message, the same for every error of the kind.
    pub const fn message(self) -> &'static str {
        match self {
            Self::IncompatibleOptions => "Incompatible options",
            Self::SharedKeybindingName => "Multiple keybindings share a name",
        }
    }
}

#[cfg(feature = "nushell")]
impl From<&nu::ConfigWarning> for ConfigWarningKind {
    fn from(v: &nu::ConfigWarning) -> Self {
        match v {
            nu::ConfigWarning::IncompatibleOptions { .. } => Self::IncompatibleOptions,
            nu::ConfigWarning::SharedKeybindingName { .. } => Self::SharedKeybindingName,
        }
    }
}
