//! The Channel's Codex face: each line delivered into a thread by the app-server.
use super::face::ThreadSlot;
use crate::*;

/// How long the app-server has for the handshake, `initialize`, and each delivery.
const APP_SERVER_WAIT: tk::TkDuration = tk::TkDuration::from_secs(10);

/// The name a delivered packet's tool output carries, under the server's namespace.
const DELIVERY_NAME: &str = "channel";

/// An open's session with the app-server, initialized, and the lanes its hub feeds.
pub(crate) struct CodexSession {
    client: AppServerClient,
    packets: tk::UnboundedReceiver<String>,
    close: tk::oneshot::Receiver<ChannelCloseSignal>,
}

/// The app-server on its daemon's socket, initialized, as `epoch`'s open's hub.
pub(crate) async fn start(epoch: ChannelEpoch) -> Result<(Hub, CodexSession), GrammarMcpError> {
    let status = blocking(daemon_status).await?;
    let socket = status.socket;
    let client = blocking(move || {
        AppServerClient::connect(&socket, consts::GRAMMAR, env!("CARGO_PKG_VERSION"), APP_SERVER_WAIT)
            .map_err(|reason| format!("Codex's app-server: {reason}"))
    })
    .await?;
    let (packet_tx, packets) = tk::unbounded_channel::<String>();
    let (close_tx, close) = tk::oneshot::channel::<ChannelCloseSignal>();
    let hub = Hub {
        epoch,
        face: HubFace::Codex,
        packets: packet_tx,
        close: close_tx,
        _shutdown: None,
    };
    Ok((hub, CodexSession { client, packets, close }))
}

/// A blocking step on the pool, its refusal the open's.
async fn blocking<T: Send + 'static>(
    step: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, GrammarMcpError> {
    match tk::spawn_blocking(step).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(reason)) => Err(GrammarMcpError::ChannelStart { reason }),
        Err(e) => Err(GrammarMcpError::Internal {
            phase: "channel_open::codex".to_string(),
            reason: e.to_string(),
        }),
    }
}

impl CodexSession {
    /// The claim and the greeting, then each line delivered on a task of its own.
    pub(crate) async fn serve(
        self,
        channel: Arc<ChannelHandle>,
        epoch: ChannelEpoch,
        mcp_nom: &datum::NomPair,
        nonce_gen: &datum::NonceGenerator,
        thread: ThreadSlot,
    ) {
        let _ = channel.notify(HubNotice::Claimed(epoch)).await;
        if let Ok(greeting) = open_packet(nonce_gen, mcp_nom) {
            let _ = channel.request(greeting).await;
        }
        tk::spawn(self.deliver(channel, epoch, thread));
    }

    /// Each line in order until a close, a refused delivery, or the session's loss.
    async fn deliver(
        self,
        channel: Arc<ChannelHandle>,
        epoch: ChannelEpoch,
        thread: ThreadSlot,
    ) {
        let Self {
            client,
            mut packets,
            mut close,
        } = self;
        let mut client = Some(client);
        loop {
            tokio::select! {
                biased;

                closing = &mut close => {
                    let done = closing.ok().and_then(|(_, _, done)| done);
                    if let Some(client) = client.take() {
                        let _ = tk::spawn_blocking(move || client.close()).await;
                    }
                    if let Some(done) = done {
                        let _ = done.send(());
                    }
                    break;
                }
                line = packets.recv() => {
                    let (Some(line), Some(session)) = (line, client.take()) else {
                        break;
                    };
                    match deliver_one(session, &thread, line).await {
                        Ok(session) => client = Some(session),
                        Err(reason) => {
                            eprintln!("grammar: a Codex delivery failed, the face's peer gone: {reason}");
                            break;
                        }
                    }
                }
            }
        }
        drop(packets);
        let _ = channel.notify(HubNotice::PeerGone(epoch)).await;
    }
}

/// One line as `turn/start` on the thread kept now, the session handed back.
async fn deliver_one(
    mut client: AppServerClient,
    thread: &ThreadSlot,
    line: String,
) -> Result<AppServerClient, String> {
    let to = thread
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .ok_or_else(|| "no thread is kept to deliver to".to_string())?;
    tk::spawn_blocking(move || {
        let output = ToolOutput {
            name: DELIVERY_NAME,
            namespace: consts::GRAMMAR,
            output: &line,
        };
        client.turn_start(&to, &output).map(|()| client)
    })
    .await
    .map_err(|e| e.to_string())?
}
