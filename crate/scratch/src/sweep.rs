//! The sweep: proven-dead owners' directories removed, every other kept.
use crate::*;
use std::os::unix::fs::MetadataExt;

/// A directory a sweep kept, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept {
    pub path: PathBuf,
    pub reason: String,
    /// Kept for its owner running, which a report leaves out.
    pub live: bool,
}

/// What a sweep did under its base.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SweepReport {
    pub removed: Vec<PathBuf>,
    pub kept: Vec<Kept>,
}

impl SweepReport {
    /// Each directory kept with no running owner, as a line to report.
    pub fn diagnostics(&self) -> Vec<String> {
        self.kept
            .iter()
            .filter(|kept| !kept.live)
            .map(|kept| format!("{} is kept: {}", kept.path.display(), kept.reason))
            .collect()
    }
}

/// A sweep's judgement of one directory.
enum Verdict {
    Remove,
    Keep { reason: String, live: bool },
}

/// `base` swept of `prefix`'s directories whose owners are proven dead.
pub fn sweep(
    base: &Path,
    prefix: &str,
    uid: u32,
    probe: &impl Probe,
) -> SweepReport {
    let mut report = SweepReport::default();
    let entries = match fs::read_dir(base) {
        Ok(entries) => entries,
        Err(e) => {
            report.kept.push(Kept {
                path: base.to_path_buf(),
                reason: format!("the base can't be read: {e}"),
                live: false,
            });
            return report;
        }
    };
    for entry in entries.flatten() {
        if !entry.file_name().to_str().is_some_and(|name| name.starts_with(prefix)) {
            continue;
        }
        let path = entry.path();
        match judge(&path, uid, probe) {
            Verdict::Remove => match fs::remove_dir_all(&path) {
                Ok(()) => report.removed.push(path),
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => report.kept.push(Kept {
                    path,
                    reason: format!("its owner is gone and its removal failed: {e}"),
                    live: false,
                }),
            },
            Verdict::Keep { reason, live } => report.kept.push(Kept { path, reason, live }),
        }
    }
    report
}

/// One directory judged: removed only if `uid`'s and its owner proven dead.
fn judge(
    path: &Path,
    uid: u32,
    probe: &impl Probe,
) -> Verdict {
    let keep = |reason: String| Verdict::Keep { reason, live: false };
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) => return keep(format!("it can't be read: {e}")),
    };
    if meta.file_type().is_symlink() {
        return keep("it is a symlink".to_string());
    }
    if !meta.is_dir() {
        return keep("it is no directory".to_string());
    }
    if meta.uid() != uid {
        return keep(format!("UID {} owns it", meta.uid()));
    }
    let owner = match Owner::read(path, uid) {
        Ok(owner) => owner,
        Err(e) => return keep(e.to_string()),
    };
    if owner.uid != uid {
        return keep(format!("its marker names UID {}", owner.uid));
    }
    match liveness(&owner, probe) {
        Liveness::Alive => Verdict::Keep {
            reason: "its owner runs".to_string(),
            live: true,
        },
        Liveness::Dead(_) => Verdict::Remove,
        Liveness::Unknown(why) => keep(why.to_string()),
    }
}
