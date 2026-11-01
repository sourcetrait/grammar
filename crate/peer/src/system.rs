use crate::*;

/// How long a closed listener's close waits for its accept to let the port go.
const CLOSE_FRAME_GRACE: tk::TkDuration = tk::TkDuration::from_secs(2);

/// The peer links as a subsys system: their registries, remotes file, and sends.
pub struct PeerSystem {
    inner: subsys::InnerSystem<Self>,
    /// This host's nom, which each handshake names.
    mcp_nom: String,
    identity: Option<IdentityKeys>,
    /// The agent's config file, named in an identity's refusal.
    config_file: String,
    remotes_file: PathBuf,
    expand: Vec<ExpandDefault>,
    /// Every open link, by alias.
    links: HashMap<String, RemoteLinkEntry>,
    /// Bound-but-unpaired listeners, by alias.
    listeners: HashMap<String, BoundListener>,
    /// The aliases a connector is dialing, each refused a second open meanwhile.
    dialing: HashSet<String>,
    /// Each bound listener's identity, so a task leaves only its own entry.
    seq: u64,
    /// Mints each send's id.
    nonce_gen: datum::NonceGenerator,
    ctx: LinkContext,
    ups: tk::UnboundedReceiver<Up>,
    events_tx: tk::UnboundedSender<LinkEvent>,
    events: tk::UnboundedReceiver<LinkEvent>,
}

/// The system's parameters: the host's nom, its key pair, and its remotes file.
#[cereal::derived(Data, Eq)]
pub struct PeerSysParams {
    pub mcp_nom: String,
    pub identity: Option<IdentityKeys>,
    /// The agent's config file, named where a link finds no identity.
    pub config_file: String,
    pub remotes_file: String,
    /// The stand-ins a certificate's unset variables expand to.
    pub expand: Vec<ExpandDefault>,
}

impl subsys::Params for PeerSysParams {}

/// The system's configuration, which holds nothing.
#[derive(Default, Debug, Clone, ser::Serialize, ser::Deserialize)]
pub struct PeerSysConfig;

impl subsys::Config for PeerSysConfig {}

/// The namespace's homes as the system's paths, which it never writes.
#[derive(Debug, Clone)]
pub struct PeerSysPaths {
    data_home: PathBuf,
    cache_home: PathBuf,
}

impl PeerSysPaths {
    /// The paths of a namespace with these data and cache homes.
    pub fn new(
        data_home: PathBuf,
        cache_home: PathBuf,
    ) -> Self {
        Self {
            data_home,
            cache_home,
        }
    }

    /// The namespace's secrets, its keypair's directory.
    fn secrets_dir(&self) -> PathBuf {
        self.data_home.join("keypair")
    }
}

impl agnostic::AppPaths for PeerSysPaths {
    fn cache_dir(&self) -> PathBuf {
        self.cache_home.clone()
    }

    fn config_dir(&self) -> PathBuf {
        self.data_home.clone()
    }

    fn config_secrets_dir(&self) -> PathBuf {
        self.secrets_dir()
    }

    fn data_dir(&self) -> PathBuf {
        self.data_home.clone()
    }

    fn state_dir(&self) -> PathBuf {
        self.data_home.clone()
    }

    fn state_secrets_dir(&self) -> PathBuf {
        self.secrets_dir()
    }

    fn runtime_dir(&self) -> PathBuf {
        self.cache_home.clone()
    }
}

impl agnostic::HasAppPaths for PeerSysPaths {
    fn app_paths(&self) -> &impl agnostic::AppPaths {
        self
    }
}

/// A refusal of a remote command's parameters.
fn invalid(reason: String) -> RuntimeError {
    RuntimeError::RemoteInvalidParams { reason }
}

impl PeerSystem {
    /// A request's answer now, or none for one that answers later.
    async fn answer(
        &mut self,
        id: subsys::MsgID,
        msg: ToPeerSys,
    ) -> Option<FromPeerSys> {
        let remote = match msg {
            ToPeerSys::List(RemoteListRequest) => self.rows().map(RemoteResult::Rows),
            ToPeerSys::Get(RemoteGetRequest { alias }) => self.row(&alias).map(RemoteResult::Row),
            ToPeerSys::Set(req) => self.set(req).map(RemoteResult::Row),
            ToPeerSys::Unset(RemoteUnsetRequest { alias }) => {
                self.unset(&alias).map(|()| RemoteResult::Done)
            }
            ToPeerSys::Listen(req) => self.listen(req).await.map(RemoteResult::Listening),
            ToPeerSys::Connect(req) => match self.connect(id, req) {
                Ok(()) => return None,
                Err(e) => Err(e),
            },
            ToPeerSys::Close(RemoteCloseRequest { alias }) => match self.close(id, alias) {
                Ok(true) => Ok(RemoteResult::Done),
                Ok(false) => return None,
                Err(e) => Err(e),
            },
            ToPeerSys::Send(req) => return Some(FromPeerSys::Sent(self.send(req))),
            ToPeerSys::Inbox(InboxRequest(inbox)) => {
                self.ctx.set_inbox(inbox.clone().map(PathBuf::from));
                return Some(FromPeerSys::Inbox(inbox));
            }
        };
        Some(FromPeerSys::Remote(remote))
    }

    /// The remotes file's entries; a malformed file is refused whole.
    fn entries(&self) -> Result<Vec<RemoteEntry>, RuntimeError> {
        read_remotes(&self.remotes_file).map_err(|reason| RuntimeError::RemotesFile { reason })
    }

    /// The live aliases: each open link, then each listener still waiting.
    fn live(&self) -> HashMap<String, (String, Option<String>)> {
        let mut live: HashMap<String, (String, Option<String>)> = self
            .links
            .iter()
            .map(|(alias, link)| {
                let state = (link.addr.to_string(), Some(link.handle.remote_mcp_nom.clone()));
                (alias.clone(), state)
            })
            .collect();
        for (alias, listener) in &self.listeners {
            live.entry(alias.clone())
                .or_insert((listener.addr.to_string(), None));
        }
        live
    }

    /// Every remote's row: the file's entries and each live alias, by alias.
    fn rows(&self) -> Result<Vec<RemoteChannelRow>, RuntimeError> {
        let entries = self.entries()?;
        let live = self.live();
        let mut aliases: Vec<String> = entries
            .iter()
            .map(|entry| entry.alias.clone())
            .chain(live.keys().cloned())
            .collect();
        aliases.sort();
        aliases.dedup();
        Ok(aliases
            .into_iter()
            .map(|alias| {
                let entry = entries.iter().find(|entry| entry.alias == alias);
                let (state, endpoint, remote_mcp_nom) = match live.get(&alias) {
                    Some((endpoint, Some(nom))) => {
                        (RemoteLinkState::Open, Some(endpoint.clone()), Some(nom.clone()))
                    }
                    Some((endpoint, None)) => (RemoteLinkState::Listening, Some(endpoint.clone()), None),
                    None => (RemoteLinkState::Closed, None, None),
                };
                RemoteChannelRow {
                    address: entry.and_then(|entry| entry.address.clone()),
                    listen: entry.and_then(|entry| entry.listen.clone()),
                    cert: entry.map(|entry| entry.cert.clone()),
                    alias,
                    state,
                    endpoint,
                    remote_mcp_nom,
                }
            })
            .collect())
    }

    /// One remote's row; an alias neither configured nor live is refused.
    fn row(
        &self,
        alias: &str,
    ) -> Result<RemoteChannelRow, RuntimeError> {
        self.rows()?
            .into_iter()
            .find(|row| row.alias == alias)
            .ok_or_else(|| invalid(format!("no remote channel `{alias}`")))
    }

    /// Check the fields given, write them to the entry, and read its row back.
    fn set(
        &self,
        req: RemoteSetRequest,
    ) -> Result<RemoteChannelRow, RuntimeError> {
        let RemoteSetRequest {
            alias,
            address,
            listen,
            cert,
        } = req;
        if alias.is_empty() {
            return Err(invalid("a remote channel's alias is never empty".to_string()));
        }
        if let Some(address) = &address {
            dial_address(address).map_err(invalid)?;
        }
        if let Some(listen) = &listen {
            bind_address(listen).map_err(invalid)?;
        }
        if cert.as_deref() == Some("") {
            return Err(invalid("`--cert` names the peer's certificate".to_string()));
        }
        let known = self.entries()?.iter().any(|entry| entry.alias == alias);
        if !known && cert.is_none() {
            return Err(invalid(format!(
                "remote channel `{alias}` is new; give `--cert`, its peer's certificate",
            )));
        }
        let change = RemoteChange {
            cert,
            address,
            listen,
        };
        edit_remotes(&self.remotes_file, |text| set_entry(text, &alias, &change))
            .map_err(|reason| RuntimeError::RemotesFile { reason })?;
        self.row(&alias)
    }

    /// Remove `alias`'s entry; an alias the file lacks is refused.
    fn unset(
        &self,
        alias: &str,
    ) -> Result<(), RuntimeError> {
        if !self.entries()?.iter().any(|entry| entry.alias == alias) {
            return Err(invalid(format!("no remote channel `{alias}` in the remotes file")));
        }
        edit_remotes(&self.remotes_file, |text| unset_entry(text, alias))
            .map_err(|reason| RuntimeError::RemotesFile { reason })
    }

    /// The entry a listen or a connect opens, its alias idle.
    fn to_open(
        &self,
        alias: &str,
    ) -> Result<RemoteEntry, RuntimeError> {
        let busy = self.links.contains_key(alias)
            || self.listeners.contains_key(alias)
            || self.dialing.contains(alias);
        if busy {
            return Err(RuntimeError::RemoteAlreadyOpen {
                alias: alias.to_string(),
            });
        }
        self.entries()?
            .into_iter()
            .find(|entry| entry.alias == alias)
            .ok_or_else(|| {
                invalid(format!(
                    "no remote channel `{alias}`; add it with `interface remote channel set`"
                ))
            })
    }

    /// The keys a link to `entry`'s peer presents and matches.
    fn keys(
        &self,
        entry: &RemoteEntry,
    ) -> Result<LinkKeys, RuntimeError> {
        let identity = self.identity.as_ref().ok_or_else(|| {
            invalid(format!(
                "a link presents the host's [identity]; name its `cert` and `key` in {}",
                self.config_file,
            ))
        })?;
        let peer_cert = entry.peer_cert(&self.expand).map_err(invalid)?;
        Ok(LinkKeys {
            self_cert: PathBuf::from(&identity.cert),
            self_key: PathBuf::from(&identity.key),
            peer_cert,
        })
    }

    /// Bind for `alias`'s peer, answering the address bound; the accept runs behind.
    async fn listen(
        &mut self,
        req: RemoteListenRequest,
    ) -> Result<RemoteListenResult, RuntimeError> {
        let RemoteListenRequest { alias, on } = req;
        let entry = self.to_open(&alias)?;
        let bind = match &on {
            Some(on) => bind_address(on).map_err(invalid)?,
            None => entry.bind(),
        };
        let keys = self.keys(&entry)?;
        let open_failed = |reason: String| RuntimeError::RemoteOpenFailed {
            alias: alias.clone(),
            reason,
        };
        let (listener, acceptor) = bind_listener(&keys, bind)
            .await
            .map_err(|e| open_failed(format!("bind {bind}: {e}")))?;
        let bound = listener
            .local_addr()
            .map_err(|e| open_failed(format!("bind {bind}: {e}")))?;
        self.seq += 1;
        let cancel = tku::CancellationToken::new();
        let (released, released_rx) = tk::oneshot::channel::<()>();
        self.listeners.insert(
            alias.clone(),
            BoundListener {
                addr: bound,
                seq: self.seq,
                cancel: cancel.clone(),
                released: Some(released_rx),
            },
        );
        let listen = Listen {
            self_mcp_nom: self.mcp_nom.clone(),
            alias,
            seq: self.seq,
            bound,
            listener,
            acceptor,
            cancel,
            released,
        };
        listen.spawn(self.ctx.clone(), self.events_tx.clone());
        Ok(RemoteListenResult {
            endpoint: bound.to_string(),
        })
    }

    /// Dial `alias`'s peer on a task of its own; its answer comes once it ends.
    fn connect(
        &mut self,
        reply: subsys::MsgID,
        req: RemoteConnectRequest,
    ) -> Result<(), RuntimeError> {
        let RemoteConnectRequest { alias, address } = req;
        let entry = self.to_open(&alias)?;
        let addr = match &address {
            Some(address) => dial_address(address).map_err(invalid)?,
            None => entry.dial().ok_or_else(|| {
                invalid(format!(
                    "remote channel `{alias}` names no address; give `--address` or set one \
                     with `interface remote channel set`",
                ))
            })?,
        };
        let keys = self.keys(&entry)?;
        self.dialing.insert(alias.clone());
        let dial = Dial {
            self_mcp_nom: self.mcp_nom.clone(),
            alias,
            keys,
            addr,
            reply,
        };
        dial.spawn(self.ctx.clone(), self.events_tx.clone());
        Ok(())
    }

    /// Close an open link, or a waiting listener once it lets its port go.
    fn close(
        &mut self,
        reply: subsys::MsgID,
        alias: String,
    ) -> Result<bool, RuntimeError> {
        if let Some(link) = self.links.remove(&alias) {
            link.handle.cancel();
            return Ok(true);
        }
        let Some(mut listener) = self.listeners.remove(&alias) else {
            return Err(RuntimeError::RemoteNotOpen { alias });
        };
        listener.cancel.cancel();
        let Some(released) = listener.released.take() else {
            return Ok(true);
        };
        let events = self.events_tx.clone();
        tk::spawn(async move {
            let _ = tk::timeout(CLOSE_FRAME_GRACE, released).await;
            let _ = events.send(LinkEvent::Released { reply });
        });
        Ok(false)
    }

    /// A body's send on the open link to its peer, its id minted here.
    fn send(
        &mut self,
        req: RemoteSendRequest,
    ) -> Result<String, String> {
        let RemoteSendRequest {
            mcp_nom,
            model,
            event_nuon,
            payloads,
        } = req;
        if let Some(prefix) = reserved_prefix(&model) {
            return Err(format!(
                "`{prefix}` is reserved for host-originated models; \
                 choose a model path outside it",
            ));
        }
        let id = mint_msg_id(&self.nonce_gen, "remote/send", &model, &event_nuon, None).to_string();
        let payloads = payloads.into_iter().map(|p| (p.dest, p.bytes)).collect();
        let (wire_event_nuon, payloads) = spill_remote_event(event_nuon, payloads)?;
        let link = self
            .links
            .values()
            .find(|link| link.handle.remote_mcp_nom == mcp_nom)
            .ok_or_else(|| format!("no open remote link to mcp_nom `{mcp_nom}`"))?;
        link.handle.enqueue_send(id.clone(), model, wire_event_nuon, payloads)?;
        Ok(id)
    }

    /// A link task's news, taken into the registries and reported in order.
    async fn on_event(
        &mut self,
        event: LinkEvent,
    ) {
        match event {
            LinkEvent::Dialed {
                alias,
                addr,
                drivers,
                reply,
            } => {
                self.dialing.remove(&alias);
                let (handle, msg_join, file_join) = drivers;
                let peer_nom = handle.remote_mcp_nom.clone();
                self.seq += 1;
                let generation = self.seq;
                let link = RemoteLinkEntry {
                    handle,
                    addr,
                    generation,
                };
                self.links.insert(alias.clone(), link);
                let joins = (msg_join, file_join);
                watch_link(alias, peer_nom, generation, joins, self.events_tx.clone());
                self.reply(reply, Ok(RemoteResult::Done)).await;
            }
            LinkEvent::DialFailed {
                alias,
                reason,
                reply,
            } => {
                self.dialing.remove(&alias);
                self.reply(reply, Err(RuntimeError::RemoteOpenFailed { alias, reason }))
                    .await;
            }
            LinkEvent::Paired {
                alias,
                seq,
                link,
                promoted,
            } => {
                if self.listeners.get(&alias).is_none_or(|listener| listener.seq != seq) {
                    let _ = promoted.send(Err(link));
                    return;
                }
                self.listeners.remove(&alias);
                let peer_nom = link.handle.remote_mcp_nom.clone();
                self.links.insert(alias.clone(), link);
                self.ctx.report(RemoteChannelEvent::Connected {
                    remote: alias,
                    mcp_nom: peer_nom,
                });
                let _ = promoted.send(Ok(()));
            }
            LinkEvent::ListenFailed { alias, seq, reason } => {
                if self.listeners.get(&alias).is_some_and(|listener| listener.seq == seq) {
                    self.listeners.remove(&alias);
                }
                self.ctx.report(RemoteChannelEvent::Disconnected {
                    remote: alias,
                    mcp_nom: None,
                    error: Some(RemoteFailure {
                        kind: "listen".to_string(),
                        message: reason,
                    }),
                });
            }
            LinkEvent::Released { reply } => self.reply(reply, Ok(RemoteResult::Done)).await,
            LinkEvent::Ended {
                alias,
                peer_nom,
                generation,
            } => {
                let current = self.links.get(&alias).map(|link| link.generation);
                match ended_link(current, generation) {
                    EndedLink::Current => {
                        self.links.remove(&alias);
                    }
                    EndedLink::Replaced => return,
                    EndedLink::Closed => {}
                }
                self.ctx.report(RemoteChannelEvent::Disconnected {
                    remote: alias,
                    mcp_nom: Some(peer_nom),
                    error: None,
                });
            }
        }
    }

    /// A request's answer that came later than its request.
    async fn reply(
        &mut self,
        reply: subsys::MsgID,
        answer: Result<RemoteResult, RuntimeError>,
    ) {
        let response = subsys::Packet::response(reply, FromPeerSys::Remote(answer));
        let _ = self.inner.channel.tx.send(subsys::MsgFromSys::Packet(response)).await;
    }

    /// A report or a relay up to the controller, a singular packet.
    async fn send_up(
        &mut self,
        up: Up,
    ) {
        let msg = match up {
            Up::Relay(relay) => FromPeerSys::Relay(relay),
            Up::Report(report) => FromPeerSys::Report(report),
        };
        let packet = subsys::Packet::singular(msg);
        let _ = self.inner.channel.tx.send(subsys::MsgFromSys::Packet(packet)).await;
    }
}

/// Spill an event the receiver's cap would truncate into a transferred file.
fn spill_remote_event(
    event_nuon: String,
    mut payloads: Vec<(String, Vec<u8>)>,
) -> Result<(String, Vec<(String, Vec<u8>)>), String> {
    if !event_overflows(&event_nuon) {
        return Ok((event_nuon, payloads));
    }
    let pointer_nuon = render_nuon(&event_spill_pointer(EVENT_SPILL_DEST, event_nuon.len()))?;
    payloads.push((EVENT_SPILL_DEST.to_string(), event_nuon.into_bytes()));
    Ok((pointer_nuon, payloads))
}

impl subsys::System for PeerSystem {
    const CHANNEL_SIZE: usize = 100;

    type Paths = PeerSysPaths;
    type Params = PeerSysParams;
    type Config = PeerSysConfig;
    type ToSys = ToPeerSys;
    type FromSys = FromPeerSys;
    type Flow = subsys::StdFlow;

    fn inner(&self) -> &subsys::InnerSystem<Self> { &self.inner }
    fn inner_mut(&mut self) -> &mut subsys::InnerSystem<Self> { &mut self.inner }

    async fn init(inner: subsys::InnerSystem<Self>, params: Self::Params) -> subsys::SubsysResult<Self> {
        let (ups_tx, ups) = tk::unbounded_channel();
        let (events_tx, events) = tk::unbounded_channel();
        Ok(Self {
            inner,
            mcp_nom: params.mcp_nom,
            identity: params.identity,
            config_file: params.config_file,
            remotes_file: PathBuf::from(params.remotes_file),
            expand: params.expand,
            links: HashMap::new(),
            listeners: HashMap::new(),
            dialing: HashSet::new(),
            seq: 0,
            nonce_gen: datum::NonceGenerator::new(),
            ctx: LinkContext::new(ups_tx),
            ups,
            events_tx,
            events,
        })
    }

    async fn run(mut self) -> subsys::RunResult {
        let result = loop {
            let flow = tk::select! {
                biased;
                msg = self.inner.channel.rx.recv() => match msg {
                    Some(msg) => self.handle_channel_recv(msg).await,
                    None => break subsys::SUCCESS,
                },
                Some(event) = self.events.recv() => {
                    self.on_event(event).await;
                    Ok(subsys::Flow::Continue)
                }
                Some(up) = self.ups.recv() => {
                    self.send_up(up).await;
                    Ok(subsys::Flow::Continue)
                }
            };
            match flow {
                Ok(subsys::Flow::Control(subsys::Control::Stop { .. })) => break subsys::SUCCESS,
                Ok(_) => {}
                Err(e) => break Err(e),
            }
        };
        self.done(result).await
    }

    async fn on_channel_recv(&mut self, pkt: subsys::Packet<ToPeerSys>) -> subsys::FlowResult<Self::Flow> {
        let (id, _nature, msg) = pkt.into_tuple();
        if let Some(answer) = self.answer(id, msg).await {
            let response = subsys::Packet::response(id, answer);
            let _ = self.inner.channel.tx.send(subsys::MsgFromSys::Packet(response)).await;
        }
        Ok(subsys::Flow::Continue)
    }

    async fn on_stop(&mut self, _halt: bool) -> subsys::RunResult {
        for (_, link) in self.links.drain() {
            link.handle.cancel();
        }
        for (_, listener) in self.listeners.drain() {
            listener.cancel.cancel();
        }
        subsys::SUCCESS
    }

    async fn on_resume(&mut self) -> subsys::SysResult<bool> {
        Ok(true)
    }
}

/// The peer links' running system, and the handle their controller asks through.
pub struct PeerHandle {
    control: subsys::SystemControl<PeerSystem>,
    /// The runtime the system runs on, which a thread outside it blocks on.
    runtime: tk::Handle,
}

impl PeerHandle {
    /// Starts the peer links' system, `handler` taking what it sends up.
    pub async fn start(
        paths: PeerSysPaths,
        params: PeerSysParams,
        handler: impl subsys::Handler<PeerSystem>,
    ) -> Result<Self, String> {
        let control = subsys::SystemControl::start(paths, PeerSysConfig, params, handler)
            .await
            .map_err(system_error)?;
        Ok(Self {
            control,
            runtime: tk::Handle::current(),
        })
    }

    /// Sends a request and awaits its answer.
    pub async fn request<T>(
        &self,
        req: T,
    ) -> Result<T::ResponseType, String>
    where
        T: subsys::Request<PeerSystem>,
        T::ResponseType: TryFrom<FromPeerSys, Error = subsys::SubsysError>,
    {
        self.control.request(req).await.map_err(system_error)
    }

    /// A request from a thread outside the runtime, its answer awaited.
    pub fn request_blocking<T>(
        &self,
        req: T,
    ) -> Result<T::ResponseType, String>
    where
        T: subsys::Request<PeerSystem>,
        T::ResponseType: TryFrom<FromPeerSys, Error = subsys::SubsysError>,
    {
        self.runtime.block_on(self.request(req))
    }

    /// Stops the peer links' system, every link and listener with it.
    pub async fn stop(self) -> Result<(), String> {
        self.control.stop(true).await.map_err(system_error)
    }
}

/// The system's failure as its callers report it.
fn system_error(e: subsys::SubsysError) -> String {
    format!("grammar: the peer links' system: {e}")
}
