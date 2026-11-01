//! The host's reap of the orphans its evals leave, and its subreaper.
#[cfg(target_os = "linux")]
use crate::*;

/// How long a zombie must persist before we treat it as ABANDONED.
#[cfg(target_os = "linux")]
const REAP_GRACE_SECS: u64 = 5;

/// Harvests children the subreaper ADOPTED that nobody is waiting on.
pub(crate) struct OrphanReaper {
    #[cfg(target_os = "linux")]
    seen: HashMap<u32, Instant>,
}

impl Default for OrphanReaper {
    fn default() -> Self {
        Self::new()
    }
}

impl OrphanReaper {
    pub(crate) fn new() -> Self {
        Self {
            #[cfg(target_os = "linux")]
            seen: HashMap::new(),
        }
    }

    /// One pass: observe our zombies, harvest the ones past the grace window.
    #[cfg(target_os = "linux")]
    pub(crate) fn reap(&mut self) {
        let zombies = zombie_children_of(process::id());
        self.seen.retain(|pid, _| zombies.contains(pid));
        for pid in zombies {
            let first_seen = *self.seen.entry(pid).or_insert_with(Instant::now);
            if first_seen.elapsed().as_secs() < REAP_GRACE_SECS {
                continue;
            }
            let _ = nix::sys::wait::waitpid(
                nix::unistd::Pid::from_raw(pid as i32),
                Some(nix::sys::wait::WaitPidFlag::WNOHANG),
            );
            self.seen.remove(&pid);
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub(crate) fn reap(&mut self) {}
}

/// Our own direct children that are zombies.
#[cfg(target_os = "linux")]
fn zombie_children_of(parent: u32) -> Vec<u32> {
    sourcetrait_process::processes()
        .into_iter()
        .filter(|p| p.zombie && p.ppid == parent)
        .map(|p| p.pid)
        .collect()
}

/// Make the host the subreaper so an eval's orphans stay reachable.
#[cfg(target_os = "linux")]
pub fn install_child_subreaper() {
    if let Err(e) = nix::sys::prctl::set_child_subreaper(true) {
        eprintln!("grammar: PR_SET_CHILD_SUBREAPER failed: {e}");
    }
}

#[cfg(not(target_os = "linux"))]
pub fn install_child_subreaper() {}
