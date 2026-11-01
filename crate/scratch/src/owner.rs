//! A scratch directory's owner: the marker naming the process that made it.
use crate::*;

/// The marker's file in its directory.
pub(crate) const MARKER_FILE: &str = "owner.nuon";

/// The predecessor's marker, `<pid> <start>`, read where `owner.nuon` is absent.
pub(crate) const LEGACY_MARKER_FILE: &str = "owner";

/// The most bytes a marker is read to.
const MARKER_MAX: u64 = 4096;

/// The marker's schema, which every marker is held to.
const OWNER_MODEL: &str = include_str!("../../../include/scratch/Owner.nutype");

/// The running boot's identity on Linux.
#[cfg(target_os = "linux")]
const BOOT_ID: &str = "/proc/sys/kernel/random/boot_id";

/// This process's PID namespace on Linux, the link naming it.
#[cfg(target_os = "linux")]
const PID_NAMESPACE: &str = "/proc/self/ns/pid";

/// Windows' System process, running from boot, its creation time the boot's.
#[cfg(windows)]
const WINDOWS_SYSTEM_PID: u32 = 4;

/// The process that made a scratch directory, as its marker names it.
#[derive(Debug, Clone, PartialEq, Eq, ser::Serialize, ser::Deserialize)]
pub struct Owner {
    pub uid: u32,
    pub pid: u32,
    /// Its start, in the platform's unit.
    pub start: u64,
    /// The boot it started in, where its start alone names none.
    pub boot: Option<String>,
    /// The PID namespace its pid is numbered in, where the platform keeps them.
    pub pidns: Option<String>,
    /// The guard's own tag, none in a predecessor's marker.
    pub nonce: Option<String>,
}

/// Why a directory's marker names no owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkerError {
    /// Neither marker is there.
    Absent,
    /// A marker is there and cannot be read as a plain file.
    Unreadable(String),
    /// A marker is read and names no owner.
    Malformed(String),
}

impl fmt::Display for MarkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Absent => write!(f, "it holds no marker"),
            Self::Unreadable(reason) => write!(f, "its marker can't be read: {reason}"),
            Self::Malformed(reason) => write!(f, "its marker is malformed: {reason}"),
        }
    }
}

impl Owner {
    /// This process under its boot, with a fresh nonce.
    pub fn current() -> io::Result<Self> {
        let pid = std::process::id();
        let start = sourcetrait_process::start_time(pid)
            .ok_or_else(|| io::Error::other("this process's start can't be read"))?;
        Ok(Self {
            uid: current_uid(),
            pid,
            start,
            boot: current_boot(),
            pidns: current_pid_namespace(),
            nonce: Some(random_nonce()?),
        })
    }

    /// The marker written into `dir`, a file nothing else there may hold yet.
    pub(crate) fn write(
        &self,
        dir: &Path,
    ) -> io::Result<()> {
        let text = nuval::to_nuon(self, NuonStyle::Default).map_err(|e| io::Error::other(e.to_string()))?;
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        let mut file = private_file(no_follow(&mut options)).open(dir.join(MARKER_FILE))?;
        file.write_all(format!("{text}\n").as_bytes())
    }

    /// The owner `dir`'s marker names, else the predecessor's under `dir_uid`.
    pub fn read(
        dir: &Path,
        dir_uid: u32,
    ) -> Result<Self, MarkerError> {
        if let Some(text) = read_marker(&dir.join(MARKER_FILE))? {
            return parse_marker(&text);
        }
        match read_marker(&dir.join(LEGACY_MARKER_FILE))? {
            Some(text) => parse_legacy(&text, dir_uid),
            None => Err(MarkerError::Absent),
        }
    }

    /// The owner, refused where its pid could name a group or every process.
    fn checked(self) -> Result<Self, MarkerError> {
        if self.pid == 0 || i32::try_from(self.pid).is_err() {
            return Err(MarkerError::Malformed(format!("its pid {} names no one process", self.pid)));
        }
        Ok(self)
    }
}

/// The effective UID this process runs as, 0 on Windows, which names users by SID.
pub fn current_uid() -> u32 {
    #[cfg(unix)]
    {
        nix::unistd::geteuid().as_raw()
    }
    #[cfg(not(unix))]
    {
        0
    }
}

/// This machine's running boot, apart from every other boot; none where unread.
pub(crate) fn current_boot() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        fs::read_to_string(BOOT_ID).ok().map(|id| id.trim().to_string()).filter(|id| !id.is_empty())
    }
    #[cfg(target_os = "macos")]
    {
        let mut buf = [0u8; 64];
        let mut len = buf.len();
        let read = unsafe {
            nix::libc::sysctlbyname(
                c"kern.bootsessionuuid".as_ptr(),
                buf.as_mut_ptr().cast(),
                &mut len,
                std::ptr::null_mut(),
                0,
            )
        };
        if read != 0 || len > buf.len() {
            return None;
        }
        let id = std::str::from_utf8(&buf[..len]).ok()?.trim_end_matches('\0').trim();
        (!id.is_empty()).then(|| id.to_string())
    }
    #[cfg(windows)]
    {
        sourcetrait_process::process(WINDOWS_SYSTEM_PID).map(|system| system.start_time.to_string())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        None
    }
}

/// This process's PID namespace on Linux, none elsewhere or where unreadable.
pub(crate) fn current_pid_namespace() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        fs::read_link(PID_NAMESPACE).ok().map(|link| link.to_string_lossy().into_owned())
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Sixteen random bytes from the system's generator, as lower-case hex.
fn random_nonce() -> io::Result<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| io::Error::other(e.to_string()))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// A marker's text: no symlink followed, never waited on, plain, and bounded.
fn read_marker(path: &Path) -> Result<Option<String>, MarkerError> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    let opened = no_follow(&mut options).open(path);
    let file = match opened {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(MarkerError::Unreadable(e.to_string())),
    };
    let meta = file.metadata().map_err(|e| MarkerError::Unreadable(e.to_string()))?;
    if !meta.file_type().is_file() {
        return Err(MarkerError::Unreadable("it is not a regular file".to_string()));
    }
    let mut text = String::new();
    file.take(MARKER_MAX + 1)
        .read_to_string(&mut text)
        .map_err(|e| MarkerError::Malformed(e.to_string()))?;
    if text.len() as u64 > MARKER_MAX {
        return Err(MarkerError::Malformed(format!("it passes {MARKER_MAX} bytes")));
    }
    Ok(Some(text))
}

/// The owner a marker's NUON names, held to the owner model.
fn parse_marker(text: &str) -> Result<Owner, MarkerError> {
    let malformed = |reason: String| MarkerError::Malformed(reason);
    let value = NuVal::from_nuon(text).map_err(|e| malformed(e.to_string()))?;
    let model = NuTypeFile::from_text(OWNER_MODEL).map_err(|e| malformed(format!("the owner model: {e}")))?;
    model.ty.check(&value).map_err(|e| malformed(e.to_string()))?;
    let owner: Owner = nuval::from_nuval(value).map_err(|e| malformed(e.to_string()))?;
    owner.checked()
}

/// The owner a predecessor's `<pid> <start>` names, its UID the directory's.
fn parse_legacy(
    text: &str,
    dir_uid: u32,
) -> Result<Owner, MarkerError> {
    let mut parts = text.split_whitespace();
    let (Some(pid), Some(start), None) = (parts.next(), parts.next(), parts.next()) else {
        return Err(MarkerError::Malformed("the predecessor's marker holds no `<pid> <start>`".to_string()));
    };
    let pid = pid.parse().map_err(|_| MarkerError::Malformed(format!("its pid `{pid}` is no number")))?;
    let start = start.parse().map_err(|_| MarkerError::Malformed(format!("its start `{start}` is no number")))?;
    Owner {
        uid: dir_uid,
        pid,
        start,
        boot: None,
        pidns: None,
        nonce: None,
    }
    .checked()
}
