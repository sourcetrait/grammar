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
        self.append(&emergency_line(&emergency, now_millis(), &self.mcp_nom, host));
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
        let emit = EmitRequest {
            from: FROM_MCP.to_string(),
            model: emergency_model(emergency).to_string(),
            event: announced_record(emergency, host),
            attached: None,
        };
        let _ = self.channel.request(emit).await;
    }
}
