//! Layered teardown of an eval's external process tree.
use crate::*;

/// Build this eval's external-child tracker.
pub(crate) fn make_tracker(cancel: Arc<AtomicBool>) -> nu::ThreadJob {
    let (sender, _rx) = std::sync::mpsc::channel::<nu::Mail>();
    nu::ThreadJob::new(
        nu::Signals::new(cancel),
        Some("grammar eval".to_string()),
        sender,
    )
}

/// SIGKILL every tracked child AND its descendants in the process table.
pub(crate) fn tree_kill(tracked: &[u32]) {
    let table = sourcetrait_process::processes();
    let mut victims: Vec<u32> = Vec::new();
    for &root in tracked {
        for descendant in sourcetrait_process::descendants_in(&table, root) {
            if !victims.contains(&descendant) {
                victims.push(descendant);
            }
        }
        if !victims.contains(&root) {
            victims.push(root);
        }
    }
    for pid in victims {
        let _ = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(pid as i32),
            nix::sys::signal::Signal::SIGKILL,
        );
    }
}

/// SIGKILL the host's plugin subprocesses.
pub(crate) fn kill_plugin_subprocesses() {
    let me = std::process::id();
    for p in sourcetrait_process::processes() {
        if p.ppid == me && p.name.starts_with("nu_plugin_") {
            let _ = nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(p.pid as i32),
                nix::sys::signal::Signal::SIGKILL,
            );
        }
    }
}
