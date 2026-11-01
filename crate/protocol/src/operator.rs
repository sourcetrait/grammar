use crate::*;

/// What the operator's manager observed of the engine and the Channel.
#[cereal::derived(Data)]
pub struct Observation {
    /// The engine's reading, none when it didn't answer in time.
    pub engine: Option<EngineHealth>,
    /// Whether the host's Channel is open, which a test host samples by.
    pub listening: bool,
}

/// Copy discriminant mirror of `Emergency`, without the payload.
#[cereal::derived(Data, Copy, Eq, Hash)]
pub enum EmergencyKind {
    HungEngineThread,
    CpuWarning,
    RamWarning,
    VramWarning,
    BackgroundJobsWarning,
    DiskWarning,
    ChannelSpamWarning,
    ChannelSpamError,
    Critical,
}

impl EmergencyKind {
    /// The kind's name, as its log line's `kind` carries it.
    pub fn name(self) -> &'static str {
        match self {
            Self::HungEngineThread => "hung_engine_thread",
            Self::CpuWarning => "cpu_warning",
            Self::RamWarning => "ram_warning",
            Self::VramWarning => "vram_warning",
            Self::BackgroundJobsWarning => "background_jobs_warning",
            Self::DiskWarning => "disk_warning",
            Self::ChannelSpamWarning => "channel_spam_warning",
            Self::ChannelSpamError => "channel_spam_error",
            Self::Critical => "critical",
        }
    }
}

/// A cancelled engine thread still alive past the grace window.
#[cereal::derived(Data)]
pub struct HungEngineThreadEmergency {
    pub nonce: String,
    pub lane: String,
    pub tool: String,
    pub started_at: u64,
    pub cancelled_at: u64,
    pub hung_ms: u64,
    pub pool_held: u64,
    pub pool_cap: u64,
}

/// Sustained host-process CPU over the sample window.
#[cereal::derived(Data)]
pub struct CpuWarningEmergency {
    pub cpu_pct: f64,
    pub sample_ms: u64,
}

/// Host-process resident set size above the threshold.
#[cereal::derived(Data)]
pub struct RamWarningEmergency {
    pub rss_kb: u64,
}

/// GPU memory in use above the line, summed across cards.
#[cereal::derived(Data)]
pub struct VramWarningEmergency {
    pub used_mib: u64,
    pub total_mib: u64,
}

/// The environment-wide jobs table above the threshold count.
#[cereal::derived(Data)]
pub struct BackgroundJobsWarningEmergency {
    pub job_count: u64,
}

/// A watched filesystem reached the warning line.
#[cereal::derived(Data)]
pub struct DiskWarningEmergency {
    pub mount: String,
    pub used_pct: u32,
    pub threshold_pct: u32,
}

/// An origin crossed the SOFT send threshold, once per origin.
#[cereal::derived(Data)]
pub struct ChannelSpamWarningEmergency {
    pub origin: String,
    pub hits: u32,
    pub window_secs: u64,
    pub rate: u32,
}

/// An origin crossed the HARD threshold and was stopped.
#[cereal::derived(Data)]
pub struct ChannelSpamErrorEmergency {
    pub origin: String,
    pub hits: u32,
    pub window_secs: u64,
    pub rate: u32,
    pub action: String,
}

/// The unambiguous total-failure case; the response is an MCP restart.
#[cereal::derived(Data)]
pub struct CriticalEmergency {
    pub reason: String,
    pub hung: u64,
    pub cap: u64,
}

/// A classified resource condition.
#[cereal::derived(Data)]
pub enum Emergency {
    HungEngineThread(HungEngineThreadEmergency),
    CpuWarning(CpuWarningEmergency),
    RamWarning(RamWarningEmergency),
    VramWarning(VramWarningEmergency),
    BackgroundJobsWarning(BackgroundJobsWarningEmergency),
    DiskWarning(DiskWarningEmergency),
    ChannelSpamWarning(ChannelSpamWarningEmergency),
    ChannelSpamError(ChannelSpamErrorEmergency),
    Critical(CriticalEmergency),
}

impl Emergency {
    /// The condition's kind, without its payload.
    pub fn kind(&self) -> EmergencyKind {
        match self {
            Self::HungEngineThread(_) => EmergencyKind::HungEngineThread,
            Self::CpuWarning(_) => EmergencyKind::CpuWarning,
            Self::RamWarning(_) => EmergencyKind::RamWarning,
            Self::VramWarning(_) => EmergencyKind::VramWarning,
            Self::BackgroundJobsWarning(_) => EmergencyKind::BackgroundJobsWarning,
            Self::DiskWarning(_) => EmergencyKind::DiskWarning,
            Self::ChannelSpamWarning(_) => EmergencyKind::ChannelSpamWarning,
            Self::ChannelSpamError(_) => EmergencyKind::ChannelSpamError,
            Self::Critical(_) => EmergencyKind::Critical,
        }
    }
}
