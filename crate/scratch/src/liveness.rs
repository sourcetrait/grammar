//! Whether a marker's owner still runs: alive, proven dead, or unknown.
use crate::*;

/// What a probe answers of the process holding a pid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessAnswer {
    /// A process holds the pid, started at `start`, a zombie once it exits unreaped.
    Started { start: u64, zombie: bool },
    /// No process holds the pid.
    Absent,
    /// The null signal was refused: another user's process, or a sandbox's denial.
    Denied,
    /// A process holds the pid, and its start can't be read.
    Unreadable,
}

/// The process table a liveness check asks.
pub trait Probe {
    /// The process holding `pid`.
    fn process(
        &self,
        pid: u32,
    ) -> ProcessAnswer;

    /// The running boot's identity, none where it can't be read or isn't kept.
    fn boot(&self) -> Option<String>;

    /// The PID namespace the probe's pids are numbered in, none where unread.
    fn pid_namespace(&self) -> Option<String>;

    /// Whether the probe's platform numbers pids in PID namespaces at all.
    fn keeps_pid_namespaces(&self) -> bool;
}

/// The host's table: its process records, then a null signal where they fail.
#[derive(Debug, Clone, Copy, Default)]
pub struct HostProbe;

impl Probe for HostProbe {
    fn process(
        &self,
        pid: u32,
    ) -> ProcessAnswer {
        if let Some(process) = sourcetrait_process::process(pid) {
            return ProcessAnswer::Started {
                start: process.start_time,
                zombie: process.zombie,
            };
        }
        let Ok(raw) = i32::try_from(pid) else {
            return ProcessAnswer::Unreadable;
        };
        if raw <= 0 {
            return ProcessAnswer::Unreadable;
        }
        match nix::sys::signal::kill(nix::unistd::Pid::from_raw(raw), None) {
            Err(nix::errno::Errno::ESRCH) => ProcessAnswer::Absent,
            Err(nix::errno::Errno::EPERM) => ProcessAnswer::Denied,
            _ => ProcessAnswer::Unreadable,
        }
    }

    fn boot(&self) -> Option<String> {
        owner::current_boot()
    }

    fn pid_namespace(&self) -> Option<String> {
        owner::current_pid_namespace()
    }

    fn keeps_pid_namespaces(&self) -> bool {
        cfg!(target_os = "linux")
    }
}

/// Whether a marker's owner runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Liveness {
    /// It runs.
    Alive,
    /// It is proven gone, and how.
    Dead(&'static str),
    /// Neither is proven, and why.
    Unknown(&'static str),
}

/// `owner`'s liveness as `probe` answers it.
pub fn liveness(
    owner: &Owner,
    probe: &impl Probe,
) -> Liveness {
    match (&owner.boot, probe.boot()) {
        (Some(then), Some(now)) if *then == now => {}
        _ => return Liveness::Unknown("its boot is another, an earlier one or another machine's, or none is known"),
    }
    if probe.keeps_pid_namespaces() {
        match (&owner.pidns, probe.pid_namespace()) {
            (Some(then), Some(now)) if *then == now => {}
            (Some(_), Some(_)) => return Liveness::Unknown("its pid is numbered in another PID namespace"),
            _ => return Liveness::Unknown("its PID namespace or this one can't be read"),
        }
    } else if owner.pidns.is_some() {
        return Liveness::Unknown("it names a PID namespace this platform keeps none of");
    }
    match probe.process(owner.pid) {
        ProcessAnswer::Started { start, zombie: false } if start == owner.start => Liveness::Alive,
        ProcessAnswer::Started { start, zombie: true } if start == owner.start => {
            Liveness::Dead("it has exited, its parent yet to reap it")
        }
        ProcessAnswer::Started { .. } => Liveness::Dead("another process holds its pid"),
        ProcessAnswer::Absent => Liveness::Dead("no process holds its pid"),
        ProcessAnswer::Denied => Liveness::Unknown("the null signal to its pid was refused"),
        ProcessAnswer::Unreadable => Liveness::Unknown("the process holding its pid can't be read"),
    }
}
