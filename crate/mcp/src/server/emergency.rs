//! The controller's emergency policy: each logged, then announced on the Channel.
use crate::*;

/// The reserved model path an emergency rides under on the channel.
pub(crate) fn emergency_model(emergency: &Emergency) -> &'static str {
    match emergency.kind() {
        EmergencyKind::HungEngineThread => "mcp/supervisor/HungEngineThread",
        EmergencyKind::CpuWarning => "mcp/supervisor/CpuWarning",
        EmergencyKind::RamWarning => "mcp/supervisor/RamWarning",
        EmergencyKind::VramWarning => "mcp/supervisor/VramWarning",
        EmergencyKind::BackgroundJobsWarning => "mcp/supervisor/BackgroundJobsWarning",
        EmergencyKind::DiskWarning => "mcp/supervisor/DiskWarning",
        EmergencyKind::ChannelSpamWarning => "mcp/channel/spam/Warning",
        EmergencyKind::ChannelSpamError => "mcp/channel/spam/Error",
        EmergencyKind::Critical => "mcp/supervisor/Critical",
    }
}

/// An emergency's own fields, without the envelope the log adds.
fn emergency_record(emergency: &Emergency) -> nu::Record {
    let span = nu::Span::unknown();
    let mut r = nu::Record::new();
    match emergency {
        Emergency::HungEngineThread(h) => {
            r.insert("nonce", nu::Value::string(h.nonce.clone(), span));
            r.insert("lane", nu::Value::string(h.lane.clone(), span));
            r.insert("tool", nu::Value::string(h.tool.clone(), span));
            r.insert("started_at", nu::Value::int(h.started_at as i64, span));
            r.insert("cancelled_at", nu::Value::int(h.cancelled_at as i64, span));
            r.insert("hung_ms", nu::Value::int(h.hung_ms as i64, span));
            r.insert("pool_held", nu::Value::int(h.pool_held as i64, span));
            r.insert("pool_cap", nu::Value::int(h.pool_cap as i64, span));
        }
        Emergency::CpuWarning(c) => {
            r.insert("cpu_pct", nu::Value::float(c.cpu_pct, span));
            r.insert("sample_ms", nu::Value::int(c.sample_ms as i64, span));
        }
        Emergency::RamWarning(m) => {
            r.insert("rss_kb", nu::Value::int(m.rss_kb as i64, span));
        }
        Emergency::VramWarning(v) => {
            r.insert("used_mib", nu::Value::int(v.used_mib as i64, span));
            r.insert("total_mib", nu::Value::int(v.total_mib as i64, span));
        }
        Emergency::BackgroundJobsWarning(b) => {
            r.insert("job_count", nu::Value::int(b.job_count as i64, span));
        }
        Emergency::DiskWarning(d) => {
            r.insert("mount", nu::Value::string(d.mount.clone(), span));
            r.insert("used_pct", nu::Value::int(d.used_pct as i64, span));
            r.insert("threshold_pct", nu::Value::int(d.threshold_pct as i64, span));
        }
        Emergency::ChannelSpamWarning(w) => {
            r.insert("origin", nu::Value::string(w.origin.clone(), span));
            r.insert("hits", nu::Value::int(w.hits as i64, span));
            r.insert("window_secs", nu::Value::int(w.window_secs as i64, span));
            r.insert("rate", nu::Value::int(w.rate as i64, span));
        }
        Emergency::ChannelSpamError(e) => {
            r.insert("origin", nu::Value::string(e.origin.clone(), span));
            r.insert("hits", nu::Value::int(e.hits as i64, span));
            r.insert("window_secs", nu::Value::int(e.window_secs as i64, span));
            r.insert("rate", nu::Value::int(e.rate as i64, span));
            r.insert("action", nu::Value::string(e.action.clone(), span));
        }
        Emergency::Critical(c) => {
            r.insert("reason", nu::Value::string(c.reason.clone(), span));
            r.insert("hung", nu::Value::int(c.hung as i64, span));
            r.insert("cap", nu::Value::int(c.cap as i64, span));
        }
    }
    r
}

/// An emergency's fields as announced, a remote engine's led by its `host`.
fn announced_record(
    emergency: &Emergency,
    host: Option<&str>,
) -> nu::Record {
    let span = nu::Span::unknown();
    let mut r = nu::Record::new();
    if let Some(host) = host {
        r.insert("host", nu::Value::string(host.to_string(), span));
    }
    for (key, value) in emergency_record(emergency) {
        r.insert(key, value);
    }
    r
}

/// An emergency as the log's one-line NUON record, a remote one naming its host.
pub(crate) fn emergency_line(
    emergency: &Emergency,
    ts: u64,
    mcp_nom: &str,
    host: Option<&str>,
) -> Result<String, String> {
    let span = nu::Span::unknown();
    let mut r = nu::Record::new();
    r.insert("ts", nu::Value::int(ts as i64, span));
    r.insert("mcp_nom", nu::Value::string(mcp_nom.to_string(), span));
    r.insert("kind", nu::Value::string(emergency.kind().name().to_string(), span));
    for (key, value) in announced_record(emergency, host) {
        r.insert(key, value);
    }
    nu::to_nuon(
        &nu::EngineState::new(),
        &nu::Value::record(r, span),
        nu::ToNuonConfig::default(),
    )
    .map(|line| escape_line(&line))
    .map_err(|e| e.to_string())
}

/// The per-process emergency log path under the namespace's cache home.
pub(crate) fn emergency_log_path(
    cache_home: &Path,
    mcp_nom: &str,
) -> PathBuf {
    cache_home.join("log").join(mcp_nom).join("emergency.nuonl")
}

/// Append one already-rendered NUON line, creating the parent dir on demand.
pub(crate) fn append_line(
    path: &std::path::Path,
    line: &str,
) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut record = String::with_capacity(line.len() + 1);
    record.push_str(line);
    record.push('\n');
    let mut f = fs::OpenOptions::new().create(true).append(true).open(path)?;
    f.write_all(record.as_bytes())
}

/// The controller's policy over its systems' emergencies: logged, then announced.
#[derive(Clone)]
pub(crate) struct EmergencyPolicy {
    channel: Arc<ChannelHandle>,
    mcp_nom: String,
    log: PathBuf,
    /// Held across each append, shared by every clone, so records never interleave.
    appending: Arc<std::sync::Mutex<()>>,
}

impl EmergencyPolicy {
    /// The policy of the host `mcp_nom`, logging to `log` and announcing on `channel`.
    pub(crate) fn new(
        channel: Arc<ChannelHandle>,
        mcp_nom: &str,
        log: PathBuf,
    ) -> Self {
        Self {
            channel,
            mcp_nom: mcp_nom.to_string(),
            log,
            appending: Arc::new(std::sync::Mutex::new(())),
        }
    }

    /// An emergency logged, then announced on the Channel under its reserved model.
    pub(crate) async fn respond(
        &self,
        emergency: Emergency,
    ) {
        self.respond_from(None, emergency).await;
    }

    /// An emergency of the host `host` names, none the MCP's, logged then announced.
    pub(crate) async fn respond_from(
        &self,
        host: Option<&str>,
        emergency: Emergency,
    ) {
        match emergency_line(&emergency, now_millis(), &self.mcp_nom, host) {
            Ok(line) => self.append(&line),
            Err(e) => eprintln!("grammar: emergency serialize failed: {e}"),
        }
        self.announce(&emergency, host).await;
    }

    /// One record appended whole, the clones' appends taken one at a time.
    fn append(
        &self,
        line: &str,
    ) {
        let _appending = self.appending.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = append_line(&self.log, line) {
            eprintln!(
                "grammar: emergency log append failed at {}: {e}",
                self.log.display(),
            );
        }
    }

    /// Put one emergency on the Channel under its reserved `mcp/` model.
    async fn announce(
        &self,
        emergency: &Emergency,
        host: Option<&str>,
    ) {
        let event = nu::Value::record(announced_record(emergency, host), nu::Span::unknown());
        let Ok(event) = NuVal::try_from_nu(event) else {
            return;
        };
        let emit = EmitRequest {
            from: FROM_MCP.to_string(),
            model: emergency_model(emergency).to_string(),
            event,
            attached: None,
        };
        let _ = self.channel.request(emit).await;
    }
}
