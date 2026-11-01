//! The worker's own scratch: its bulk and transient directories, for its life.
use crate::*;

/// The worker's scratch directories' prefix, apart from the frontend's.
pub(crate) const WORKER_PREFIX: &str = "grammar-ssh.";

/// The worker's two scratch directories, made at its start, removed at its end.
#[derive(Debug)]
pub struct WorkerScratch {
    tmp: OwnedDir,
    transient: OwnedDir,
    diagnostics: Vec<String>,
}

impl WorkerScratch {
    /// Each base under `homes` swept, then the worker's two directories made there.
    pub fn create(homes: &InstanceHomes) -> GrammarSshResult<Self> {
        let mut diagnostics = Vec::new();
        let transient_base = match &homes.runtime {
            Some(runtime) => runtime.clone(),
            None => {
                diagnostics.push(format!(
                    "no runtime home names a directory; transient files go under {}",
                    homes.tmp.display(),
                ));
                homes.tmp.clone()
            }
        };
        let uid = current_uid();
        diagnostics.extend(sweep(&homes.tmp, WORKER_PREFIX, uid, &HostProbe).diagnostics());
        if transient_base != homes.tmp {
            diagnostics.extend(sweep(&transient_base, WORKER_PREFIX, uid, &HostProbe).diagnostics());
        }
        let tmp = make(&homes.tmp)?;
        let transient = make(&transient_base).inspect_err(|_| {
            let _ = tmp.remove();
        })?;
        Ok(Self {
            tmp,
            transient,
            diagnostics,
        })
    }

    /// The bulk directory, a definition's `$env.EQUIP_TMP`.
    pub fn tmp(&self) -> &Path {
        self.tmp.path()
    }

    /// The transient directory, a definition's `$env.EQUIP_TRANSIENT`.
    pub fn transient(&self) -> &Path {
        self.transient.path()
    }

    /// What the start reports: the fallback, and each directory a sweep kept.
    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }

    /// Both directories removed, each while its marker is still this worker's.
    pub fn remove(&self) -> GrammarSshResult<()> {
        let tmp = self.tmp.remove().map_err(|e| unavailable(self.tmp.path(), e));
        let transient = self.transient.remove().map_err(|e| unavailable(self.transient.path(), e));
        tmp.and(transient)
    }
}

/// One worker directory in `base`, marked with a nonce of its own.
fn make(base: &Path) -> GrammarSshResult<OwnedDir> {
    let owner = Owner::current().map_err(|e| unavailable(base, e))?;
    OwnedDir::create(base, WORKER_PREFIX, &owner).map_err(|e| unavailable(base, e))
}

/// A scratch failure under `path`.
fn unavailable(
    path: &Path,
    e: std::io::Error,
) -> GrammarSshError {
    GrammarSshError::ScratchUnavailable {
        path: path.to_path_buf(),
        reason: e.to_string(),
    }
}
