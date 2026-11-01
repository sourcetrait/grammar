use crate::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CacheKind {
    Execute,
    Calls,
}

impl CacheKind {
    fn dir_name(self) -> &'static str {
        match self {
            Self::Execute => "execute",
            Self::Calls => "calls",
        }
    }
}

pub(crate) fn cache_kind_dir(kind: CacheKind) -> PathBuf {
    cache_home().join(kind.dir_name())
}

pub fn cache_dir(kind: CacheKind, nonce: &datum::NoncePair) -> PathBuf {
    cache_kind_dir(kind).join(nonce.as_str())
}

/// The log dir of the call `nonce` names, in whichever cache kind holds it.
pub(crate) fn call_log_dir(nonce: &str) -> Option<PathBuf> {
    if nonce.is_empty() || !nonce.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    [CacheKind::Execute, CacheKind::Calls]
        .into_iter()
        .map(|kind| cache_kind_dir(kind).join(nonce))
        .find(|dir| dir.is_dir())
}

/// The cache entries of the call `nonce` names removed, in each kind it may sit in.
pub fn remove_call_cache(nonce: &str) {
    if nonce.is_empty() || !nonce.chars().all(|c| c.is_ascii_alphanumeric()) {
        return;
    }
    for kind in [CacheKind::Execute, CacheKind::Calls] {
        let dir = cache_kind_dir(kind).join(nonce);
        if dir.is_dir()
            && let Err(e) = fs::remove_dir_all(&dir)
        {
            eprintln!("grammar: the call cache {}: {e}", dir.display());
        }
    }
}

/// An `execute` call's definition, cached beside its logs for `rerun`.
pub const DEFINITION_FILE: &str = "definition.nu";

/// The definition an `execute` call cached, at `execute/<nonce>/definition.nu`.
pub fn definition_file(nonce: &str) -> PathBuf {
    cache_kind_dir(CacheKind::Execute).join(nonce).join(DEFINITION_FILE)
}
