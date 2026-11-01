//! The watchdog: sample the host, classify trouble, raise it edge-triggered.
use crate::*;

/// How often the watchdog samples.
pub const SAMPLE_INTERVAL: Duration = Duration::from_secs(2);
/// A cancelled thread still alive this long past its cancel is hung.
const HUNG_GRACE_MS: u64 = 5_000;

/// How long a level must HOLD before it is worth telling the agent about.
const LEVEL_SUSTAIN: Duration = Duration::from_secs(60);
/// The floor between two warnings about the SAME condition.
const LEVEL_REWARN: Duration = Duration::from_secs(30 * 60);

/// Environment-wide background job count.
const JOBS_THRESHOLD: f64 = 32.0;
/// nvidia-smi is sampled every Nth tick (it is a subprocess; keep it coarse).
const VRAM_SAMPLE_EVERY: u64 = 8;

/// Total system RAM in kB, read once from /proc/meminfo.
#[cfg(target_os = "linux")]
fn total_ram_kb() -> Option<f64> {
    static TOTAL: OnceLock<Option<u64>> = OnceLock::new();
    let cached = TOTAL.get_or_init(|| {
        let meminfo = fs::read_to_string("/proc/meminfo").ok()?;
        parse_mem_total_kb(&meminfo)
    });
    (*cached).map(|kb| kb as f64)
}

#[cfg(not(target_os = "linux"))]
fn total_ram_kb() -> Option<f64> {
    None
}

/// Parse `MemTotal:` (kB) out of /proc/meminfo contents.
#[cfg(any(target_os = "linux", test))]
pub(crate) fn parse_mem_total_kb(meminfo: &str) -> Option<u64> {
    for line in meminfo.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            return rest.split_whitespace().next()?.parse().ok();
        }
    }
    None
}

/// `df` is SLOW, so it is sampled rarely.
const DISK_SAMPLE_EVERY: u64 = 150;

/// Parse `df -P` output into `(mount, used_pct)` rows.
pub(crate) fn parse_df(output: &str) -> Vec<(String, u32)> {
    let mut rows = Vec::new();
    for line in output.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 6 {
            continue;
        }
        let Ok(used_pct) = fields[4].trim_end_matches('%').parse::<u32>() else {
            continue;
        };
        rows.push((fields[5..].join(" "), used_pct));
    }
    rows
}

/// Which filesystems we watch, and their gates.
#[derive(Default)]
pub(crate) struct DiskWatch {
    gates: HashMap<String, LevelGate>,
    baselined: bool,
}

impl DiskWatch {
    pub(crate) fn sample(
        &mut self,
        rows: &[(String, u32)],
        threshold_pct: u32,
        now: Instant,
    ) -> Vec<Emergency> {
        if !self.baselined {
            self.baselined = true;
            for (mount, used_pct) in rows {
                if *used_pct < threshold_pct {
                    self.gates.insert(mount.clone(), LevelGate::default());
                }
            }
            return Vec::new();
        }
        let mut out = Vec::new();
        for (mount, used_pct) in rows {
            if let Some(gate) = self.gates.get_mut(mount)
                && gate.sample(
                    *used_pct as f64,
                    threshold_pct as f64,
                    now,
                    LEVEL_SUSTAIN,
                    LEVEL_REWARN,
                )
            {
                out.push(Emergency::DiskWarning(DiskWarningEmergency {
                    mount: mount.clone(),
                    used_pct: *used_pct,
                    threshold_pct,
                }));
            }
        }
        out
    }
}

/// `df -P`'s rows, run on the calling thread; none when it fails.
pub(crate) fn probe_disk() -> Option<Vec<(String, u32)>> {
    let out = process::Command::new("df").arg("-P").output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(parse_df(&String::from_utf8_lossy(&out.stdout)))
}

/// The CPU line, as a percentage: all cores busy is `cores * 100`.
fn cpu_capacity_pct() -> f64 {
    std::thread::available_parallelism()
        .map(|n| n.get() as f64 * 100.0)
        .unwrap_or(100.0)
}

/// A debounced level detector with a long re-warn floor.
#[derive(Default)]
pub(crate) struct LevelGate {
    over_since: Option<Instant>,
    last_fired: Option<Instant>,
}

impl LevelGate {
    pub(crate) fn sample(
        &mut self,
        value: f64,
        fire_at: f64,
        now: Instant,
        sustain: Duration,
        rewarn: Duration,
    ) -> bool {
        if value < fire_at {
            self.over_since = None;
            return false;
        }
        let since = *self.over_since.get_or_insert(now);
        if now.saturating_duration_since(since) < sustain {
            return false;
        }
        if let Some(last) = self.last_fired
            && now.saturating_duration_since(last) < rewarn
        {
            return false;
        }
        self.last_fired = Some(now);
        true
    }
}

/// Linux `_SC_CLK_TCK`; the CPU conversion assumes the standard 100.
const CLK_TCK: f64 = 100.0;

/// Confirm the engine's watched threads as hung past `grace_ms`.
pub(crate) fn scan_hung(
    hung: &[HungThread],
    now: u64,
    grace_ms: u64,
    pool_held: u64,
    pool_cap: u64,
) -> (Vec<(String, Emergency)>, u64) {
    let mut out = Vec::new();
    let mut confirmed_stateless = 0u64;
    for w in hung {
        let hung_ms = now.saturating_sub(w.cancelled_at);
        if hung_ms <= grace_ms {
            continue;
        }
        if matches!(w.lane, ThreadLane::Stateless) {
            confirmed_stateless += 1;
        }
        out.push((
            format!("hung:{}:{}", w.lane.as_str(), w.nonce),
            Emergency::HungEngineThread(HungEngineThreadEmergency {
                nonce: w.nonce.clone(),
                lane: w.lane.as_str().to_string(),
                tool: w.tool.clone(),
                started_at: w.started_at,
                cancelled_at: w.cancelled_at,
                hung_ms,
                pool_held,
                pool_cap,
            }),
        ));
    }
    (out, confirmed_stateless)
}

/// The engine's confirmed hangs, and the Critical when they hold every permit.
pub(crate) fn engine_conditions(
    reading: &EngineReading,
    now: u64,
) -> Vec<(String, Emergency)> {
    let held = reading.permits_held;
    let cap = reading.permits_cap;
    let (mut conditions, confirmed_stateless) =
        scan_hung(&reading.hung, now, HUNG_GRACE_MS, held, cap);
    if cap > 0 && confirmed_stateless >= cap {
        conditions.push((
            "critical:stateless_saturation".to_string(),
            Emergency::Critical(CriticalEmergency {
                reason: "all stateless engine-thread permits held by hung threads; the stateless pool cannot dispatch"
                    .to_string(),
                hung: confirmed_stateless,
                cap,
            }),
        ));
    }
    conditions
}

/// The conditions newly arisen, and the active set the next sample diffs against.
pub(crate) fn rising_edges(
    prev_active: &HashSet<String>,
    current: Option<Vec<(String, Emergency)>>,
) -> (HashSet<String>, Vec<Emergency>) {
    let Some(current) = current else {
        return (prev_active.clone(), Vec::new());
    };
    let active = current.iter().map(|(key, _)| key.clone()).collect();
    let risen = current
        .into_iter()
        .filter(|(key, _)| !prev_active.contains(key))
        .map(|(_, emergency)| emergency)
        .collect();
    (active, risen)
}

/// Parse VmRSS (kB) out of `/proc/self/status` contents.
#[cfg(any(target_os = "linux", test))]
pub(crate) fn parse_rss_kb(status: &str) -> Option<u64> {
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            return rest.split_whitespace().next()?.parse().ok();
        }
    }
    None
}

/// Parse cumulative CPU jiffies (utime + stime) out of a stat line.
#[cfg(any(target_os = "linux", test))]
pub(crate) fn parse_cpu_ticks(stat: &str) -> Option<u64> {
    let rparen = stat.rfind(')')?;
    let fields: Vec<&str> = stat[rparen + 1..].split_whitespace().collect();
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some(utime + stime)
}

/// Sum `(used_mib, total_mib)` across the nvidia-smi CSV rows.
pub(crate) fn parse_vram(csv: &str) -> Option<(u64, u64)> {
    let mut used_sum = 0u64;
    let mut total_sum = 0u64;
    for line in csv.lines() {
        let mut parts = line.split(',');
        let (Some(u), Some(t)) = (parts.next(), parts.next()) else {
            continue;
        };
        let (Ok(u), Ok(t)) = (u.trim().parse::<u64>(), t.trim().parse::<u64>()) else {
            continue;
        };
        used_sum += u;
        total_sum += t;
    }
    if total_sum == 0 {
        None
    } else {
        Some((used_sum, total_sum))
    }
}

#[cfg(target_os = "linux")]
fn read_rss_kb() -> Option<u64> {
    parse_rss_kb(&fs::read_to_string("/proc/self/status").ok()?)
}

#[cfg(not(target_os = "linux"))]
fn read_rss_kb() -> Option<u64> {
    None
}

#[cfg(target_os = "linux")]
fn read_cpu_ticks() -> Option<u64> {
    parse_cpu_ticks(&fs::read_to_string("/proc/self/stat").ok()?)
}

#[cfg(not(target_os = "linux"))]
fn read_cpu_ticks() -> Option<u64> {
    None
}

/// Summed GPU memory from nvidia-smi, run on the calling thread; none without it.
pub(crate) fn probe_vram() -> Option<(u64, u64)> {
    let out = process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=memory.used,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_vram(&String::from_utf8_lossy(&out.stdout))
}

/// A probe's result, back from its blocking thread.
pub(crate) enum Probe {
    Vram(Option<(u64, u64)>),
    Disk(Option<Vec<(String, u32)>>),
}

/// The probes a tick found due, none of them already in flight.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct DueProbes {
    pub vram: bool,
    pub disk: bool,
}

/// The milliseconds since the epoch.
pub(crate) fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// The watchdog's state between its ticks, its observations, and its probes.
pub(crate) struct Watchdog {
    base: SupervisorConfig,
    test: bool,
    listening: bool,
    tick: u64,
    reaper: OrphanReaper,
    prev_cpu: Option<(u64, Instant)>,
    prev_active: HashSet<String>,
    cpu_gate: LevelGate,
    ram_gate: LevelGate,
    vram_gate: LevelGate,
    jobs_gate: LevelGate,
    disk: DiskWatch,
    vram_probing: bool,
    disk_probing: bool,
}

impl Watchdog {
    pub(crate) fn new(
        base: SupervisorConfig,
        test: bool,
    ) -> Self {
        Self {
            base,
            test,
            listening: false,
            tick: 0,
            reaper: OrphanReaper::new(),
            prev_cpu: None,
            prev_active: HashSet::new(),
            cpu_gate: LevelGate::default(),
            ram_gate: LevelGate::default(),
            vram_gate: LevelGate::default(),
            jobs_gate: LevelGate::default(),
            disk: DiskWatch::default(),
            vram_probing: false,
            disk_probing: false,
        }
    }

    /// May the watchdog SAMPLE now?
    fn online(&self) -> bool {
        !self.test || self.listening
    }

    /// Forget every reading, so sampling resumes fresh.
    fn reset(&mut self) {
        self.prev_cpu = None;
        self.prev_active.clear();
        self.cpu_gate = LevelGate::default();
        self.ram_gate = LevelGate::default();
        self.vram_gate = LevelGate::default();
        self.jobs_gate = LevelGate::default();
        self.disk = DiskWatch::default();
    }

    /// One tick: the housekeeping, then, while online, the process's levels.
    pub(crate) fn tick(
        &mut self,
        now: Instant,
    ) -> (Vec<Emergency>, DueProbes) {
        self.tick += 1;
        self.reaper.reap();
        reap_pins();
        if !self.online() {
            self.reset();
            return (Vec::new(), DueProbes::default());
        }
        let supervisor = effective_supervisor(self.base);
        let mut raised = Vec::new();

        if let Some(rss) = read_rss_kb()
            && let Some(total_kb) = total_ram_kb()
            && self.ram_gate.sample(
                rss as f64,
                total_kb * supervisor.ram_warn_fraction,
                now,
                LEVEL_SUSTAIN,
                LEVEL_REWARN,
            )
        {
            raised.push(Emergency::RamWarning(RamWarningEmergency { rss_kb: rss }));
        }

        let cur_ticks = read_cpu_ticks();
        if let (Some(ct), Some((pt, pinst))) = (cur_ticks, self.prev_cpu) {
            let elapsed = now.saturating_duration_since(pinst).as_secs_f64();
            if elapsed > 0.0 {
                let cpu_pct = (ct.saturating_sub(pt) as f64 / CLK_TCK) / elapsed * 100.0;
                if self.cpu_gate.sample(
                    cpu_pct,
                    cpu_capacity_pct() * supervisor.cpu_warn_fraction,
                    now,
                    LEVEL_SUSTAIN,
                    LEVEL_REWARN,
                ) {
                    raised.push(Emergency::CpuWarning(CpuWarningEmergency {
                        cpu_pct,
                        sample_ms: (elapsed * 1000.0) as u64,
                    }));
                }
            }
        }
        if let Some(ct) = cur_ticks {
            self.prev_cpu = Some((ct, now));
        }

        let due = DueProbes {
            vram: self.tick.is_multiple_of(VRAM_SAMPLE_EVERY) && !self.vram_probing,
            disk: (self.tick == 1 || self.tick.is_multiple_of(DISK_SAMPLE_EVERY))
                && !self.disk_probing,
        };
        self.vram_probing |= due.vram;
        self.disk_probing |= due.disk;
        (raised, due)
    }

    /// The engine and the Channel as observed: the hangs and the jobs, while online.
    pub(crate) fn observe(
        &mut self,
        observation: Observation,
        now_ms: u64,
        now: Instant,
    ) -> Vec<Emergency> {
        self.listening = observation.listening;
        if !self.online() {
            return Vec::new();
        }
        let mut raised = Vec::new();
        if let Some(reading) = &observation.engine {
            let jobs = reading.jobs;
            if self.jobs_gate.sample(
                jobs as f64,
                JOBS_THRESHOLD,
                now,
                LEVEL_SUSTAIN,
                LEVEL_REWARN,
            ) {
                raised.push(Emergency::BackgroundJobsWarning(
                    BackgroundJobsWarningEmergency { job_count: jobs },
                ));
            }
        }
        let current = observation
            .engine
            .as_ref()
            .map(|reading| engine_conditions(reading, now_ms));
        let (active, risen) = rising_edges(&self.prev_active, current);
        self.prev_active = active;
        raised.extend(risen);
        raised
    }

    /// A VRAM probe's result, its gate sampled while online.
    pub(crate) fn vram_sampled(
        &mut self,
        sample: Option<(u64, u64)>,
        now: Instant,
    ) -> Vec<Emergency> {
        self.vram_probing = false;
        if !self.online() {
            return Vec::new();
        }
        let Some((used, total)) = sample else {
            return Vec::new();
        };
        let headroom = effective_supervisor(self.base).vram_warn_headroom_mib;
        if total > 0
            && self.vram_gate.sample(
                used as f64,
                (total as f64 - headroom as f64).max(0.0),
                now,
                LEVEL_SUSTAIN,
                LEVEL_REWARN,
            )
        {
            vec![Emergency::VramWarning(VramWarningEmergency {
                used_mib: used,
                total_mib: total,
            })]
        } else {
            Vec::new()
        }
    }

    /// A disk probe's result, the first a baseline, while online.
    pub(crate) fn disk_sampled(
        &mut self,
        rows: Option<Vec<(String, u32)>>,
        now: Instant,
    ) -> Vec<Emergency> {
        self.disk_probing = false;
        if !self.online() {
            return Vec::new();
        }
        let Some(rows) = rows else {
            return Vec::new();
        };
        let threshold_pct = (effective_supervisor(self.base).disk_warn_fraction * 100.0) as u32;
        self.disk.sample(&rows, threshold_pct, now)
    }
}
