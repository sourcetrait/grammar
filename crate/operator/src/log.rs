//! The emergency log: each emergency one NUON line under a host's cache home.
use crate::*;

/// A count as the log's int, held at the int's ceiling past it.
fn int(n: u64) -> NuVal {
    NuVal::Int(i64::try_from(n).unwrap_or(i64::MAX))
}

/// A field's text as the log's string.
fn text(s: &str) -> NuVal {
    NuVal::String(s.to_string())
}

/// An emergency's own fields, without the envelope the log adds.
fn emergency_fields(emergency: &Emergency) -> Vec<(&'static str, NuVal)> {
    match emergency {
        Emergency::HungEngineThread(h) => vec![
            ("nonce", text(&h.nonce)),
            ("lane", text(&h.lane)),
            ("tool", text(&h.tool)),
            ("started_at", int(h.started_at)),
            ("cancelled_at", int(h.cancelled_at)),
            ("hung_ms", int(h.hung_ms)),
            ("pool_held", int(h.pool_held)),
            ("pool_cap", int(h.pool_cap)),
        ],
        Emergency::CpuWarning(c) => vec![("cpu_pct", NuVal::Float(c.cpu_pct)), ("sample_ms", int(c.sample_ms))],
        Emergency::RamWarning(m) => vec![("rss_kb", int(m.rss_kb))],
        Emergency::VramWarning(v) => vec![("used_mib", int(v.used_mib)), ("total_mib", int(v.total_mib))],
        Emergency::BackgroundJobsWarning(b) => vec![("job_count", int(b.job_count))],
        Emergency::DiskWarning(d) => vec![
            ("mount", text(&d.mount)),
            ("used_pct", int(u64::from(d.used_pct))),
            ("threshold_pct", int(u64::from(d.threshold_pct))),
        ],
        Emergency::ChannelSpamWarning(w) => vec![
            ("origin", text(&w.origin)),
            ("hits", int(u64::from(w.hits))),
            ("window_secs", int(w.window_secs)),
            ("rate", int(u64::from(w.rate))),
        ],
        Emergency::ChannelSpamError(e) => vec![
            ("origin", text(&e.origin)),
            ("hits", int(u64::from(e.hits))),
            ("window_secs", int(e.window_secs)),
            ("rate", int(u64::from(e.rate))),
            ("action", text(&e.action)),
        ],
        Emergency::Critical(c) => vec![("reason", text(&c.reason)), ("hung", int(c.hung)), ("cap", int(c.cap))],
    }
}

/// An emergency's fields as announced, a remote engine's led by its `host`.
fn announced_fields(
    emergency: &Emergency,
    host: Option<&str>,
) -> Vec<(String, NuVal)> {
    let host = host.map(|host| ("host".to_string(), text(host)));
    let fields = emergency_fields(emergency).into_iter().map(|(key, value)| (key.to_string(), value));
    host.into_iter().chain(fields).collect()
}

/// An emergency's record as announced, a remote engine's led by its `host`.
pub fn announced_record(
    emergency: &Emergency,
    host: Option<&str>,
) -> NuVal {
    NuVal::Record(announced_fields(emergency, host))
}

/// An emergency as the log's one-line NUON record, a remote one naming its host.
pub fn emergency_line(
    emergency: &Emergency,
    ts: u64,
    nom: &str,
    host: Option<&str>,
) -> String {
    let envelope = [
        ("ts".to_string(), int(ts)),
        ("mcp_nom".to_string(), text(nom)),
        ("kind".to_string(), text(emergency.kind().name())),
    ];
    let record = NuVal::Record(envelope.into_iter().chain(announced_fields(emergency, host)).collect());
    record.to_nuon(NuonStyle::Default).replace('\n', "\\n").replace('\r', "\\r")
}

/// A host's emergency log under its namespace's cache home, by its nom.
pub fn emergency_log_path(
    cache_home: &Path,
    nom: &str,
) -> PathBuf {
    cache_home.join("log").join(nom).join("emergency.nuonl")
}

/// One rendered line and its newline appended in one write, its directory made.
pub fn append_line(
    path: &Path,
    line: &str,
) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut record = String::with_capacity(line.len() + 1);
    record.push_str(line);
    record.push('\n');
    let mut file = fs::OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(record.as_bytes())
}
