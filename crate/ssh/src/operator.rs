//! The operator's handler and observer: its emergencies to the policy, the engine seen.
use crate::*;

/// The controller's handler of the operator: its emergencies to the policy.
pub(crate) struct OperatorWatch {
    /// The controller's policy, which logs each emergency and sends it to the MCP.
    pub(crate) policy: EmergencyPolicy,
    /// The controller's shutdown flag, under which the operator's end goes unsaid.
    pub(crate) stopping: Arc<AtomicBool>,
}

impl subsys::Handler<OperatorSystem> for OperatorWatch {
    fn on_packet(&mut self, pkt: subsys::Packet<FromOperatorSys>) -> impl Future<Output = Option<subsys::Packet<FromOperatorSys>>> + 'static + Send {
        let FromOperatorSys::Emergency(emergency) = pkt.take_msg();
        self.policy.respond(emergency);
        std::future::ready(None)
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

/// Each interval, the operator given the engine's health and a session's presence.
pub(crate) fn spawn_observer(
    engine: Arc<EngineHandle>,
    operator: Arc<OperatorHandle>,
    asker: Arc<Asker>,
) -> tk::JoinHandle<()> {
    tk::spawn(async move {
        loop {
            tk::sleep(SAMPLE_INTERVAL).await;
            let observation = Observation {
                engine: engine_health(&engine).await,
                listening: asker.attached(),
            };
            if let Err(e) = operator.observe(observation).await {
                eprintln!("grammar: {e}");
                return;
            }
        }
    })
}
