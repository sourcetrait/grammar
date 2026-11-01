use crate::*;

/// The hub binds loopback, always, and this is not configurable.
const BIND: &str = "127.0.0.1";

/// The origin stamped on a host-originated packet.
pub(crate) const FROM_MCP: &str = "mcp";

/// The host's handshake model, under the `mcp/` reservation.
pub(crate) const MODEL_OPEN: &str = "mcp/channel/Open";

/// RFC 6455 "try again later" - the refusal code for a second claimant.
const CLOSE_CLAIMED: u16 = 1013;
const CLAIM_REASON: &str = "channel already claimed";

/// The host's `channel/Open` control packet - the thing the agent verifies.
pub(crate) fn open_packet(
    nonce_gen: &datum::NonceGenerator,
    mcp_nom: &datum::NomPair,
) -> Result<String, String> {
    let span = nu::Span::unknown();
    let mut data = nu::Record::new();
    data.insert("mcp_nom", nu::Value::string(mcp_nom.as_str().to_string(), span));
    let event = nu::Value::record(data, span);
    let event_nuon = render_nuon(&event)?;
    let id = mint_msg_id(nonce_gen, FROM_MCP, MODEL_OPEN, &event_nuon, None);
    render_packet(id, FROM_MCP, MODEL_OPEN, &event, None)
}

/// The hub's TLS: the host's `[identity]` certificate and key, which it presents.
fn server_config(config: &Config) -> Result<Arc<tls::ServerConfig>, GrammarMcpError> {
    let Some(identity) = config.identity.as_ref() else {
        return Err(GrammarMcpError::ChannelStart {
            reason: format!(
                "the Channel presents the host's [identity]; name its `cert` and \
                 `key` in {AGENT_CONFIG_FILE}"
            ),
        });
    };
    let leaf = tls::CertificateDer::from_pem_file(&identity.cert).map_err(|e| {
        GrammarMcpError::ChannelStart {
            reason: format!("read {}: {e}", identity.cert.display()),
        }
    })?;
    let key = tls::PrivateKeyDer::from_pem_file(&identity.key).map_err(|e| {
        GrammarMcpError::ChannelStart {
            reason: format!("read {}: {e}", identity.key.display()),
        }
    })?;
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = tls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .and_then(|b| b.with_no_client_auth().with_single_cert(vec![leaf], key))
        .map_err(|e| GrammarMcpError::ChannelStart {
            reason: format!("server config: {e}"),
        })?;
    Ok(Arc::new(config))
}

/// What the hub's peer task greets with and tells its Channel through.
#[derive(Clone)]
struct PeerContext {
    channel: Arc<ChannelHandle>,
    epoch: ChannelEpoch,
    mcp_nom: datum::NomPair,
    nonce_gen: Arc<datum::NonceGenerator>,
}

/// Bind the hub of `epoch`'s open on the config's port, presenting its identity.
pub(crate) async fn start(
    config: &Config,
    channel: Arc<ChannelHandle>,
    epoch: ChannelEpoch,
    mcp_nom: &datum::NomPair,
    nonce_gen: Arc<datum::NonceGenerator>,
) -> Result<Hub, GrammarMcpError> {
    let tls_config = server_config(config)?;
    let listener = tk::TcpListener::bind((BIND, config.channel.port.unwrap_or(0)))
        .await
        .map_err(|e| GrammarMcpError::ChannelStart {
            reason: format!("bind {BIND}: {e}"),
        })?;
    let port = listener
        .local_addr()
        .map_err(|e| GrammarMcpError::ChannelStart {
            reason: format!("local_addr: {e}"),
        })?
        .port();
    let url = format!("wss://{BIND}:{port}");
    let (packet_tx, packet_rx) = tk::unbounded_channel::<String>();
    let (close_tx, close_rx) = tk::oneshot::channel::<ChannelCloseSignal>();
    let (shutdown_tx, shutdown_rx) = tk::oneshot::channel::<()>();
    let peer = PeerContext {
        channel,
        epoch,
        mcp_nom: mcp_nom.clone(),
        nonce_gen,
    };
    tk::spawn(accept_loop(
        listener,
        tls::TlsAcceptor::from(tls_config),
        (packet_rx, close_rx),
        shutdown_rx,
        peer,
    ));
    Ok(Hub {
        epoch,
        url,
        packets: packet_tx,
        close: close_tx,
        _shutdown: shutdown_tx,
    })
}

/// The first connection claims the hub, its Channel told; each later is refused.
async fn accept_loop(
    listener: tk::TcpListener,
    acceptor: tls::TlsAcceptor,
    lanes: (tk::UnboundedReceiver<String>, tk::oneshot::Receiver<ChannelCloseSignal>),
    shutdown_rx: tk::oneshot::Receiver<()>,
    peer: PeerContext,
) {
    let mut peer_slot = Some(lanes);
    let mut shutdown_rx = shutdown_rx;
    loop {
        tokio::select! {
            _ = &mut shutdown_rx => break,
            accepted = listener.accept() => {
                let Ok((tcp, _peer)) = accepted else { continue };
                let acceptor = acceptor.clone();
                match peer_slot.take() {
                    Some((packet_rx, close_rx)) => {
                        let _ = peer.channel.notify(HubNotice::Claimed(peer.epoch)).await;
                        tk::spawn(serve_peer(tcp, acceptor, packet_rx, close_rx, peer.clone()));
                    }
                    None => {
                        tk::spawn(refuse_peer(tcp, acceptor));
                    }
                }
            }
        }
    }
}

/// Serve the one claiming peer for the channel's life, its end told after.
async fn serve_peer(
    tcp: tk::TcpStream,
    acceptor: tls::TlsAcceptor,
    packets: tk::UnboundedReceiver<String>,
    close_rx: tk::oneshot::Receiver<ChannelCloseSignal>,
    peer: PeerContext,
) {
    serve(tcp, acceptor, packets, close_rx, &peer).await;
    let _ = peer.channel.notify(HubNotice::PeerGone(peer.epoch)).await;
}

/// The peer's greeting, then its packets until a close, its leaving, or a failure.
async fn serve(
    tcp: tk::TcpStream,
    acceptor: tls::TlsAcceptor,
    mut packets: tk::UnboundedReceiver<String>,
    close_rx: tk::oneshot::Receiver<ChannelCloseSignal>,
    peer: &PeerContext,
) {
    let Ok(tls_stream) = acceptor.accept(tcp).await else {
        return;
    };
    let Ok(mut socket) = ws::accept_async(tls_stream).await else {
        return;
    };

    if let Ok(line) = open_packet(&peer.nonce_gen, &peer.mcp_nom)
        && socket.send(ws::Message::text(line)).await.is_err()
    {
        return;
    }

    let mut close_rx = close_rx;
    loop {
        tokio::select! {
            biased;

            closing = &mut close_rx => {
                if let Ok((code, reason, done)) = closing {
                    let frame = ws::CloseFrame {
                        code: ws::CloseCode::from(code),
                        reason: reason.into(),
                    };
                    let _ = socket.send(ws::Message::Close(Some(frame))).await;
                    let _ = socket.flush().await;
                    if let Some(done) = done {
                        let _ = done.send(());
                    }
                }
                break;
            }
            line = packets.recv() => match line {
                Some(line) => {
                    if line.len() >= MAX_FRAME_BYTES {
                        eprintln!(
                            "grammar: channel packet of {} bytes refused (max {})",
                            line.len(),
                            MAX_FRAME_BYTES,
                        );
                        continue;
                    }
                    if socket.send(ws::Message::text(line)).await.is_err() {
                        break;
                    }
                }
                None => break,
            },
            incoming = socket.next() => match incoming {
                None | Some(Ok(ws::Message::Close(_))) | Some(Err(_)) => break,
                Some(Ok(_)) => {}
            },
        }
    }
}

/// Refuse a second claimant, saying why.
async fn refuse_peer(
    tcp: tk::TcpStream,
    acceptor: tls::TlsAcceptor,
) {
    let Ok(tls_stream) = acceptor.accept(tcp).await else {
        return;
    };
    let Ok(mut socket) = ws::accept_async(tls_stream).await else {
        return;
    };
    let frame = ws::CloseFrame {
        code: ws::CloseCode::from(CLOSE_CLAIMED),
        reason: CLAIM_REASON.to_string().into(),
    };
    let _ = socket.send(ws::Message::Close(Some(frame))).await;
    let _ = socket.flush().await;
}
