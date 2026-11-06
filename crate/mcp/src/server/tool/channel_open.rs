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

/// A Codex client's async open before any `interface info` named its thread.
#[cfg(unix)]
const CODEX_NO_THREAD: &str = "Codex's Channel delivers into the thread `interface info` names; \
                               run `interface info` first";

/// A Codex client's async open on a platform its face can't reach Codex on.
#[cfg(not(unix))]
const CODEX_NEEDS_UNIX: &str = "Codex's Channel rides its app-server's Unix socket, which this \
                                platform lacks; open the channel sync";

/// The face an async open takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenFace {
    /// The wss hub a Monitor connects to.
    Wss,
    /// The Codex face, delivering into the caller's thread.
    #[cfg(unix)]
    Codex,
}

/// What a started Codex face leaves to serve once its hub is installed.
#[cfg(unix)]
type CodexStart = CodexSession;
#[cfg(not(unix))]
type CodexStart = std::convert::Infallible;

/// The host's `channel/Open` packet, rendered with its id and its event's NUON.
fn greeting(
    nonce_gen: &datum::NonceGenerator,
    mcp_nom: &datum::NomPair,
) -> Result<HostRequest, GrammarMcpError> {
    open_packet(nonce_gen, mcp_nom).map_err(|reason| GrammarMcpError::Internal {
        phase: "channel_open::render".to_string(),
        reason,
    })
}

/// What an open came to: its label, its epoch, its endpoint, and its inbox.
type OpenedChannel = (&'static str, ChannelEpoch, Option<String>, PathBuf);

impl ChannelFace {
    /// `interface channel open`: the Channel opened in its mode, its face the caller's.
    pub(crate) async fn channel_open(
        &self,
        nonce: datum::Nonce,
        p: ChannelOpenParams,
        caller: &Caller,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let _opening = self.channel_open_lock.lock().await;
        let (label, epoch, wss, inbox) = match self.open_or_report(p.mode, caller).await {
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
        caller: &Caller,
    ) -> Result<OpenedChannel, GrammarMcpError> {
        let status = self.status().await?;
        if matches!(status.state.phase, ChannelPhase::Closed) {
            let face = match mode {
                ChannelMode::Async => Some(self.open_face(caller)?),
                ChannelMode::Sync => None,
            };
            let inbox = make_channel_dir().map_err(|reason| GrammarMcpError::ChannelStart { reason })?;
            let epoch = self.hub.next_epoch();
            let wss = self.start_open(epoch, mode, &inbox, face).await?;
            return Ok((STATUS_NEW, epoch, wss, inbox));
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
            ChannelMode::Async => self
                .hub
                .face(epoch)
                .ok_or_else(|| GrammarMcpError::Internal {
                    phase: "channel_open::face".to_string(),
                    reason: "an open channel carries no hub".to_string(),
                })?
                .wss(),
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

    /// The async face for a caller: Codex's for a Codex client, else the wss hub.
    fn open_face(
        &self,
        caller: &Caller,
    ) -> Result<OpenFace, GrammarMcpError> {
        if !matches!(caller.client.as_deref().map(Harness::of), Some(Harness::Codex)) {
            return Ok(OpenFace::Wss);
        }
        self.codex_face()
    }

    /// The Codex face, once `interface info` has named the thread it delivers to.
    #[cfg(unix)]
    fn codex_face(&self) -> Result<OpenFace, GrammarMcpError> {
        if self.codex_thread().is_none() {
            return Err(GrammarMcpError::ChannelStart {
                reason: CODEX_NO_THREAD.to_string(),
            });
        }
        Ok(OpenFace::Codex)
    }

    /// No Codex face off Unix, its app-server's socket a Unix socket.
    #[cfg(not(unix))]
    fn codex_face(&self) -> Result<OpenFace, GrammarMcpError> {
        Err(GrammarMcpError::ChannelStart {
            reason: CODEX_NEEDS_UNIX.to_string(),
        })
    }

    /// A started Codex face's claim, greeting, and deliveries, its hub installed.
    #[cfg(unix)]
    async fn serve_codex(
        &self,
        session: CodexStart,
        epoch: ChannelEpoch,
    ) {
        let thread = self.codex_thread.clone();
        session
            .serve(self.channel.clone(), epoch, &self.mcp_nom, &self.nonce_gen, thread)
            .await;
    }

    /// No Codex face starts off Unix.
    #[cfg(not(unix))]
    async fn serve_codex(
        &self,
        session: CodexStart,
        _epoch: ChannelEpoch,
    ) {
        match session {}
    }

    /// Open the Channel on `inbox`, then its face's hub; a hub that fails closes it.
    async fn start_open(
        &self,
        epoch: ChannelEpoch,
        mode: ChannelMode,
        inbox: &Path,
        face: Option<OpenFace>,
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
        let Some(face) = face else {
            return Ok(None);
        };
        let started: Result<(Hub, Option<CodexStart>), GrammarMcpError> = match face {
            OpenFace::Wss => start_channel_hub(
                &self.config,
                self.channel.clone(),
                epoch,
                &self.mcp_nom,
                self.nonce_gen.clone(),
            )
            .await
            .map(|hub| (hub, None)),
            #[cfg(unix)]
            OpenFace::Codex => start_codex_face(epoch).await.map(|(hub, session)| (hub, Some(session))),
        };
        match started {
            Ok((hub, session)) => {
                let face = hub.face.clone();
                if self.hub.install(hub).is_err() {
                    self.tell_inbox(None).await;
                    return Err(GrammarMcpError::ChannelStart {
                        reason: CLOSED_DURING_START.to_string(),
                    });
                }
                if let Some(session) = session {
                    self.serve_codex(session, epoch).await;
                }
                Ok(face.wss())
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
        let greeting = greeting(&self.nonce_gen, &self.mcp_nom)?;
        match self.channel.request(greeting).await.map_err(channel_failed)? {
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
