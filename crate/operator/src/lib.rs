pub(crate) mod msg;
pub(crate) mod pin;
pub(crate) mod reaper;
pub(crate) mod supervisor;
pub(crate) mod system;
pub(crate) mod watchdog;
#[cfg(test)]
mod tests {
    mod pin;
    mod reaper;
    mod watchdog;
}

pub(crate) use crate::{
    pin::reap_pins,
    reaper::OrphanReaper,
    watchdog::{Probe, Watchdog, now_millis, probe_disk, probe_vram},
};

pub(crate) use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    process,
    sync::LazyLock,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[cfg(target_os = "linux")]
pub(crate) use std::{fs, sync::OnceLock};

pub(crate) use sourcetrait_common::{agnostic, cereal, subsys};

pub(crate) use sourcetrait_grammar_protocol::{EngineHealth, HungEval, Lane};

pub(crate) mod ser {
    pub(crate) use ::serde::{Deserialize, Serialize};
}

pub(crate) mod tk {
    pub(crate) use tokio::{
        select,
        sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
        task::spawn_blocking,
        time::{MissedTickBehavior, interval},
    };
}

pub use sourcetrait_grammar_protocol::{
    BackgroundJobsWarningEmergency, ChannelSpamErrorEmergency, ChannelSpamWarningEmergency,
    CpuWarningEmergency, CriticalEmergency, DiskWarningEmergency, Emergency, EmergencyKind,
    HungEngineThreadEmergency, Observation, RamWarningEmergency, VramWarningEmergency,
};

pub use crate::{
    msg::{EngineReading, FromOperatorSys, HungThread, ThreadLane, ToOperatorSys},
    pin::{clear_pins, effective_supervisor, pin},
    reaper::install_child_subreaper,
    supervisor::{SupervisorConfig, check_fraction},
    system::{
        OperatorHandle, OperatorSysConfig, OperatorSysParams, OperatorSysPaths, OperatorSystem,
    },
    watchdog::SAMPLE_INTERVAL,
};
