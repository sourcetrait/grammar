use crate::*;

/// The operator as a subsys system: the watchdog's ticks and observations.
pub struct OperatorSystem {
    inner: subsys::InnerSystem<Self>,
    watchdog: Watchdog,
    probe_tx: tk::UnboundedSender<Probe>,
    probe_rx: tk::UnboundedReceiver<Probe>,
}

/// The system's parameters: how it samples and the lines it warns at.
#[cereal::derived(Data)]
pub struct OperatorSysParams {
    /// Whether the host runs with `--test`, sampling only while its Channel is open.
    pub test: bool,
    /// The supervisor lines in force under any pin.
    pub supervisor: SupervisorConfig,
}

impl subsys::Params for OperatorSysParams {}

/// The system's configuration, which holds nothing.
#[derive(Default, Debug, Clone, ser::Serialize, ser::Deserialize)]
pub struct OperatorSysConfig;

impl subsys::Config for OperatorSysConfig {}

/// The namespace's homes as the system's paths.
#[derive(Debug, Clone)]
pub struct OperatorSysPaths {
    data_home: PathBuf,
    cache_home: PathBuf,
}

impl OperatorSysPaths {
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

impl agnostic::AppPaths for OperatorSysPaths {
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

impl agnostic::HasAppPaths for OperatorSysPaths {
    fn app_paths(&self) -> &impl agnostic::AppPaths {
        self
    }
}

impl OperatorSystem {
    /// One tick: its raised conditions sent, its due probes started.
    async fn on_tick(&mut self) {
        let (raised, due) = self.watchdog.tick(Instant::now());
        self.emit(raised).await;
        if due.vram {
            let tx = self.probe_tx.clone();
            tk::spawn_blocking(move || {
                let _ = tx.send(Probe::Vram(probe_vram()));
            });
        }
        if due.disk {
            let tx = self.probe_tx.clone();
            tk::spawn_blocking(move || {
                let _ = tx.send(Probe::Disk(probe_disk()));
            });
        }
    }

    /// A probe's result, sampled and any raised condition sent.
    async fn on_probe(
        &mut self,
        probe: Probe,
    ) {
        let now = Instant::now();
        let raised = match probe {
            Probe::Vram(sample) => self.watchdog.vram_sampled(sample, now),
            Probe::Disk(rows) => self.watchdog.disk_sampled(rows, now),
        };
        self.emit(raised).await;
    }

    /// Send each emergency to the manager, a packet no request waits on.
    async fn emit(
        &mut self,
        raised: Vec<Emergency>,
    ) {
        for emergency in raised {
            let packet = subsys::Packet::singular(FromOperatorSys::Emergency(emergency));
            let _ = self.inner.channel.tx.send(subsys::MsgFromSys::Packet(packet)).await;
        }
    }
}

impl subsys::System for OperatorSystem {
    const CHANNEL_SIZE: usize = 100;

    type Paths = OperatorSysPaths;
    type Params = OperatorSysParams;
    type Config = OperatorSysConfig;
    type ToSys = ToOperatorSys;
    type FromSys = FromOperatorSys;
    type Flow = subsys::StdFlow;

    fn inner(&self) -> &subsys::InnerSystem<Self> { &self.inner }
    fn inner_mut(&mut self) -> &mut subsys::InnerSystem<Self> { &mut self.inner }

    async fn init(inner: subsys::InnerSystem<Self>, params: Self::Params) -> subsys::SubsysResult<Self> {
        let (probe_tx, probe_rx) = tk::unbounded_channel();
        Ok(Self {
            inner,
            watchdog: Watchdog::new(params.supervisor, params.test),
            probe_tx,
            probe_rx,
        })
    }

    async fn run(mut self) -> subsys::RunResult {
        let mut tick = tk::interval(SAMPLE_INTERVAL);
        tick.set_missed_tick_behavior(tk::MissedTickBehavior::Delay);
        tick.tick().await;
        let result = loop {
            let flow = tk::select! {
                msg = self.inner.channel.rx.recv() => match msg {
                    Some(msg) => self.handle_channel_recv(msg).await,
                    None => break subsys::SUCCESS,
                },
                _ = tick.tick() => {
                    self.on_tick().await;
                    Ok(subsys::Flow::Continue)
                }
                Some(probe) = self.probe_rx.recv() => {
                    self.on_probe(probe).await;
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

    async fn on_channel_recv(&mut self, pkt: subsys::Packet<ToOperatorSys>) -> subsys::FlowResult<Self::Flow> {
        let ToOperatorSys::Observe(observation) = pkt.take_msg();
        let raised = self.watchdog.observe(observation, now_millis(), Instant::now());
        self.emit(raised).await;
        Ok(subsys::Flow::Continue)
    }

    async fn on_stop(&mut self, _halt: bool) -> subsys::RunResult {
        subsys::SUCCESS
    }

    async fn on_resume(&mut self) -> subsys::SysResult<bool> {
        Ok(true)
    }
}

/// The operator's running system, and the handle its manager observes through.
pub struct OperatorHandle {
    control: subsys::SystemControl<OperatorSystem>,
}

impl OperatorHandle {
    /// Starts the operator on a namespace's homes, `handler` taking its emergencies.
    pub async fn start(
        paths: OperatorSysPaths,
        params: OperatorSysParams,
        handler: impl subsys::Handler<OperatorSystem>,
    ) -> Result<Self, String> {
        let control = subsys::SystemControl::start(paths, OperatorSysConfig, params, handler)
            .await
            .map_err(|e| format!("grammar: the operator's system: {e}"))?;
        Ok(Self { control })
    }

    /// Passes what the manager observed; the operator raises what it confirms.
    pub async fn observe(
        &self,
        observation: Observation,
    ) -> Result<(), String> {
        let packet = subsys::Packet::singular(ToOperatorSys::Observe(observation));
        self.control
            .send_packet(packet)
            .await
            .map_err(|e| format!("grammar: the operator's system: {e}"))
    }

    /// Stops the operator's system.
    pub async fn stop(self) -> Result<(), String> {
        self.control
            .stop(true)
            .await
            .map_err(|e| format!("grammar: the operator's system: {e}"))
    }
}
