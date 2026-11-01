use crate::*;

/// Success result of `interface channel open`.
#[derive(Debug, ser::Serialize)]
pub(crate) struct ChannelOpenResult {
    /// `new` when this call opened the channel, `existing` when one was open.
    pub status: String,
    /// The mode the channel is open in.
    pub mode: ChannelMode,
    /// The endpoint to point a Monitor at, its port the bound one; null in sync.
    pub wss: Option<String>,
    /// The directory an attachment lands in.
    pub inbox: String,
}

const STATUS_NEW: &str = "new";
const STATUS_EXISTING: &str = "existing";

/// How long a peer has to prove it owns the stdio session.
const VERIFY_WINDOW: tk::TkDuration = tk::TkDuration::from_secs(300);

/// RFC 6455 policy violation - the peer never verified.
const CLOSE_UNVERIFIED: u16 = 1008;
const UNVERIFIED_REASON: &str = "verification window expired";

/// RFC 6455 internal error - the open's hub never started.
const CLOSE_UNSTARTED: u16 = 1011;
const UNSTARTED_REASON: &str = "the channel hub did not start";

/// An open whose Channel was closed while its hub started, the hub dropped unused.
const CLOSED_DURING_START: &str = "the channel was closed while its hub started";

/// The host's `channel/Open` packet, rendered.
fn greeting(
    nonce_gen: &datum::NonceGenerator,
    mcp_nom: &datum::NomPair,
) -> Result<String, GrammarMcpError> {
    open_packet(nonce_gen, mcp_nom).map_err(|reason| GrammarMcpError::Internal {
        phase: "channel_open::render".to_string(),
        reason,
    })
}

/// What an open came to: its label, its epoch, its endpoint, and its inbox.
type OpenedChannel = (&'static str, ChannelEpoch, Option<String>, PathBuf);

impl ChannelFace {
    /// `interface channel open`: open the packet channel in its mode.
    pub(crate) async fn channel_open(
        &self,
        nonce: datum::Nonce,
        p: ChannelOpenParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let _opening = self.channel_open_lock.lock().await;
        let (label, epoch, wss, inbox) = match self.open_or_report(p.mode).await {
            Ok(opened) => opened,
            Err(error) => return error_to_call_result(error, nonce),
        };
        self.arm_verify_timer(epoch);
        success(
            nonce,
            &ChannelOpenResult {
                status: label.to_string(),
                mode: p.mode,
                wss,
                inbox: inbox.display().to_string(),
            },
        )
    }

    /// Open a closed channel in `mode` on a new inbox, or report the open one.
    async fn open_or_report(
        &self,
        mode: ChannelMode,
    ) -> Result<OpenedChannel, GrammarMcpError> {
        let status = self.status().await?;
        if matches!(status.state.phase, ChannelPhase::Closed) {
            let inbox = make_channel_dir(self.mcp_nom.as_str())
                .map_err(|reason| GrammarMcpError::ChannelStart { reason })?;
            let epoch = self.hub.next_epoch();
            return match self.start_open(epoch, mode, &inbox).await {
                Ok(wss) => Ok((STATUS_NEW, epoch, wss, inbox)),
                Err(error) => {
                    remove_channel_dir(&inbox);
                    Err(error)
                }
            };
        }
        let open = status.state.mode.ok_or_else(|| GrammarMcpError::Internal {
            phase: "channel_open::mode".to_string(),
            reason: "an open channel carries no mode".to_string(),
        })?;
        if open != mode {
            return Err(GrammarMcpError::ChannelWrongMode { open, asked: mode });
        }
        let epoch = status.state.epoch;
        let wss = match open {
            ChannelMode::Async => Some(self.hub.url(epoch).ok_or_else(|| {
                GrammarMcpError::Internal {
                    phase: "channel_open::url".to_string(),
                    reason: "an open channel carries no url".to_string(),
                }
            })?),
            ChannelMode::Sync => None,
        };
        if status.state.claimed {
            self.resend_open_packet(open, epoch).await?;
        }
        let inbox = status.inbox.map(PathBuf::from).ok_or_else(|| GrammarMcpError::Internal {
            phase: "channel_open::inbox".to_string(),
            reason: "an open channel carries no inbox".to_string(),
        })?;
        Ok((STATUS_EXISTING, epoch, wss, inbox))
    }

    /// Open the Channel on `inbox`, then start its hub; a hub that fails closes it.
    async fn start_open(
        &self,
        epoch: ChannelEpoch,
        mode: ChannelMode,
        inbox: &Path,
    ) -> Result<Option<String>, GrammarMcpError> {
        let inbox = inbox
            .to_str()
            .ok_or_else(|| GrammarMcpError::ChannelStart {
                reason: format!("the inbox {} is not UTF-8", inbox.display()),
            })?
            .to_string();
        let greeting = match mode {
            ChannelMode::Sync => Some(greeting(&self.nonce_gen, &self.mcp_nom)?),
            ChannelMode::Async => None,
        };
        let open = OpenRequest {
            epoch,
            mode,
            inbox: inbox.clone(),
            greeting,
        };
        self.channel.request(open).await.map_err(channel_failed)?;
        self.tell_inbox(Some(inbox)).await;
        if mode == ChannelMode::Sync {
            return Ok(None);
        }
        let hub = start_channel_hub(
            &self.config,
            self.channel.clone(),
            epoch,
            &self.mcp_nom,
            self.nonce_gen.clone(),
        );
        match hub.await {
            Ok(hub) => {
                let url = hub.url.clone();
                if self.hub.install(hub).is_err() {
                    self.tell_inbox(None).await;
                    return Err(GrammarMcpError::ChannelStart {
                        reason: CLOSED_DURING_START.to_string(),
                    });
                }
                Ok(Some(url))
            }
            Err(error) => {
                let close = CloseRequest {
                    code: CLOSE_UNSTARTED,
                    reason: UNSTARTED_REASON.to_string(),
                    prune: true,
                };
                let _ = self.channel.request(close).await;
                self.tell_inbox(None).await;
                Err(error)
            }
        }
    }

    /// The peer links told where the Channel's inbox now is, once they hold it.
    pub(crate) async fn tell_inbox(
        &self,
        inbox: Option<String>,
    ) {
        let _ = self.peer.request(InboxRequest(inbox)).await;
    }

    /// Re-greet an existing channel's peer.
    async fn resend_open_packet(
        &self,
        mode: ChannelMode,
        epoch: ChannelEpoch,
    ) -> Result<(), GrammarMcpError> {
        if mode == ChannelMode::Async && self.hub.peer_gone(epoch) {
            return Err(GrammarMcpError::ChannelPeerGone);
        }
        let line = greeting(&self.nonce_gen, &self.mcp_nom)?;
        match self.channel.request(HostRequest { line }).await.map_err(channel_failed)? {
            Ok(()) => Ok(()),
            Err(ChannelSendError::NotOpen) => Err(GrammarMcpError::ChannelNotOpen),
            Err(_) => Err(GrammarMcpError::ChannelPeerGone),
        }
    }

    /// Start the verify window of the open of `epoch`.
    fn arm_verify_timer(
        &self,
        epoch: ChannelEpoch,
    ) {
        let (cancel_tx, cancel_rx) = tk::oneshot::channel::<()>();
        self.hub.arm_verify(epoch, cancel_tx);
        let channel = self.channel.clone();
        tk::spawn(async move {
            tokio::select! {
                _ = cancel_rx => {}
                _ = tk::sleep(VERIFY_WINDOW) => {
                    let expire = ExpireRequest {
                        epoch,
                        code: CLOSE_UNVERIFIED,
                        reason: UNVERIFIED_REASON.to_string(),
                    };
                    let _ = channel.request(expire).await;
                }
            }
        });
    }
}
