use crate::*;
use crate::watchdog::{
    DiskWatch, DueProbes, LevelGate, parse_cpu_ticks, parse_df, parse_mem_total_kb, parse_rss_kb,
    parse_vram, rising_edges, scan_hung,
};

/// Real `df -P` output from the box, including the two cases that matter: the 4 KiB
/// pseudo-mount pinned at 100% by design, and a mount point that is genuinely worth
/// watching at 70%.
const DF: &str = "Filesystem             1K-blocks       Used  Available Use% Mounted on
shm                        64000         40      63960   1% /dev/shm
tmpfs                    6559820        248    6559572   1% /etc/hosts
/dev/mapper/data-root 3835242120 2534268808 1106078876  70% /data
overlay               3835242120 2534268808 1106078876  70% /
tmpfs                          4          4          0 100% /run/nvidia-ctk-hook4682c966
";

#[test]
fn df_parses_to_mount_and_percentage() {
    let rows = parse_df(DF);
    assert_eq!(rows.len(), 5, "the header is skipped; got {rows:?}");
    assert_eq!(rows[0], ("/dev/shm".to_string(), 1));
    assert_eq!(rows[2], ("/data".to_string(), 70));
    assert_eq!(rows[4], ("/run/nvidia-ctk-hook4682c966".to_string(), 100));
}

#[test]
fn mem_total_is_read_in_kb() {
    let meminfo = "MemTotal:       65598248 kB\nMemFree:  123 kB\n";
    assert_eq!(parse_mem_total_kb(meminfo), Some(65598248));
    assert_eq!(parse_mem_total_kb("MemFree: 1 kB\n"), None);
}

#[test]
fn the_baseline_never_warns_and_excludes_what_is_already_over() {
    let mut watch = DiskWatch::default();
    let now = Instant::now();
    assert!(
        watch.sample(&parse_df(DF), 80, now).is_empty(),
        "the baseline pass establishes what to watch; it is not itself an observation",
    );
    // The pseudo-mount was at 100% from the start - a pre-existing condition by design,
    // never ours to report - so it stays silent no matter how long it sits there.
    let rows = vec![("/run/nvidia-ctk-hook4682c966".to_string(), 100)];
    for secs in [0, 120, 4000] {
        let at = now + Duration::from_secs(secs);
        assert!(
            watch.sample(&rows, 80, at).is_empty(),
            "a mount excluded at baseline must never warn (t={secs})",
        );
    }
}

#[test]
fn a_watched_filesystem_warns_once_it_crosses() {
    let mut watch = DiskWatch::default();
    let base = Instant::now();
    watch.sample(&parse_df(DF), 80, base);

    let crossed = vec![("/data".to_string(), 81)];
    assert!(
        watch.sample(&crossed, 80, base).is_empty(),
        "one sample over the line is not yet sustained",
    );
    let emergencies = watch.sample(&crossed, 80, base + Duration::from_secs(300));
    assert_eq!(emergencies.len(), 1, "sustained across two samples; got {emergencies:?}");
    assert!(matches!(
        &emergencies[0],
        Emergency::DiskWarning(d) if d.mount == "/data" && d.used_pct == 81,
    ));
}

const SUSTAIN: Duration = Duration::from_secs(60);
const REWARN: Duration = Duration::from_secs(30 * 60);

/// Sample a gate at `t` seconds from `base`, firing at 150.
fn at(
    gate: &mut LevelGate,
    base: Instant,
    secs: u64,
    value: f64,
) -> bool {
    gate.sample(
        value,
        150.0,
        base + Duration::from_secs(secs),
        SUSTAIN,
        REWARN,
    )
}

#[test]
fn a_level_must_hold_before_it_warns() {
    let mut gate = LevelGate::default();
    let base = Instant::now();
    assert!(!at(&mut gate, base, 0, 200.0), "over, but not yet sustained");
    assert!(!at(&mut gate, base, 30, 200.0), "still inside the minute");
    assert!(at(&mut gate, base, 60, 200.0), "held for the full window");
    assert!(
        !at(&mut gate, base, 62, 200.0),
        "the agent needs ONE signal to decide on, not a stream of them",
    );
}

#[test]
fn a_momentary_spike_is_not_an_event() {
    let mut gate = LevelGate::default();
    let base = Instant::now();
    assert!(!at(&mut gate, base, 0, 200.0));
    assert!(!at(&mut gate, base, 2, 10.0), "dropped back");
    assert!(
        !at(&mut gate, base, 61, 200.0),
        "the clock restarts on the new excursion rather than counting the old one",
    );
}

#[test]
fn hovering_at_the_threshold_does_not_flap() {
    let mut gate = LevelGate::default();
    let base = Instant::now();
    at(&mut gate, base, 0, 200.0);
    assert!(at(&mut gate, base, 60, 200.0), "fires once");
    // Oscillating across the firing line: each dip restarts the sustain clock and the
    // re-warn floor outlasts the whole oscillation, so none of this is an event.
    for secs in [62, 64, 66, 68, 70, 200, 400] {
        let value = if secs % 4 == 0 { 149.0 } else { 200.0 };
        assert!(!at(&mut gate, base, secs, value), "no re-fire at t={secs}");
    }
}

#[test]
fn a_long_sustained_level_warns_only_twice_in_an_hour() {
    // The shape of a normal 30-60 minute inference run: resources pinned throughout,
    // which is NOT a fault. Warning about it repeatedly would talk over the work.
    let mut gate = LevelGate::default();
    let base = Instant::now();
    let mut warnings = 0;
    // Sample every 2s across a full hour, exactly as the watchdog ticks.
    for tick in 0..1800u64 {
        if at(&mut gate, base, tick * 2, 200.0) {
            warnings += 1;
        }
    }
    assert_eq!(
        warnings, 2,
        "one at the first sustained minute, one 30 minutes later - enough to notice, \
         not enough to nag",
    );
}

fn watch_entry(nonce: &str, lane: ThreadLane, cancelled_at: u64) -> HungThread {
    HungThread {
        nonce: nonce.to_string(),
        tool: "execute".to_string(),
        lane,
        started_at: 0,
        cancelled_at,
    }
}

#[test]
fn scan_confirms_stateless_hang_past_grace() {
    let hung = vec![watch_entry("w1", ThreadLane::Stateless, 1000)];
    // now 7000 - cancelled 1000 = hung 6000 > grace 5000 -> confirmed
    let (ems, confirmed) = scan_hung(&hung, 7000, 5000, 2, 8);
    assert_eq!(confirmed, 1);
    assert_eq!(ems.len(), 1);
    assert_eq!(ems[0].0, "hung:stateless:w1");
    assert!(matches!(ems[0].1, Emergency::HungEngineThread(_)));
}

#[test]
fn scan_ignores_within_grace() {
    let hung = vec![watch_entry("w1", ThreadLane::Stateless, 1000)];
    // hung 2000 <= grace 5000 -> not yet confirmed
    let (ems, confirmed) = scan_hung(&hung, 3000, 5000, 0, 8);
    assert_eq!(confirmed, 0);
    assert!(ems.is_empty());
}

#[test]
fn scan_stateful_hang_is_not_a_stateless_permit() {
    let hung = vec![watch_entry("i1", ThreadLane::Stateful, 0)];
    let (ems, confirmed) = scan_hung(&hung, 10000, 5000, 0, 8);
    assert_eq!(confirmed, 0, "a stateful-lane hang holds no pooled permit");
    assert_eq!(ems.len(), 1);
    assert_eq!(ems[0].0, "hung:stateful:i1");
    assert!(matches!(ems[0].1, Emergency::HungEngineThread(_)));
}

/// A sample the engine didn't answer is no reading: the hang it reported stays
/// active, so its answer on the next sample raises nothing twice.
#[test]
fn an_absent_reading_keeps_the_active_set() {
    let hung = scan_hung(&[watch_entry("w1", ThreadLane::Stateless, 0)], 10_000, 5_000, 1, 8).0;
    let (active, risen) = rising_edges(&HashSet::new(), Some(hung.clone()));
    assert_eq!(risen.len(), 1, "a new condition rises once");
    let (kept, quiet) = rising_edges(&active, None);
    assert!(quiet.is_empty());
    assert_eq!(kept, active, "an absent reading leaves the active set as it was");
    let (_, again) = rising_edges(&kept, Some(hung));
    assert!(again.is_empty(), "a condition still active is not raised again");
    let (cleared, none) = rising_edges(&kept, Some(Vec::new()));
    assert!(none.is_empty());
    assert!(cleared.is_empty(), "a reading without the condition clears it");
}

/// Lines no level of the test process reaches, so a tick of its own never warns.
const BASE: SupervisorConfig = SupervisorConfig {
    cpu_warn_fraction: 1.0,
    ram_warn_fraction: 1.0,
    vram_warn_headroom_mib: 0,
    disk_warn_fraction: 1.0,
};

/// An engine reading holding one watched thread, cancelled at the epoch.
fn holding(
    nonce: &str,
    lane: ThreadLane,
) -> EngineReading {
    EngineReading {
        hung: vec![watch_entry(nonce, lane, 0)],
        permits_held: 1,
        permits_cap: 8,
        jobs: 0,
    }
}

/// A probe that hasn't answered is never started again, so a wedged mount or
/// card costs one blocked thread rather than one a tick; its answer frees it.
#[test]
fn a_probe_in_flight_is_not_started_twice() {
    let mut watchdog = Watchdog::new(BASE, false);
    let now = Instant::now();
    let (_, first) = watchdog.tick(now);
    assert_eq!(first, DueProbes { vram: false, disk: true }, "the first tick takes the disk baseline");
    for _ in 2..8 {
        watchdog.tick(now);
    }
    assert!(watchdog.tick(now).1.vram, "every eighth tick probes the cards");
    for _ in 9..16 {
        watchdog.tick(now);
    }
    assert!(!watchdog.tick(now).1.vram, "the unanswered probe is not started again");
    watchdog.vram_sampled(None, now);
    for _ in 17..24 {
        watchdog.tick(now);
    }
    assert!(watchdog.tick(now).1.vram, "its answer frees the slot");
}

/// A test host samples only while its Channel is open: a closed one raises and
/// probes nothing, and opening it brings the hang in.
#[test]
fn a_test_host_samples_only_while_its_channel_is_open() {
    let mut watchdog = Watchdog::new(BASE, true);
    let closed = Observation {
        engine: Some(holding("w1", ThreadLane::Stateless)),
        listening: false,
    };
    assert!(watchdog.observe(closed, 10_000, Instant::now()).is_empty());
    assert_eq!(watchdog.tick(Instant::now()).1, DueProbes::default(), "nor does it probe");
    let open = Observation {
        engine: Some(holding("w1", ThreadLane::Stateless)),
        listening: true,
    };
    assert_eq!(watchdog.observe(open, 10_000, Instant::now()).len(), 1);
}

/// Through the observations a hang is raised once, kept across an absent
/// reading, and raised again only after a reading cleared it.
#[test]
fn an_observed_hang_is_raised_once_until_it_clears() {
    let mut watchdog = Watchdog::new(BASE, false);
    let now = Instant::now();
    let seen = |engine: Option<EngineReading>| Observation {
        engine,
        listening: false,
    };
    let held = || Some(holding("w1", ThreadLane::Stateful));
    let clear = || {
        Some(EngineReading {
            hung: Vec::new(),
            permits_held: 0,
            permits_cap: 8,
            jobs: 0,
        })
    };
    assert_eq!(watchdog.observe(seen(held()), 10_000, now).len(), 1);
    assert!(watchdog.observe(seen(None), 10_000, now).is_empty(), "an absent reading raises nothing");
    assert!(watchdog.observe(seen(held()), 12_000, now).is_empty(), "still active");
    assert!(watchdog.observe(seen(clear()), 14_000, now).is_empty());
    assert_eq!(watchdog.observe(seen(held()), 16_000, now).len(), 1, "raised again once cleared");
}

#[test]
fn parse_rss_reads_vmrss_kb() {
    let status = "Name:\tgrammar\nVmPeak:\t  100 kB\nVmRSS:\t  204800 kB\nThreads:\t9\n";
    assert_eq!(parse_rss_kb(status), Some(204800));
    assert_eq!(parse_rss_kb("Name:\tx\n"), None);
}

#[test]
fn parse_cpu_counts_fields_after_comm_parens() {
    // comm itself contains a ')' and a space; fields are counted after the LAST
    // ')': index 0 = state (field 3), so utime (field 14) = index 11 and stime
    // (field 15) = index 12. Here utime=200, stime=55.
    let stat = "1 (weird ) name) S 4 5 6 7 8 9 10 11 12 13 200 55 0 0";
    assert_eq!(parse_cpu_ticks(stat), Some(255));
}

#[test]
fn parse_vram_sums_across_gpus() {
    let csv = "1000, 24000\n2000, 24000\n";
    assert_eq!(parse_vram(csv), Some((3000, 48000)));
    assert_eq!(parse_vram(""), None);
    assert_eq!(parse_vram("garbage\n"), None);
}
