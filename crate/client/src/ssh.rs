//! grammar-ssh over SSH: the connection, its exec channel, and its streams.
use crate::*;

/// grammar-ssh's output the pump holds before the session reads it.
const PIPE_BYTES: usize = 256 * 1024;

/// The longest stderr line logged whole; a longer one is logged in pieces.
const LINE_MAX: usize = 8 * 1024;

/// The stderr lines kept to explain a grammar-ssh that ends before its reply.
const TAIL_LINES: usize = 8;

/// How long a close waits for its frame to leave, and the channel to end after it.
const CLOSE_FLUSH: Duration = Duration::from_secs(2);

/// A session with grammar-ssh over SSH, and the connection it rides.
pub struct RemoteSession {
    session: Session,
    /// The connection, held so the session lasts and ended at its close.
    connection: tk::AsyncMutex<russh::client::Handle<Pinned>>,
    /// The jump's connection the session rides through, ended after the session's.
    jump: Option<Through>,
    /// The pump of the channel's messages, which ends with the channel.
    pump: std::sync::Mutex<Option<tk::JoinHandle<()>>>,
    /// What the pump saw of grammar-ssh, its exit among it.
    witness: Arc<std::sync::Mutex<Witness>>,
}

impl RemoteSession {
    /// The grammar-ssh this session reached.
    pub fn ssh(&self) -> &SshHello {
        self.session.ssh()
    }

    /// A request's response, or the session's loss where it ends first.
    pub async fn request(&self, request: EngineRequest) -> Result<EngineResponse, TransportError> {
        self.session.request(request).await
    }

    /// Whether the session has ended.
    pub fn has_ended(&self) -> bool {
        self.session.has_ended()
    }

    /// What ended the session, grammar-ssh's exit first where its channel told one.
    pub fn ended_by(&self) -> Option<String> {
        let reason = self.session.ended_by()?;
        let witness = self.witness.lock().unwrap_or_else(|e| e.into_inner());
        Some(match &witness.exit {
            Some(exit) => format!("grammar-ssh {exit}: {reason}"),
            None => reason,
        })
    }

    /// The session closed by the MCP: its close written, then the connection ended.
    pub async fn close(&self) {
        self.session.finish(CLOSE_FLUSH).await;
        let connection = self.connection.lock().await;
        match &self.jump {
            Some(through) => {
                let jump = through.connection.lock().await;
                let hop = Some((&*jump, &*through.tunnel_closed));
                end(&connection, hop, "the MCP closed the session").await;
            }
            None => end(&connection, None, "the MCP closed the session").await,
        }
        drop(connection);
        let pump = self.pump.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(pump) = pump {
            let _ = tk::timeout(CLOSE_FLUSH, pump).await;
        }
    }
}

/// A session on `endpoint`'s grammar-ssh: the connection, the exec, and the hello.
pub async fn open(
    endpoint: &Endpoint,
    requested: Limits,
) -> Result<(RemoteSession, tk::UnboundedReceiver<SessionEvent>), TransportError> {
    let instance = endpoint.instance()?;
    let target = Login::target(endpoint)?;
    let jump = endpoint.jump.as_ref().map(Login::jump).transpose()?;
    let (connection, through, channel) =
        match tk::timeout(AUTH_DEADLINE, connect(target, jump, endpoint.command_line())).await {
            Ok(connected) => connected?,
            Err(_) => {
                return Err(TransportError::ConnectFailed {
                    reason: format!("no session with {} in {} s", endpoint.place(), AUTH_DEADLINE.as_secs()),
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
                jump: through.map(|through| Through {
                    connection: tk::AsyncMutex::new(through.connection),
                    tunnel_closed: through.tunnel_closed,
                }),
                pump: std::sync::Mutex::new(Some(pump)),
                witness,
            },
            events,
        )),
        Err(refusal) => {
            let hop = through.as_ref().map(|through| (&through.connection, &*through.tunnel_closed));
            end(&connection, hop, "the session didn't open").await;
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

/// A session's connection, its jump's where it takes one, and its exec channel.
type Connected = (
    russh::client::Handle<Pinned>,
    Option<Gate<russh::client::Handle<Pinned>>>,
    russh::Channel<russh::client::Msg>,
);

/// A jump's connection and the signal its host gives when it closes the tunnel.
struct Gate<C> {
    connection: C,
    tunnel_closed: Arc<tk::Notify>,
}

/// A session's jump, held as long as the session's connection is.
type Through = Gate<tk::AsyncMutex<russh::client::Handle<Pinned>>>;

/// One hop's login: its place, the keys its host may show, the key it signs with.
struct Login {
    /// Where the hop is, as its refusals name it.
    place: String,
    /// Whether a jump is taken, so that every refusal names its hop.
    hop_named: bool,
    host: String,
    port: u16,
    user: String,
    pinned: Vec<keys::PublicKey>,
    key: Arc<keys::PrivateKey>,
}

impl Login {
    /// The engine's own host's login, named by its hop where a jump is taken.
    fn target(endpoint: &Endpoint) -> Result<Self, TransportError> {
        let hop_named = endpoint.jump.is_some();
        let pinned = endpoint.pinned_keys();
        Self::new(endpoint.place(), hop_named, &endpoint.host, endpoint.port, &endpoint.user, &endpoint.key, pinned)
    }

    /// The jump host's login, always named by its hop.
    fn jump(jump: &Jump) -> Result<Self, TransportError> {
        Self::new(jump.place(), true, &jump.host, jump.port, &jump.user, &jump.key, jump.pinned_keys())
    }

    /// A login whose pinned keys and key file are read before any connection.
    fn new(
        place: String,
        hop_named: bool,
        host: &str,
        port: u16,
        user: &str,
        key: &str,
        pinned: Result<Vec<keys::PublicKey>, String>,
    ) -> Result<Self, TransportError> {
        let named = |reason: String| if hop_named { format!("{place}: {reason}") } else { reason };
        let pinned = pinned.map_err(|reason| TransportError::HostKeyRejected { reason: named(reason) })?;
        let secret = keys::load_secret_key(key, None).map_err(|e| TransportError::AuthenticationFailed {
            reason: named(format!("the key file {key} couldn't be read: {e}")),
        })?;
        Ok(Self {
            place,
            hop_named,
            host: host.to_string(),
            port,
            user: user.to_string(),
            pinned,
            key: Arc::new(secret),
        })
    }

    /// `reason` named by the hop's place where a jump is taken.
    fn named(&self, reason: String) -> String {
        if self.hop_named { format!("{}: {reason}", self.place) } else { reason }
    }
}

/// The connection authenticated and grammar-ssh's command running on its channel.
async fn connect(
    target: Login,
    jump: Option<Login>,
    command_line: String,
) -> Result<Connected, TransportError> {
    let (mut connection, through) = match jump {
        None => (handshake(tcp(&target).await?, &target, Arc::default()).await?, None),
        Some(jump) => {
            let tunnel_closed = Arc::new(tk::Notify::new());
            let mut gate = handshake(tcp(&jump).await?, &jump, tunnel_closed.clone()).await?;
            authenticate(&mut gate, &jump).await?;
            let tunnel = gate
                .channel_open_direct_tcpip(target.host.clone(), u32::from(target.port), "127.0.0.1", 0)
                .await
                .map_err(|e| TransportError::ConnectFailed {
                    reason: format!("{} opened no channel to {}:{}: {e}", jump.place, target.host, target.port),
                })?;
            let connection = handshake(tunnel.into_stream(), &target, Arc::default()).await?;
            (connection, Some(Gate { connection: gate, tunnel_closed }))
        }
    };
    authenticate(&mut connection, &target).await?;
    let place = &target.place;
    let channel = connection.channel_open_session().await.map_err(|e| TransportError::Unavailable {
        reason: format!("{place} opened no session channel: {e}"),
    })?;
    channel.exec(true, command_line).await.map_err(|e| TransportError::Unavailable {
        reason: format!("{place} took no command: {e}"),
    })?;
    Ok((connection, through, channel))
}

/// A TCP connection to the hop's host, its failure named by the hop's place.
async fn tcp(login: &Login) -> Result<tk::TcpStream, TransportError> {
    let tcp = tk::TcpStream::connect((login.host.as_str(), login.port))
        .await
        .map_err(|e| TransportError::ConnectFailed { reason: format!("{}: {e}", login.place) })?;
    let _ = tcp.set_nodelay(true);
    Ok(tcp)
}

/// The SSH handshake over `stream`, the host held to the hop's pinned keys.
async fn handshake<S>(
    stream: S,
    login: &Login,
    closed: Arc<tk::Notify>,
) -> Result<russh::client::Handle<Pinned>, TransportError>
where
    S: tk::AsyncRead + tk::AsyncWrite + Unpin + Send + 'static,
{
    let rejected = Arc::new(std::sync::Mutex::new(None));
    let handler = Pinned {
        keys: login.pinned.clone(),
        rejected: rejected.clone(),
        closed,
    };
    let config = Arc::new(russh::client::Config {
        preferred: preferred(),
        nodelay: true,
        ..Default::default()
    });
    russh::client::connect_stream(config, stream, handler).await.map_err(|e| {
        match rejected.lock().unwrap_or_else(|e| e.into_inner()).take() {
            Some(reason) => TransportError::HostKeyRejected { reason: login.named(reason) },
            None => TransportError::ConnectFailed {
                reason: format!("the SSH handshake with {}: {e}", login.place),
            },
        }
    })
}

/// The hop's account authenticated with its key.
async fn authenticate(
    connection: &mut russh::client::Handle<Pinned>,
    login: &Login,
) -> Result<(), TransportError> {
    let hash = rsa_hash(connection, &login.key).await.map_err(|refusal| match refusal {
        TransportError::AuthenticationFailed { reason } => {
            TransportError::AuthenticationFailed { reason: login.named(reason) }
        }
        other => other,
    })?;
    let signer = keys::PrivateKeyWithHashAlg::new(login.key.clone(), hash);
    let auth = connection
        .authenticate_publickey(login.user.clone(), signer)
        .await
        .map_err(|e| TransportError::AuthenticationFailed { reason: login.named(e.to_string()) })?;
    if !auth.success() {
        return Err(TransportError::AuthenticationFailed {
            reason: format!("{} refused the key for `{}`", login.place, login.user),
        });
    }
    Ok(())
}

/// The connection ended, then the jump's once its host has closed the tunnel.
async fn end(
    connection: &russh::client::Handle<Pinned>,
    jump: Option<(&russh::client::Handle<Pinned>, &tk::Notify)>,
    reason: &str,
) {
    disconnect(connection, reason).await;
    if let Some((jump, tunnel_closed)) = jump {
        let _ = tk::timeout(CLOSE_FLUSH, tunnel_closed.notified()).await;
        disconnect(jump, reason).await;
    }
}

/// A connection ended by the MCP, `reason` told to its host.
async fn disconnect(
    connection: &russh::client::Handle<Pinned>,
    reason: &str,
) {
    let _ = connection.disconnect(russh::Disconnect::ByApplication, reason, "en").await;
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
    /// Told when the host closes a channel: a jump's one channel is its tunnel.
    closed: Arc<tk::Notify>,
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

    async fn channel_close(
        &mut self,
        _channel: russh::ChannelId,
        _session: &mut russh::client::Session,
    ) -> Result<(), Self::Error> {
        self.closed.notify_one();
        Ok(())
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
    /// A line of grammar-ssh's stderr kept, the oldest past the tail dropped.
    pub(crate) fn keep(&mut self, line: String) {
        if self.tail.len() == TAIL_LINES {
            self.tail.pop_front();
        }
        self.tail.push_back(line);
    }

    /// The session's refusal, told as what grammar-ssh did where the pump saw it.
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
            (_, true) => "the host refused to run grammar-ssh's command".to_string(),
            (Some(exit), false) => format!("grammar-ssh {exit} before its reply"),
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
            Some(1) => {
                said(&format!("Command `{command}` not found"))
                    || said(&format!("`{command}` is neither a Nushell built-in"))
                    || said(&format!("The term '{command}' is not recognized"))
                    || said(&format!("'{command}' is not recognized as an internal or external command"))
                    || (command.contains('\\') && said("The system cannot find the path specified."))
            }
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
                eprintln!("grammar: {alias}: grammar-ssh {exit}");
                let mut witness = witness.lock().unwrap_or_else(|e| e.into_inner());
                witness.exit = Some(exit);
                witness.status = Some(exit_status);
            }
            russh::ChannelMsg::ExitSignal { signal_name, .. } => {
                let exit = format!("was killed by {signal_name:?}");
                eprintln!("grammar: {alias}: grammar-ssh {exit}");
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
