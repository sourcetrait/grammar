//! The namespace lock: one engine a namespace, held from a start to its end.
use crate::*;
use std::io::{Seek, SeekFrom};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

/// The lock's file in the namespace's data home.
pub const LOCK_FILE: &str = "host.lock";

/// The most bytes of a holder's record a contention report shows.
const HOLDER_MAX: u64 = 1024;

/// What a holder writes into the lock it holds: whose namespace, which process.
#[derive(Debug, Clone, PartialEq, Eq, ser::Serialize)]
pub struct LockRecord {
    /// The program holding it.
    pub holder: String,
    pub uid: u32,
    pub id: String,
    pub namespace: String,
    pub pid: u32,
    /// Its start, in the platform's unit.
    pub start: u64,
    /// The boot it started in, where its start alone names none.
    pub boot: Option<String>,
    /// The holder's nom for this start.
    pub nom: String,
}

impl LockRecord {
    /// This process as `holder` of `id`'s `namespace`, under `nom`.
    pub fn current(
        holder: &str,
        id: &str,
        namespace: &str,
        nom: &str,
    ) -> io::Result<Self> {
        let pid = std::process::id();
        let start = sourcetrait_process::start_time(pid)
            .ok_or_else(|| io::Error::other("this process's start can't be read"))?;
        Ok(Self {
            holder: holder.to_string(),
            uid: current_uid(),
            id: id.to_string(),
            namespace: namespace.to_string(),
            pid,
            start,
            boot: current_boot(),
            nom: nom.to_string(),
        })
    }
}

/// Why a namespace's lock isn't held.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockError {
    /// Another process holds it; its record as it wrote it.
    Held { path: PathBuf, holder: String },
    /// It can't be taken or written for a reason of its own.
    Unavailable { path: PathBuf, reason: String },
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Held { path, holder } => {
                write!(f, "the namespace lock {} is held: {holder}", path.display())
            }
            Self::Unavailable { path, reason } => {
                write!(f, "the namespace lock {} can't be taken: {reason}", path.display())
            }
        }
    }
}

impl std::error::Error for LockError {}

/// The namespace's lock, held while it lives; its drop releases the lock.
#[derive(Debug)]
pub struct NamespaceLock {
    #[cfg(unix)]
    file: nix::fcntl::Flock<fs::File>,
    #[cfg(not(unix))]
    file: fs::File,
    path: PathBuf,
}

impl NamespaceLock {
    /// The lock at `path` taken without waiting, refused while anyone holds it.
    pub fn acquire(path: &Path) -> Result<Self, LockError> {
        let unavailable = |reason: String| LockError::Unavailable {
            path: path.to_path_buf(),
            reason,
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| unavailable(e.to_string()))?;
        }
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        let file = private_file(no_follow(&mut options))
            .open(path)
            .map_err(|e| unavailable(format!("it does not open as a plain file: {e}")))?;
        let meta = file.metadata().map_err(|e| unavailable(e.to_string()))?;
        if !meta.file_type().is_file() {
            return Err(unavailable("it is not a regular file".to_string()));
        }
        #[cfg(unix)]
        if meta.uid() != current_uid() {
            return Err(unavailable(format!("UID {} owns it", meta.uid())));
        }
        Self::lock(file, path)
    }

    /// The opened lock taken exclusive by `flock`, without waiting.
    #[cfg(unix)]
    fn lock(
        file: fs::File,
        path: &Path,
    ) -> Result<Self, LockError> {
        match nix::fcntl::Flock::lock(file, nix::fcntl::FlockArg::LockExclusiveNonblock) {
            Ok(file) => Ok(Self {
                file,
                path: path.to_path_buf(),
            }),
            Err((mut file, nix::errno::Errno::EWOULDBLOCK)) => Err(LockError::Held {
                path: path.to_path_buf(),
                holder: holder_of(&mut file),
            }),
            Err((_, errno)) => Err(LockError::Unavailable {
                path: path.to_path_buf(),
                reason: errno.to_string(),
            }),
        }
    }

    /// The opened lock taken exclusive by the system's file lock, without waiting.
    #[cfg(not(unix))]
    fn lock(
        mut file: fs::File,
        path: &Path,
    ) -> Result<Self, LockError> {
        match file.try_lock() {
            Ok(()) => Ok(Self {
                file,
                path: path.to_path_buf(),
            }),
            Err(fs::TryLockError::WouldBlock) => Err(LockError::Held {
                path: path.to_path_buf(),
                holder: holder_of(&mut file),
            }),
            Err(fs::TryLockError::Error(e)) => Err(LockError::Unavailable {
                path: path.to_path_buf(),
                reason: e.to_string(),
            }),
        }
    }

    /// The lock's path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The holder's record written into the lock in place, its inode kept.
    pub fn record(
        &mut self,
        record: &LockRecord,
    ) -> Result<(), LockError> {
        let unavailable = |reason: String| LockError::Unavailable {
            path: self.path.clone(),
            reason,
        };
        let text = nuval::to_nuon(record, NuonStyle::Default).map_err(|e| unavailable(e.to_string()))?;
        self.file.set_len(0).map_err(|e| unavailable(e.to_string()))?;
        self.file.seek(SeekFrom::Start(0)).map_err(|e| unavailable(e.to_string()))?;
        self.file
            .write_all(format!("{text}\n").as_bytes())
            .map_err(|e| unavailable(e.to_string()))?;
        self.file.sync_data().map_err(|e| unavailable(e.to_string()))
    }
}

/// A holder's record, one line and bounded, or why it can't be read.
fn holder_of(file: &mut fs::File) -> String {
    let mut text = String::new();
    let read = file
        .seek(SeekFrom::Start(0))
        .and_then(|_| Read::by_ref(file).take(HOLDER_MAX).read_to_string(&mut text));
    if let Err(e) = read {
        return format!("a holder whose record can't be read: {e}");
    }
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.is_empty() {
        "a holder that wrote no record".to_string()
    } else {
        line
    }
}
