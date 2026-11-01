use crate::*;

/// The kind of a nushell I/O error, nushell's `shell_error::io::ErrorKind`.
#[cereal::derived(Data, Copy, Eq, Hash)]
pub enum IoErrorKind {
    Std(StdIoErrorKind),
    KillJobProcess,
    NotAFile,
    AlreadyInUse,
    FileNotFound,
    DirectoryNotFound,
}

impl IoErrorKind {
    /// The diagnostic code nushell gives the kind.
    pub const fn code(self) -> Option<&'static str> {
        Some(match self {
            Self::Std(kind) => kind.code(),
            Self::KillJobProcess => "nu::shell::io::kill_job_process",
            Self::NotAFile => "nu::shell::io::not_a_file",
            Self::AlreadyInUse => "nu::shell::io::already_in_use",
            Self::FileNotFound => "nu::shell::io::file_not_found",
            Self::DirectoryNotFound => "nu::shell::io::directory_not_found",
        })
    }

    /// The kind's message, the same for every error of the kind.
    pub const fn message(self) -> &'static str {
        match self {
            Self::Std(kind) => kind.message(),
            Self::KillJobProcess => "Killing job process failed",
            Self::NotAFile => "Not a file",
            Self::AlreadyInUse => "Already in use",
            Self::FileNotFound => "File not found",
            Self::DirectoryNotFound => "Directory not found",
        }
    }
}

#[cfg(feature = "nushell")]
impl From<&nu::IoKind> for IoErrorKind {
    fn from(v: &nu::IoKind) -> Self {
        match v {
            nu::IoKind::Std(kind, ..) => Self::Std((*kind).into()),
            nu::IoKind::KillJobProcess => Self::KillJobProcess,
            nu::IoKind::NotAFile => Self::NotAFile,
            nu::IoKind::AlreadyInUse => Self::AlreadyInUse,
            nu::IoKind::FileNotFound => Self::FileNotFound,
            nu::IoKind::DirectoryNotFound => Self::DirectoryNotFound,
        }
    }
}

/// The kind of a standard library I/O error, `std::io::ErrorKind`.
#[cereal::derived(Data, Copy, Eq, Hash)]
pub enum StdIoErrorKind {
    NotFound,
    PermissionDenied,
    ConnectionRefused,
    ConnectionReset,
    HostUnreachable,
    NetworkUnreachable,
    ConnectionAborted,
    NotConnected,
    AddrInUse,
    AddrNotAvailable,
    NetworkDown,
    BrokenPipe,
    AlreadyExists,
    WouldBlock,
    NotADirectory,
    IsADirectory,
    DirectoryNotEmpty,
    ReadOnlyFilesystem,
    StaleNetworkFileHandle,
    InvalidInput,
    InvalidData,
    TimedOut,
    WriteZero,
    StorageFull,
    NotSeekable,
    QuotaExceeded,
    FileTooLarge,
    ResourceBusy,
    ExecutableFileBusy,
    Deadlock,
    CrossesDevices,
    TooManyLinks,
    InvalidFilename,
    ArgumentListTooLong,
    Interrupted,
    Unsupported,
    UnexpectedEof,
    OutOfMemory,
    Other,
    Uncategorized,
}

impl StdIoErrorKind {
    /// The diagnostic code nushell gives the kind.
    pub const fn code(self) -> &'static str {
        match self {
            Self::NotFound => "nu::shell::io::not_found",
            Self::PermissionDenied => "nu::shell::io::permission_denied",
            Self::ConnectionRefused => "nu::shell::io::connection_refused",
            Self::ConnectionReset => "nu::shell::io::connection_reset",
            Self::HostUnreachable => "nu::shell::io::host_unreachable",
            Self::NetworkUnreachable => "nu::shell::io::network_unreachable",
            Self::ConnectionAborted => "nu::shell::io::connection_aborted",
            Self::NotConnected => "nu::shell::io::not_connected",
            Self::AddrInUse => "nu::shell::io::addr_in_use",
            Self::AddrNotAvailable => "nu::shell::io::addr_not_available",
            Self::NetworkDown => "nu::shell::io::network_down",
            Self::BrokenPipe => "nu::shell::io::broken_pipe",
            Self::AlreadyExists => "nu::shell::io::already_exists",
            Self::WouldBlock => "nu::shell::io::would_block",
            Self::NotADirectory => "nu::shell::io::not_a_directory",
            Self::IsADirectory => "nu::shell::io::is_a_directory",
            Self::DirectoryNotEmpty => "nu::shell::io::directory_not_empty",
            Self::ReadOnlyFilesystem => "nu::shell::io::read-only_filesystem_or_storage_medium",
            Self::StaleNetworkFileHandle => "nu::shell::io::stale_network_file_handle",
            Self::InvalidInput => "nu::shell::io::invalid_input",
            Self::InvalidData => "nu::shell::io::invalid_data",
            Self::TimedOut => "nu::shell::io::timed_out",
            Self::WriteZero => "nu::shell::io::write_zero",
            Self::StorageFull => "nu::shell::io::no_storage_space",
            Self::NotSeekable => "nu::shell::io::seek_on_unseekable_file",
            Self::QuotaExceeded => "nu::shell::io::quota_exceeded",
            Self::FileTooLarge => "nu::shell::io::file_too_large",
            Self::ResourceBusy => "nu::shell::io::resource_busy",
            Self::ExecutableFileBusy => "nu::shell::io::executable_file_busy",
            Self::Deadlock => "nu::shell::io::deadlock",
            Self::CrossesDevices => "nu::shell::io::cross-device_link_or_rename",
            Self::TooManyLinks => "nu::shell::io::too_many_links",
            Self::InvalidFilename => "nu::shell::io::invalid_filename",
            Self::ArgumentListTooLong => "nu::shell::io::argument_list_too_long",
            Self::Interrupted => "nu::shell::io::interrupted",
            Self::Unsupported => "nu::shell::io::unsupported",
            Self::UnexpectedEof => "nu::shell::io::unexpected_eof",
            Self::OutOfMemory => "nu::shell::io::out_of_memory",
            Self::Other => "nu::shell::io::other",
            Self::Uncategorized => "nu::shell::io::uncategorized_error",
        }
    }

    /// The kind's message, the same for every error of the kind.
    pub const fn message(self) -> &'static str {
        match self {
            Self::NotFound => "Not found",
            Self::PermissionDenied => "Permission denied",
            Self::ConnectionRefused => "Connection refused",
            Self::ConnectionReset => "Connection reset",
            Self::HostUnreachable => "Host unreachable",
            Self::NetworkUnreachable => "Network unreachable",
            Self::ConnectionAborted => "Connection aborted",
            Self::NotConnected => "Not connected",
            Self::AddrInUse => "Address in use",
            Self::AddrNotAvailable => "Address not available",
            Self::NetworkDown => "Network down",
            Self::BrokenPipe => "Broken pipe",
            Self::AlreadyExists => "Entity already exists",
            Self::WouldBlock => "Operation would block",
            Self::NotADirectory => "Not a directory",
            Self::IsADirectory => "Is a directory",
            Self::DirectoryNotEmpty => "Directory not empty",
            Self::ReadOnlyFilesystem => "Read-only filesystem or storage medium",
            Self::StaleNetworkFileHandle => "Stale network file handle",
            Self::InvalidInput => "Invalid input parameter",
            Self::InvalidData => "Invalid data",
            Self::TimedOut => "Timed out",
            Self::WriteZero => "Write zero",
            Self::StorageFull => "No storage space",
            Self::NotSeekable => "Seek on unseekable file",
            Self::QuotaExceeded => "Quota exceeded",
            Self::FileTooLarge => "File too large",
            Self::ResourceBusy => "Resource busy",
            Self::ExecutableFileBusy => "Executable file busy",
            Self::Deadlock => "Deadlock",
            Self::CrossesDevices => "Cross-device link or rename",
            Self::TooManyLinks => "Too many links",
            Self::InvalidFilename => "Invalid filename",
            Self::ArgumentListTooLong => "Argument list too long",
            Self::Interrupted => "Operation interrupted",
            Self::Unsupported => "Unsupported",
            Self::UnexpectedEof => "Unexpected end of file",
            Self::OutOfMemory => "Out of memory",
            Self::Other => "Other error",
            Self::Uncategorized => "Uncategorized error",
        }
    }
}

impl From<std::io::ErrorKind> for StdIoErrorKind {
    fn from(v: std::io::ErrorKind) -> Self {
        use std::io::ErrorKind as K;
        match v {
            K::NotFound => Self::NotFound,
            K::PermissionDenied => Self::PermissionDenied,
            K::ConnectionRefused => Self::ConnectionRefused,
            K::ConnectionReset => Self::ConnectionReset,
            K::HostUnreachable => Self::HostUnreachable,
            K::NetworkUnreachable => Self::NetworkUnreachable,
            K::ConnectionAborted => Self::ConnectionAborted,
            K::NotConnected => Self::NotConnected,
            K::AddrInUse => Self::AddrInUse,
            K::AddrNotAvailable => Self::AddrNotAvailable,
            K::NetworkDown => Self::NetworkDown,
            K::BrokenPipe => Self::BrokenPipe,
            K::AlreadyExists => Self::AlreadyExists,
            K::WouldBlock => Self::WouldBlock,
            K::NotADirectory => Self::NotADirectory,
            K::IsADirectory => Self::IsADirectory,
            K::DirectoryNotEmpty => Self::DirectoryNotEmpty,
            K::ReadOnlyFilesystem => Self::ReadOnlyFilesystem,
            K::StaleNetworkFileHandle => Self::StaleNetworkFileHandle,
            K::InvalidInput => Self::InvalidInput,
            K::InvalidData => Self::InvalidData,
            K::TimedOut => Self::TimedOut,
            K::WriteZero => Self::WriteZero,
            K::StorageFull => Self::StorageFull,
            K::NotSeekable => Self::NotSeekable,
            K::QuotaExceeded => Self::QuotaExceeded,
            K::FileTooLarge => Self::FileTooLarge,
            K::ResourceBusy => Self::ResourceBusy,
            K::ExecutableFileBusy => Self::ExecutableFileBusy,
            K::Deadlock => Self::Deadlock,
            K::CrossesDevices => Self::CrossesDevices,
            K::TooManyLinks => Self::TooManyLinks,
            K::InvalidFilename => Self::InvalidFilename,
            K::ArgumentListTooLong => Self::ArgumentListTooLong,
            K::Interrupted => Self::Interrupted,
            K::Unsupported => Self::Unsupported,
            K::UnexpectedEof => Self::UnexpectedEof,
            K::OutOfMemory => Self::OutOfMemory,
            K::Other => Self::Other,
            _ => Self::Uncategorized,
        }
    }
}
