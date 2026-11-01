//! The engine's start, and the engine's host asks routed by mcp's controller.
use crate::*;

/// Start the engine on the config's id, namespace, and homes.
pub(crate) fn init_engine(config: &Config) -> Result<(), String> {
    let scratch = make_scratch_dirs()?;
    let (_, mcp_nom) = process_identity();
    let params = EngineParams {
        id: config.id.clone(),
        namespace: config.namespace.clone(),
        work_dir: config.work_dir.clone(),
        tmp_dir: scratch.tmp,
        transient_dir: scratch.transient,
        mcp_nom: mcp_nom.as_str().to_string(),
        data_home: config.data_home(),
        cache_home: config.cache_home(),
    };
    engine::init(params).map_err(|e| e.to_string())
}

/// The substrate: the equip store, the default purview, the registry, builtins.
pub(crate) async fn ensure_substrate() -> io::Result<()> {
    let fresh = ensure_equip_store()?;
    if let Err(e) = ensure_default_purview() {
        eprintln!("grammar: default purview not materialized: {e:?}");
    }
    ensure_plugin_registry();
    if fresh {
        seed_builtin_purviews(&install_builtins());
    }
    Ok(())
}

/// The controller's handler of the engine: its host asks routed to their systems.
#[derive(Clone)]
pub(crate) struct HostRoute {
    pub(crate) channel: Arc<ChannelHandle>,
    pub(crate) peer: Arc<PeerHandle>,
    /// The runtime config, which `grimm get_config_all` reads.
    pub(crate) config: Arc<Config>,
    /// The controller's shutdown flag, under which the engine's end goes unsaid.
    pub(crate) stopping: Arc<AtomicBool>,
}

impl HostRoute {
    /// An eval's ask, answered by the system it is for.
    pub(crate) async fn answer(
        &self,
        ask: HostAsk,
    ) -> HostAnswer {
        match ask {
            HostAsk::ChannelSend(data) => self.channel_send(data).await,
            HostAsk::Stopped(stopped) => {
                let _ = self.channel.notify(StoppedNotice(stopped)).await;
                HostAnswer::Noted
            }
            HostAsk::RemoteSend(data) => self.remote_send(data).await,
            HostAsk::Config => match config_record(&self.channel, &self.config).await {
                Ok(record) => HostAnswer::Config(record),
                Err(reason) => HostAnswer::Refused(reason),
            },
            HostAsk::Pin(PinAsk { key, pid, value }) => match pin(&key, pid, value) {
                Ok(()) => HostAnswer::Pinned,
                Err(reason) => HostAnswer::Refused(reason),
            },
        }
    }

    /// A body's Channel send through the Channel's system, its verdict answered.
    async fn channel_send(
        &self,
        data: ChannelSendData,
    ) -> HostAnswer {
        match self.channel.request(SendRequest(data)).await {
            Ok(SendVerdict::Sent(id)) => HostAnswer::Sent(id),
            Ok(SendVerdict::Refused(reason)) | Err(reason) => HostAnswer::Refused(reason),
            Ok(SendVerdict::Stop(spam)) => HostAnswer::Stopped(HostStop {
                hits: spam.hits,
                window_secs: spam.window_secs,
                rate: spam.rate,
                notify: spam.notify,
                reason: spam.reason,
            }),
        }
    }

    /// A body's remote send, its files read and its event rendered, to the peer links.
    async fn remote_send(
        &self,
        data: RemoteSendData,
    ) -> HostAnswer {
        match tk::spawn_blocking(move || prepare_send(data)).await {
            Ok(Ok(prepared)) => self.send_prepared(prepared).await,
            Ok(Err(reason)) => HostAnswer::Refused(reason),
            Err(e) => HostAnswer::Refused(format!("the send's files: {e}")),
        }
    }

    /// A prepared remote send, its files carried whole, to the peer links.
    async fn send_prepared(
        &self,
        prepared: PreparedSend,
    ) -> HostAnswer {
        let PreparedSend {
            mcp_nom,
            model,
            event_nuon,
            files,
        } = prepared;
        let send = RemoteSendRequest {
            mcp_nom,
            model,
            event_nuon,
            payloads: files.into_iter().map(|file| Payload { dest: file.dest, bytes: file.bytes }).collect(),
        };
        match self.peer.request(send).await {
            Ok(Ok(id)) => HostAnswer::Sent(id),
            Ok(Err(reason)) | Err(reason) => HostAnswer::Refused(reason),
        }
    }

    /// A remote engine's ask, answered as a local eval's, its origin naming the engine.
    pub(crate) async fn remote_answer(
        &self,
        alias: &str,
        ask: SessionAsk,
    ) -> HostAnswer {
        match ask {
            SessionAsk::ChannelSend(mut data) => {
                data.from = remote_origin(alias, &data.from);
                self.channel_send(data).await
            }
            SessionAsk::Stopped(mut stopped) => {
                stopped.origin = remote_origin(alias, &stopped.origin);
                let _ = self.channel.notify(StoppedNotice(stopped)).await;
                HostAnswer::Noted
            }
            SessionAsk::RemoteSend(prepared) => self.send_prepared(prepared).await,
            SessionAsk::Config => match config_record(&self.channel, &self.config).await {
                Ok(record) => HostAnswer::Config(record),
                Err(reason) => HostAnswer::Refused(reason),
            },
        }
    }
}

/// A remote eval's origin, `<alias>/thread/<nonce>`, apart from every local one.
pub(crate) fn remote_origin(
    alias: &str,
    origin: &str,
) -> String {
    format!("{alias}/{origin}")
}

impl subsys::Handler<EngineSystem> for HostRoute {
    fn on_packet(&mut self, pkt: subsys::Packet<FromEngine>) -> impl Future<Output = Option<subsys::Packet<FromEngine>>> + 'static + Send {
        std::future::ready(Some(pkt))
    }

    fn on_request(&mut self, pkt: subsys::Packet<FromEngine>) -> impl Future<Output = Option<ToEngine>> + 'static + Send {
        let route = self.clone();
        async move {
            let FromEngine::Host(ask) = pkt.take_msg() else {
                return None;
            };
            Some(ToEngine::Host(route.answer(ask).await))
        }
    }

    fn on_sub(&mut self, sub: subsys::FromSub) -> impl Future<Output = ()> + Send {
        log_status("the engine's system", sub, &self.stopping);
        std::future::ready(())
    }

    fn on_closed(&mut self) -> impl Future<Output = ()> + Send {
        log_closed("the engine's system", &self.stopping);
        std::future::ready(())
    }
}

/// The whole config as a record mirroring the TOML's tables, with what is in force.
async fn config_record(
    handle: &ChannelHandle,
    cfg: &Config,
) -> Result<NuVal, String> {
    let span = nu::Span::unknown();
    let spam = live_thresholds(handle, cfg).await;
    let supervisor = effective_supervisor(cfg.supervisor);

    let mut channel = nu::Record::new();
    channel.insert(
        "port",
        match cfg.channel.port {
            Some(port) => nu::Value::int(port as i64, span),
            None => nu::Value::nothing(span),
        },
    );
    channel.insert(
        "spam_warn_window_secs",
        nu::Value::int(spam.warn_window_secs as i64, span),
    );
    channel.insert("spam_warn_rate", nu::Value::int(spam.warn_rate as i64, span));
    channel.insert(
        "spam_error_window_secs",
        nu::Value::int(spam.error_window_secs as i64, span),
    );
    channel.insert(
        "spam_error_rate",
        nu::Value::int(spam.error_rate as i64, span),
    );

    let mut sup = nu::Record::new();
    sup.insert(
        "cpu_warn_fraction",
        nu::Value::float(supervisor.cpu_warn_fraction, span),
    );
    sup.insert(
        "ram_warn_fraction",
        nu::Value::float(supervisor.ram_warn_fraction, span),
    );
    sup.insert(
        "vram_warn_headroom_mib",
        nu::Value::int(supervisor.vram_warn_headroom_mib as i64, span),
    );
    sup.insert(
        "disk_warn_fraction",
        nu::Value::float(supervisor.disk_warn_fraction, span),
    );

    let identity = match &cfg.identity {
        Some(identity) => {
            let path = |file: &Path| nu::Value::string(file.to_string_lossy().into_owned(), span);
            let mut record = nu::Record::new();
            record.insert("cert", path(&identity.cert));
            record.insert("key", path(&identity.key));
            nu::Value::record(record, span)
        }
        None => nu::Value::nothing(span),
    };

    let mut root = nu::Record::new();
    root.insert("channel", nu::Value::record(channel, span));
    root.insert("supervisor", nu::Value::record(sup, span));
    root.insert("identity", identity);
    NuVal::try_from_nu(nu::Value::record(root, span)).map_err(|e| e.to_string())
}

/// The spam thresholds in force: the running Channel's, else the config's start.
async fn live_thresholds(
    handle: &ChannelHandle,
    cfg: &Config,
) -> SpamThresholdsData {
    match handle.request(ThresholdsRequest::default()).await {
        Ok(Ok(thresholds)) => thresholds,
        _ => cfg.channel.spam.data(),
    }
}
