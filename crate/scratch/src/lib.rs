pub(crate) mod dir;
pub(crate) mod liveness;
pub(crate) mod lock;
pub(crate) mod owner;
pub(crate) mod sweep;

pub(crate) use std::{
    fmt, fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

#[cfg(unix)]
pub(crate) use nix::fcntl::OFlag;
pub(crate) use nutype::NuTypeFile;
pub(crate) use nuval::{NuVal, NuonStyle};

pub(crate) use crate::owner::current_boot;

/// Options opening a path itself, never a link's target, and never waiting on it.
pub(crate) fn no_follow(options: &mut fs::OpenOptions) -> &mut fs::OpenOptions {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags((OFlag::O_NOFOLLOW | OFlag::O_NONBLOCK).bits())
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        /// `FILE_FLAG_OPEN_REPARSE_POINT`: a link opened as itself.
        const OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        options.custom_flags(OPEN_REPARSE_POINT)
    }
    #[cfg(not(any(unix, windows)))]
    {
        options
    }
}

/// Options creating a file its owner's alone, mode 0600 on Unix.
pub(crate) fn private_file(options: &mut fs::OpenOptions) -> &mut fs::OpenOptions {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600)
    }
    #[cfg(not(unix))]
    {
        options
    }
}

pub(crate) mod ser {
    pub(crate) use ::serde::{Deserialize, Serialize};
}

pub use crate::{
    dir::OwnedDir,
    liveness::{HostProbe, Liveness, ProcessAnswer, Probe, liveness},
    lock::{LOCK_FILE, LockError, LockRecord, NamespaceLock},
    owner::{MarkerError, Owner, current_uid},
    sweep::{Kept, SweepReport, sweep},
};
