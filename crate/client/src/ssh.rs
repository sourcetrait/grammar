//! A worker over SSH: the connection, its exec channel, and the channel's streams.
use crate::*;

/// The worker's output the pump holds before the session reads it.
const PIPE_BYTES: usize = 256 * 1024;

/// The longest stderr line logged whole; a longer one is logged in pieces.
const LINE_MAX: usize = 8 * 1024;

/// The stderr lines kept to explain a worker that ends before its reply.
const TAIL_LINES: usize = 8;

/// How long a close waits for its frame to leave, and the channel to end after it.
const CLOSE_FLUSH: Duration = Duration::from_secs(2);

/// A session with a worker over SSH, and the connection it rides.
pub struct RemoteSession {
    session: Session,
    /// The connection, held so the session lasts and ended at its close.
    connection: tk::AsyncMutex<russh::client::Handle<Pinned>>,
    /// The pump of the channel's messages, which ends with the channel.
    pump: std::sync::Mutex<Option<tk::JoinHandle<()>>>,
}

impl RemoteSession {
    /// The worker this session reached.
    pub fn worker(&self) -> &WorkerHello {
        self.session.worker()
    }

    /// A request's response, or the session's loss where it ends first.
    pub async fn request(&self, request: EngineRequest) -> Result<EngineResponse, TransportError> {
        self.session.request(request).await
    }

    /// Whether the session has ended.
    pub fn has_ended(&self) -> bool {
        self.session.has_ended()
    }

    /// The session closed by the MCP: its close written, then the connection ended.
    pub async fn close(&self) {
        self.session.finish(CLOSE_FLUSH).await;
        let _ = self
            .connection
            .lock()
            .await
            .disconnect(russh::Disconnect::ByApplication, "the MCP closed the session", "en")
            .await;
        let pump = self.pump.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(pump) = pump {
            let _ = tk::timeout(CLOSE_FLUSH, pump).await;
        }
    }
}

/// A session on `endpoint`'s worker: the connection, the exec, and the hello.
pub async fn open(
    endpoint: &Endpoint,
    requested: Limits,
) -> Result<(RemoteSession, tk::UnboundedReceiver<SessionEvent>), TransportError> {
    let instance = endpoint.instance()?;
    let pinned = endpoint.pinned_keys().map_err(|reason| TransportError::HostKeyRejected { reason })?;
    let key = keys::load_secret_key(&endpoint.key, None).map_err(|e| TransportError::AuthenticationFailed {
        reason: format!("the key file {} couldn't be read: {e}", endpoint.key),
    })?;
    let (connection, channel) = match tk::timeout(AUTH_DEADLINE, connect(endpoint, pinned, key)).await {
        Ok(connected) => connected?,
        Err(_) => {
            return Err(TransportError::ConnectFailed {
                reason: format!(
                    "no session with {}:{} in {} s",
                    endpoint.host,
                    endpoint.port,
                    AUTH_DEADLINE.as_secs()
                ),
            });
        }
    };
    let (read, write) = channel.split();
    let (output, input) = tk::duplex(PIPE_BYTES);
    let witness = Arc::new(std::sync::Mutex::new(Witness::default()));
    let mut pump = tk::spawn(pump(endpoint.alias.clone(), read, input, witness.clone()));
    let hello = ClientHello::new(instance, requested);
    match Session::open(output, write.make_writer(), hello).await {
        Ok((session, events)) => Ok((
            RemoteSession {
                session,
                connection: tk::AsyncMutex::new(connection),
                pump: std::sync::Mutex::new(Some(pump)),
            },
            events,
        )),
        Err(refusal) => {
            let _ = connection
                .disconnect(russh::Disconnect::ByApplication, "the session didn't open", "en")
                .await;
            let _ = tk::timeout(CLOSE_FLUSH, &mut pump).await;
            let witness = witness.lock().unwrap_or_else(|e| e.into_inner());
            Err(witness.explain(refusal, &endpoint.command))
        }
    }
}

/// The client's algorithms: russh's own, less `ssh-rsa`'s SHA-1 host signatures.
fn preferred() -> russh::Preferred {
    let key: Vec<keys::Algorithm> = russh::Preferred::DEFAULT
        .key
        .iter()
        .filter(|algorithm| !matches!(algorithm, keys::Algorithm::Rsa { hash: None }))
        .cloned()
        .collect();
    russh::Preferred {
        key: Cow::Owned(key),
        ..russh::Preferred::DEFAULT
    }
}

/// The connection authenticated and the worker's command running on its channel.
async fn connect(
    endpoint: &Endpoint,
    pinned: Vec<keys::PublicKey>,
    key: keys::PrivateKey,
) -> Result<(russh::client::Handle<Pinned>, russh::Channel<russh::client::Msg>), TransportError> {
    let place = format!("{}:{}", endpoint.host, endpoint.port);
    let tcp = tk::TcpStream::connect((endpoint.host.as_str(), endpoint.port))
        .await
        .map_err(|e| TransportError::ConnectFailed { reason: format!("{place}: {e}") })?;
    let _ = tcp.set_nodelay(true);
    let rejected = Arc::new(std::sync::Mutex::new(None));
    let handler = Pinned {
        keys: pinned,
        rejected: rejected.clone(),
    };
    let config = Arc::new(russh::client::Config {
        preferred: preferred(),
        nodelay: true,
        ..Default::default()
    });
    let mut connection = match russh::client::connect_stream(config, tcp, handler).await {
        Ok(connection) => connection,
        Err(e) => {
            let rejected = rejected.lock().unwrap_or_else(|e| e.into_inner()).take();
            return Err(match rejected {
                Some(reason) => TransportError::HostKeyRejected { reason },
                None => TransportError::ConnectFailed {
                    reason: format!("the SSH handshake with {place}: {e}"),
                },
            });
        }
    };
    let hash = rsa_hash(&connection, &key).await?;
    let signer = keys::PrivateKeyWithHashAlg::new(Arc::new(key), hash);
    let auth = connection
        .authenticate_publickey(endpoint.user.clone(), signer)
        .await
        .map_err(|e| TransportError::AuthenticationFailed { reason: e.to_string() })?;
    if !auth.success() {
        return Err(TransportError::AuthenticationFailed {
            reason: format!("{place} refused the key for `{}`", endpoint.user),
        });
    }
    let channel = connection.channel_open_session().await.map_err(|e| TransportError::Unavailable {
        reason: format!("{place} opened no session channel: {e}"),
    })?;
    channel.exec(true, endpoint.command_line()).await.map_err(|e| TransportError::Unavailable {
        reason: format!("{place} took no command: {e}"),
    })?;
    Ok((connection, channel))
}

/// The hash an RSA key signs with, SHA-2 alone; none for any other key.
async fn rsa_hash(
    connection: &russh::client::Handle<Pinned>,
    key: &keys::PrivateKey,
) -> Result<Option<keys::HashAlg>, TransportError> {
    if !key.algorithm().is_rsa() {
        return Ok(None);
    }
    match connection.best_supported_rsa_hash().await {
        Ok(Some(Some(hash))) => Ok(Some(hash)),
        Ok(_) => Err(TransportError::AuthenticationFailed {
            reason: "the host names no SHA-2 RSA signature, and an RSA key signs with SHA-2 alone".to_string(),
        }),
        Err(e) => Err(TransportError::AuthenticationFailed { reason: e.to_string() }),
    }
}

/// The connection's handler: the host's key held to the pinned ones.
pub(crate) struct Pinned {
    keys: Vec<keys::PublicKey>,
    /// Why the host's key was refused, kept for the connect's error.
    rejected: Arc<std::sync::Mutex<Option<String>>>,
}

impl russh::client::Handler for Pinned {
    type Error = russh::Error;

    async fn check_server_key(&mut self, presented: &keys::PublicKeyOrCertificate) -> Result<bool, Self::Error> {
        let keys::PublicKeyOrCertificate::PublicKey { key, .. } = presented else {
            *self.rejected.lock().unwrap_or_else(|e| e.into_inner()) =
                Some("the host presented a certificate, and only plain keys are pinned".to_string());
            return Ok(false);
        };
        let matched = self.keys.iter().any(|pinned| pinned.key_data() == key.key_data());
        if !matched {
            *self.rejected.lock().unwrap_or_else(|e| e.into_inner()) =
                Some(format!("the host presented {}", key.fingerprint(keys::HashAlg::Sha256)));
        }
        Ok(matched)
    }
}

/// What the pump saw besides output: a refused command, an exit, the last lines.
#[derive(Debug, Default)]
pub(crate) struct Witness {
    pub(crate) refused: bool,
    pub(crate) exit: Option<String>,
    /// The exit's status, when the command exited rather than died by a signal.
    pub(crate) status: Option<u32>,
    pub(crate) tail: VecDeque<String>,
}

impl Witness {
    /// A line of the worker's stderr kept, the oldest past the tail dropped.
    pub(crate) fn keep(&mut self, line: String) {
        if self.tail.len() == TAIL_LINES {
            self.tail.pop_front();
        }
        self.tail.push_back(line);
    }

    /// The session's refusal, told as what the worker did where the pump saw it.
    pub(crate) fn explain(
        &self,
        refusal: TransportError,
        command: &str,
    ) -> TransportError {
        let TransportError::SessionLost { reason } = &refusal else {
            return refusal;
        };
        if self.not_found(command) {
            return TransportError::Unavailable {
                reason: format!("command not found: {command}"),
            };
        }
        let what = match (&self.exit, self.refused) {
            (_, true) => "the host refused to run the worker's command".to_string(),
            (Some(exit), false) => format!("the worker {exit} before its reply"),
            (None, false) => return refusal,
        };
        let tail = Vec::from_iter(self.tail.iter().cloned()).join(" | ");
        TransportError::Unavailable {
            reason: match tail.is_empty() {
                true => format!("{what}: {reason}"),
                false => format!("{what}: {reason}; its stderr: {tail}"),
            },
        }
    }

    /// Whether the login's shell found no `command`, by its status and words.
    fn not_found(&self, command: &str) -> bool {
        let said = |words: &str| self.tail.iter().any(|line| line.contains(words));
        let named_missing = || {
            self.tail.iter().any(|line| line.contains(command) && line.contains("No such file or directory"))
        };
        match self.status {
            Some(127) => said(command),
            Some(126) => named_missing(),
            Some(1) => said(&format!("Command `{command}` not found")) || said(&format!("`{command}` is neither a Nushell built-in")),
            _ => false,
        }
    }
}

/// The channel's messages: data into the session's reader, stderr into the log.
async fn pump(
    alias: String,
    mut read: russh::ChannelReadHalf,
    input: tk::DuplexStream,
    witness: Arc<std::sync::Mutex<Witness>>,
) {
    let mut input = Some(input);
    let mut line = Vec::new();
    while let Some(message) = read.wait().await {
        match message {
            russh::ChannelMsg::Data { data } => {
                if let Some(pipe) = input.as_mut()
                    && tk::AsyncWriteExt::write_all(pipe, &data).await.is_err()
                {
                    input = None;
                }
            }
            russh::ChannelMsg::ExtendedData { data, ext: 1 } => {
                line.extend_from_slice(&data);
                log_lines(&alias, &mut line, &witness, false);
            }
            russh::ChannelMsg::Eof => {
                if let Some(mut pipe) = input.take() {
                    let _ = tk::AsyncWriteExt::shutdown(&mut pipe).await;
                }
            }
            russh::ChannelMsg::Failure => {
                witness.lock().unwrap_or_else(|e| e.into_inner()).refused = true;
            }
            russh::ChannelMsg::ExitStatus { exit_status } => {
                let exit = format!("exited {exit_status}");
                eprintln!("grammar: {alias}: the worker {exit}");
                let mut witness = witness.lock().unwrap_or_else(|e| e.into_inner());
                witness.exit = Some(exit);
                witness.status = Some(exit_status);
            }
            russh::ChannelMsg::ExitSignal { signal_name, .. } => {
                let exit = format!("was killed by {signal_name:?}");
                eprintln!("grammar: {alias}: the worker {exit}");
                witness.lock().unwrap_or_else(|e| e.into_inner()).exit = Some(exit);
            }
            russh::ChannelMsg::Close => break,
            _ => {}
        }
    }
    log_lines(&alias, &mut line, &witness, true);
}

/// Each whole stderr line logged under the alias and kept for the tail.
fn log_lines(alias: &str, line: &mut Vec<u8>, witness: &std::sync::Mutex<Witness>, end: bool) {
    loop {
        let cut = match line.iter().position(|b| *b == b'\n') {
            Some(at) => at + 1,
            None if line.len() >= LINE_MAX => LINE_MAX,
            None if end && !line.is_empty() => line.len(),
            None => return,
        };
        let piece: Vec<u8> = line.drain(..cut).collect();
        let text = String::from_utf8_lossy(&piece).trim_end().to_string();
        eprintln!("grammar: {alias}: {text}");
        witness.lock().unwrap_or_else(|e| e.into_inner()).keep(text);
    }
}
