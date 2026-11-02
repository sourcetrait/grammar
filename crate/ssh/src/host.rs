//! The engine's start, and the engine's host asks routed by grammar-ssh's controller.
use crate::*;

/// The engine started on the config's namespace, homes, and workdir, and the scratch.
pub(crate) fn init_engine(
    config: &Config,
    scratch: &ScratchDirs,
) -> GrammarSshResult<()> {
    engine::init(engine::EngineParams {
        id: config.id.clone(),
        namespace: config.namespace.clone(),
        work_dir: config.workdir.clone(),
        tmp_dir: scratch.tmp().to_path_buf(),
        transient_dir: scratch.transient().to_path_buf(),
        data_home: config.data_home(),
        cache_home: config.cache_home(),
    })
    .map_err(|e| GrammarSshError::EngineInit { reason: e.to_string() })
}

/// The substrate: the equip store, the default purview, the registry, builtins.
pub(crate) fn ensure_substrate() -> GrammarSshResult<()> {
    let fresh = engine::ensure_equip_store().map_err(|e| GrammarSshError::EngineInit {
        reason: format!("the equip store: {e}"),
    })?;
    if let Err(e) = engine::ensure_default_purview() {
        eprintln!("grammar: default purview not materialized: {e:?}");
    }
    engine::ensure_plugin_registry();
    if fresh {
        engine::seed_builtin_purviews(&engine::install_builtins());
    }
    Ok(())
}

/// The controller's handler of the engine: a pin held here, every other ask the MCP's.
#[derive(Clone)]
pub(crate) struct HostRoute {
    /// The session's MCP, which answers every ask but a pin.
    pub(crate) asker: Arc<Asker>,
    /// grammar-ssh's own supervisor lines, which a config answer carries.
    pub(crate) supervisor: SupervisorConfig,
    /// The controller's shutdown flag, under which the engine's end goes unsaid.
    pub(crate) stopping: Arc<AtomicBool>,
}

impl HostRoute {
    /// An eval's ask, answered here or by the session's MCP.
    async fn answer(
        &self,
        ask: HostAsk,
    ) -> HostAnswer {
        match ask {
            HostAsk::Pin(PinAsk { key, pid, value }) => match pin(&key, pid, value) {
                Ok(()) => HostAnswer::Pinned,
                Err(reason) => HostAnswer::Refused(reason),
            },
            HostAsk::Config => match self.asker.ask(SessionAsk::Config).await {
                HostAnswer::Config(record) => {
                    HostAnswer::Config(with_supervisor(record, effective_supervisor(self.supervisor)))
                }
                other => other,
            },
            HostAsk::ChannelSend(data) => self.asker.ask(SessionAsk::ChannelSend(data)).await,
            HostAsk::Stopped(stopped) => self.asker.ask(SessionAsk::Stopped(stopped)).await,
            HostAsk::RemoteSend(data) => match tk::spawn_blocking(move || prepare_send(data)).await {
                Ok(Ok(prepared)) => self.asker.ask(SessionAsk::RemoteSend(prepared)).await,
                Ok(Err(reason)) => HostAnswer::Refused(reason),
                Err(e) => HostAnswer::Refused(format!("the send's files: {e}")),
            },
        }
    }
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

/// The MCP's config record with grammar-ssh's supervisor lines in place of its own.
fn with_supervisor(
    record: NuVal,
    supervisor: SupervisorConfig,
) -> NuVal {
    let lines = NuVal::Record(vec![
        ("cpu_warn_fraction".to_string(), NuVal::Float(supervisor.cpu_warn_fraction)),
        ("ram_warn_fraction".to_string(), NuVal::Float(supervisor.ram_warn_fraction)),
        (
            "vram_warn_headroom_mib".to_string(),
            NuVal::Int(i64::try_from(supervisor.vram_warn_headroom_mib).unwrap_or(i64::MAX)),
        ),
        ("disk_warn_fraction".to_string(), NuVal::Float(supervisor.disk_warn_fraction)),
    ]);
    match record {
        NuVal::Record(mut fields) => {
            match fields.iter_mut().find(|(key, _)| key == "supervisor") {
                Some((_, value)) => *value = lines,
                None => fields.push(("supervisor".to_string(), lines)),
            }
            NuVal::Record(fields)
        }
        other => other,
    }
}
