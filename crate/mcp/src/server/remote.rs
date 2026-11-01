//! The remote engines' handler: their workers' asks and emergencies, and their end.
use crate::*;

/// The controller's handler of the remote engines' system.
#[derive(Clone)]
pub(crate) struct RemoteRoute {
    /// The engine's route, which answers a worker's asks as it answers a local eval's.
    pub(crate) route: HostRoute,
    /// The controller's emergency policy, logging and announcing workers' emergencies.
    pub(crate) policy: EmergencyPolicy,
}

impl subsys::Handler<RemoteSystem> for RemoteRoute {
    fn on_packet(&mut self, pkt: subsys::Packet<FromRemote>) -> impl Future<Output = Option<subsys::Packet<FromRemote>>> + 'static + Send {
        let policy = self.policy.clone();
        async move {
            if let FromRemote::Emergency(RemoteEmergency { alias, emergency }) = pkt.take_msg() {
                policy.respond_from(Some(&alias), emergency).await;
            }
            None
        }
    }

    fn on_request(&mut self, pkt: subsys::Packet<FromRemote>) -> impl Future<Output = Option<ToRemote>> + 'static + Send {
        let route = self.route.clone();
        async move {
            let FromRemote::Ask(RemoteAsk { alias, ask }) = pkt.take_msg() else {
                return None;
            };
            Some(ToRemote::Answer(route.remote_answer(&alias, ask).await))
        }
    }

    fn on_sub(&mut self, sub: subsys::FromSub) -> impl Future<Output = ()> + Send {
        log_status("the remote engines' system", sub, &self.route.stopping);
        std::future::ready(())
    }

    fn on_closed(&mut self) -> impl Future<Output = ()> + Send {
        log_closed("the remote engines' system", &self.route.stopping);
        std::future::ready(())
    }
}
