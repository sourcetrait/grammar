//! The controller's emergency policy: each logged, then sent to the session's MCP.
use crate::*;

/// Now, in milliseconds since the epoch: an emergency line's `ts`.
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| u64::try_from(since.as_millis()).unwrap_or(u64::MAX))
}

/// The controller's policy over the operator's emergencies: logged, then sent.
#[derive(Clone)]
pub(crate) struct EmergencyPolicy {
    asker: Arc<Asker>,
    nom: String,
    log: PathBuf,
    /// Held across each append, shared by every clone, so records never interleave.
    appending: Arc<std::sync::Mutex<()>>,
}

impl EmergencyPolicy {
    /// The policy under the engine's `nom`, logging to `log`, sending by `asker`.
    pub(crate) fn new(
        asker: Arc<Asker>,
        nom: &str,
        log: PathBuf,
    ) -> Self {
        Self {
            asker,
            nom: nom.to_string(),
            log,
            appending: Arc::new(std::sync::Mutex::new(())),
        }
    }

    /// An emergency logged, then sent to the session's MCP to log and announce.
    pub(crate) fn respond(
        &self,
        emergency: Emergency,
    ) {
        self.append(&emergency_line(&emergency, now_millis(), &self.nom, None));
        self.asker.emergency(emergency);
    }

    /// One record appended whole, the clones' appends taken one at a time.
    fn append(
        &self,
        line: &str,
    ) {
        let _appending = held(&self.appending);
        if let Err(e) = append_line(&self.log, line) {
            eprintln!("grammar: emergency log append failed at {}: {e}", self.log.display());
        }
    }
}
