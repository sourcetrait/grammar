//! The operator's handler and observer: its emergencies routed, the engine seen.
use crate::*;

/// The controller's handler of the operator: its emergencies to the policy.
pub(crate) struct OperatorWatch {
    /// The controller's policy, which logs and announces each emergency.
    pub(crate) policy: EmergencyPolicy,
    /// The controller's shutdown flag, under which the operator's end goes unsaid.
    pub(crate) stopping: Arc<AtomicBool>,
}

impl subsys::Handler<OperatorSystem> for OperatorWatch {
    fn on_packet(&mut self, pkt: subsys::Packet<FromOperatorSys>) -> impl Future<Output = Option<subsys::Packet<FromOperatorSys>>> + 'static + Send {
        let policy = self.policy.clone();
        async move {
            let FromOperatorSys::Emergency(emergency) = pkt.take_msg();
            policy.respond(emergency).await;
            None
        }
    }

    fn on_sub(&mut self, sub: subsys::FromSub) -> impl Future<Output = ()> + Send {
        log_status("the operator's system", sub, &self.stopping);
        std::future::ready(())
    }

    fn on_closed(&mut self) -> impl Future<Output = ()> + Send {
        log_closed("the operator's system", &self.stopping);
        std::future::ready(())
    }
}

/// The engine's health, or none when it doesn't answer within a sample's interval.
async fn engine_health(engine: &EngineHandle) -> Option<EngineHealth> {
    match tk::timeout(SAMPLE_INTERVAL, engine.control(EngineCommand::Health)).await {
        Ok(Ok(ControlAnswer::Health(health))) => Some(health),
        _ => None,
    }
}

/// Whether the Channel is open, its system unanswering read as closed.
async fn channel_open(channel: &ChannelHandle) -> bool {
    match channel.request(StatusRequest).await {
        Ok(status) => !matches!(status.state.phase, ChannelPhase::Closed),
        Err(_) => false,
    }
}

/// Each interval, pass the operator the engine's health and the Channel's state.
pub(crate) fn spawn_observer(
    engine: Arc<EngineHandle>,
    operator: Arc<OperatorHandle>,
    channel: Arc<ChannelHandle>,
) -> tk::JoinHandle<()> {
    tk::spawn(async move {
        loop {
            tk::sleep(SAMPLE_INTERVAL).await;
            let reading = engine_health(&engine).await;
            let listening = channel_open(&channel).await;
            let observation = Observation {
                engine: reading,
                listening,
            };
            if let Err(e) = operator.observe(observation).await {
                eprintln!("{e}");
                return;
            }
        }
    })
}
