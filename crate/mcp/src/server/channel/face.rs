//! The Channel's and the remote channels' commands, served by either server.
use crate::*;

/// What a planned close carries: the code, the reason, and an optional ack.
pub(crate) type CloseSignal = (u16, String, Option<tk::oneshot::Sender<()>>);

/// The Channel's face to a server: its systems, hub, open lock, and identity.
pub(crate) struct ChannelFace {
    /// The runtime config, which each open's hub binds and presents by.
    pub(crate) config: Arc<Config>,
    /// The Channel's system, which this face started and holds.
    pub(crate) channel: Arc<ChannelHandle>,
    /// The peer links' system, which this face started and holds.
    pub(crate) peer: Arc<PeerHandle>,
    /// The open's async hub and what the face's pump and timer share with it.
    pub(crate) hub: Arc<HubSlot>,
    /// Serializes `channel_open`'s decide-then-start across its await.
    pub(crate) channel_open_lock: Arc<tk::AsyncMutex<()>>,
    pub(crate) nonce_gen: Arc<datum::NonceGenerator>,
    /// This host process's identity, minted once at startup.
    pub(crate) mcp_nom: datum::NomPair,
}

impl ChannelFace {
    /// Starts the Channel's and the peer links' systems, the face their controller.
    pub(crate) async fn start(
        config: Arc<Config>,
        nonce_gen: Arc<datum::NonceGenerator>,
        mcp_nom: datum::NomPair,
    ) -> Result<Self, String> {
        let hub = Arc::new(HubSlot::default());
        let params = ChannelSysParams {
            spam: config.channel.spam.data(),
        };
        let pump = HubPump { hub: hub.clone() };
        let paths = ChannelSysPaths::new(config.data_home(), config.cache_home());
        let channel = Arc::new(ChannelHandle::start(paths, params, pump).await?);
        let params = peer_params(&config, &mcp_nom);
        let pump = PeerPump {
            channel: channel.clone(),
        };
        let paths = PeerSysPaths::new(config.data_home(), config.cache_home());
        let peer = Arc::new(PeerHandle::start(paths, params, pump).await?);
        Ok(Self {
            config,
            channel,
            peer,
            hub,
            channel_open_lock: Arc::new(tk::AsyncMutex::new(())),
            nonce_gen,
            mcp_nom,
        })
    }

    /// What a server's end and its signal sweep close the Channel through.
    pub(crate) fn closer(&self) -> ChannelCloser {
        ChannelCloser {
            channel: self.channel.clone(),
            hub: self.hub.clone(),
        }
    }

    /// The Channel's state, or the internal error its system's end is.
    pub(crate) async fn status(&self) -> GrammarMcpResult<ChannelStatus> {
        self.channel.request(StatusRequest).await.map_err(channel_failed)
    }
}

/// The internal error a Channel request's failed system is.
pub(crate) fn channel_failed(reason: String) -> GrammarMcpError {
    GrammarMcpError::Internal {
        phase: "channel".to_string(),
        reason,
    }
}

/// The internal error a peer links request's failed system is.
pub(crate) fn peer_failed(reason: String) -> GrammarMcpError {
    GrammarMcpError::Internal {
        phase: "remote".to_string(),
        reason,
    }
}

/// The peer links' parameters: the host's nom, its key pair, and its remotes file.
fn peer_params(
    cfg: &Config,
    mcp_nom: &datum::NomPair,
) -> PeerSysParams {
    let path = |file: &Path| file.to_string_lossy().into_owned();
    PeerSysParams {
        mcp_nom: mcp_nom.as_str().to_string(),
        identity: cfg.identity.as_ref().map(|identity| IdentityKeys {
            cert: path(&identity.cert),
            key: path(&identity.key),
        }),
        config_file: AGENT_CONFIG_FILE.to_string(),
        remotes_file: path(&cfg.remotes_file),
        expand: expand_defaults(),
    }
}

/// The face's handler of its peer links: their relays and reports onto the Channel.
struct PeerPump {
    channel: Arc<ChannelHandle>,
}

impl subsys::Handler<PeerSystem> for PeerPump {
    fn on_packet(&mut self, pkt: subsys::Packet<FromPeerSys>) -> impl Future<Output = Option<subsys::Packet<FromPeerSys>>> + 'static + Send {
        let channel = self.channel.clone();
        async move {
            match pkt.take_msg() {
                FromPeerSys::Relay(relay) => relay_onto(&channel, relay).await,
                FromPeerSys::Report(report) => {
                    let emit = EmitRequest {
                        from: FROM_MCP.to_string(),
                        model: report.model().to_string(),
                        event: report.event(),
                        attached: None,
                    };
                    let _ = channel.request(emit).await;
                }
                _ => {}
            }
            None
        }
    }
}

/// A peer's message onto the Channel through its gate, a failure logged by its id.
async fn relay_onto(
    channel: &ChannelHandle,
    relay: Relay,
) {
    let attached = relay.attached();
    let Relay { packet, .. } = relay;
    let from = packet.packet_from();
    let emit = EmitRequest {
        from,
        model: packet.model,
        event: packet.event,
        attached,
    };
    let (kind, message) = match channel.request(emit).await {
        Ok(Ok(_)) => return,
        Ok(Err(EmitError::Refused(e))) => ("channel", e.message().to_string()),
        Ok(Err(EmitError::Render(e))) => ("render", e),
        Err(e) => ("channel", e),
    };
    eprintln!(
        "grammar: remote delivery `{}` not relayed onto the Channel: {kind}: {message}",
        packet.delivery,
    );
}

/// One open's async hub: its endpoint and the senders to its tasks.
pub(crate) struct Hub {
    pub(crate) epoch: ChannelEpoch,
    pub(crate) url: String,
    pub(crate) packets: tk::UnboundedSender<String>,
    pub(crate) close: tk::oneshot::Sender<CloseSignal>,
    /// Held for its drop, which ends the accept loop.
    pub(crate) _shutdown: tk::oneshot::Sender<()>,
}

/// The face's hub, its verify timer, and the emergency policy its pump applies.
#[derive(Default)]
pub(crate) struct HubSlot {
    hub: std::sync::Mutex<Option<Hub>>,
    verify: std::sync::Mutex<Option<(ChannelEpoch, tk::oneshot::Sender<()>)>>,
    policy: std::sync::Mutex<Option<EmergencyPolicy>>,
    epochs: AtomicU64,
    /// The highest epoch closed, under the hub's lock: a later hub of it is refused.
    closed_through: AtomicU64,
}

/// A slot's lock, a poisoned one taken as it stands.
fn held<T>(slot: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    slot.lock().unwrap_or_else(|e| e.into_inner())
}

impl HubSlot {
    /// A fresh epoch for the next open.
    pub(crate) fn next_epoch(&self) -> ChannelEpoch {
        ChannelEpoch(self.epochs.fetch_add(1, Ordering::SeqCst) + 1)
    }

    /// Install an open's hub, or hand it back when its open has closed meanwhile.
    pub(crate) fn install(
        &self,
        hub: Hub,
    ) -> Result<(), Hub> {
        let mut slot = held(&self.hub);
        if hub.epoch.0 <= self.closed_through.load(Ordering::SeqCst) {
            return Err(hub);
        }
        *slot = Some(hub);
        Ok(())
    }

    /// The endpoint of the open of `epoch`'s hub.
    pub(crate) fn url(
        &self,
        epoch: ChannelEpoch,
    ) -> Option<String> {
        held(&self.hub)
            .as_ref()
            .filter(|hub| hub.epoch == epoch)
            .map(|hub| hub.url.clone())
    }

    /// Whether the open of `epoch`'s hub has lost its peer, or has no hub.
    pub(crate) fn peer_gone(
        &self,
        epoch: ChannelEpoch,
    ) -> bool {
        match held(&self.hub).as_ref() {
            Some(hub) if hub.epoch == epoch => hub.packets.is_closed(),
            _ => true,
        }
    }

    /// A line into the open of `epoch`'s hub; a line for another is dropped.
    pub(crate) fn deliver(
        &self,
        epoch: ChannelEpoch,
        line: String,
    ) {
        if let Some(hub) = held(&self.hub).as_ref()
            && hub.epoch == epoch
        {
            let _ = hub.packets.send(line);
        }
    }

    /// The open of `epoch`'s hub closed with its frame, its timer stood down.
    pub(crate) fn close(
        &self,
        epoch: ChannelEpoch,
        code: u16,
        reason: String,
    ) {
        let taken = {
            let mut slot = held(&self.hub);
            self.closed_through.fetch_max(epoch.0, Ordering::SeqCst);
            match slot.as_ref() {
                Some(hub) if hub.epoch == epoch => slot.take(),
                _ => None,
            }
        };
        if let Some(hub) = taken {
            let _ = hub.close.send((code, reason, None));
        }
        self.disarm_verify(epoch);
    }

    /// Close the current hub now, an ack for when its frame is on the wire.
    pub(crate) fn close_now(
        &self,
        code: u16,
        reason: &str,
    ) -> Option<tk::oneshot::Receiver<()>> {
        let hub = {
            let mut slot = held(&self.hub);
            let minted = self.epochs.load(Ordering::SeqCst);
            self.closed_through.fetch_max(minted, Ordering::SeqCst);
            slot.take()
        }?;
        let (done_tx, done_rx) = tk::oneshot::channel::<()>();
        let _ = hub.close.send((code, reason.to_string(), Some(done_tx)));
        Some(done_rx)
    }

    /// Arm the verify timer of `epoch`, standing any previous one down.
    pub(crate) fn arm_verify(
        &self,
        epoch: ChannelEpoch,
        cancel: tk::oneshot::Sender<()>,
    ) {
        *held(&self.verify) = Some((epoch, cancel));
    }

    /// Stand the verify timer of `epoch` down.
    pub(crate) fn disarm_verify(
        &self,
        epoch: ChannelEpoch,
    ) {
        let mut verify = held(&self.verify);
        if verify.as_ref().is_some_and(|(armed, _)| *armed == epoch) {
            *verify = None;
        }
    }

    /// Install the emergency policy the pump applies.
    pub(crate) fn install_policy(
        &self,
        policy: EmergencyPolicy,
    ) {
        *held(&self.policy) = Some(policy);
    }

    /// An emergency to the policy, on a task of its own, when one is installed.
    pub(crate) fn raise(
        &self,
        emergency: Emergency,
    ) {
        if let Some(policy) = held(&self.policy).clone() {
            tk::spawn(async move { policy.respond(emergency).await });
        }
    }
}

/// The face's handler of its Channel: lines into the hub, its frames, emergencies.
struct HubPump {
    hub: Arc<HubSlot>,
}

impl subsys::Handler<ChannelSystem> for HubPump {
    fn on_packet(&mut self, pkt: subsys::Packet<FromChannelSys>) -> impl Future<Output = Option<subsys::Packet<FromChannelSys>>> + 'static + Send {
        match pkt.take_msg() {
            FromChannelSys::Line(Line { epoch, line }) => self.hub.deliver(epoch, line),
            FromChannelSys::CloseHub(CloseHub {
                epoch,
                code,
                reason,
            }) => self.hub.close(epoch, code, reason),
            FromChannelSys::Emergency(emergency) => self.hub.raise(emergency),
            _ => {}
        }
        std::future::ready(None)
    }
}

/// RFC 6455 "going away" - a server shutting down, which is exactly this.
const CLOSE_GOING_AWAY: u16 = 1001;
const SHUTDOWN_REASON: &str = "host shutting down";

/// How long a shutdown waits for the close frame to reach the wire.
const CLOSE_FLUSH_GRACE: tk::TkDuration = tk::TkDuration::from_millis(500);

/// What a server's end closes its Channel through, its peer told first.
#[derive(Clone)]
pub(crate) struct ChannelCloser {
    channel: Arc<ChannelHandle>,
    hub: Arc<HubSlot>,
}

impl ChannelCloser {
    /// Tell the Channel's peer we are going away, and wait for the flush.
    pub(crate) async fn close_for_shutdown(&self) {
        let done = self.hub.close_now(CLOSE_GOING_AWAY, SHUTDOWN_REASON);
        let close = CloseRequest {
            code: CLOSE_GOING_AWAY,
            reason: SHUTDOWN_REASON.to_string(),
            prune: false,
        };
        let _ = tk::timeout(CLOSE_FLUSH_GRACE, self.channel.request(close)).await;
        if let Some(done) = done {
            let _ = tk::timeout(CLOSE_FLUSH_GRACE, done).await;
        }
    }
}

/// Sweep on TERM, INT, or HUP, then exit with the conventional `128 + signo`.
#[cfg(unix)]
pub(crate) fn spawn_signal_sweep<F, Fut>(shutdown: F) -> Option<tk::JoinHandle<()>>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send,
{
    let sweep = tk::spawn(async move {
        let mut term = match tk::signal(tk::SignalKind::terminate()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("grammar: SIGTERM handler not installed: {e}");
                return;
            }
        };
        let mut interrupt = match tk::signal(tk::SignalKind::interrupt()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("grammar: SIGINT handler not installed: {e}");
                return;
            }
        };
        let mut hangup = match tk::signal(tk::SignalKind::hangup()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("grammar: SIGHUP handler not installed: {e}");
                return;
            }
        };
        let signo = tokio::select! {
            _ = term.recv() => tk::SignalKind::terminate().as_raw_value(),
            _ = interrupt.recv() => tk::SignalKind::interrupt().as_raw_value(),
            _ = hangup.recv() => tk::SignalKind::hangup().as_raw_value(),
        };
        shutdown().await;
        remove_scratch_dirs();
        process::exit(128 + signo);
    });
    Some(sweep)
}

/// Off Unix no signal is swept; the ordinary end closes and cleans up.
#[cfg(not(unix))]
pub(crate) fn spawn_signal_sweep<F, Fut>(_shutdown: F) -> Option<tk::JoinHandle<()>>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send,
{
    None
}
