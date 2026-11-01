//! The operator's system: started on a namespace's homes, the observations its
//! manager passes answered with the emergencies they confirm, each a packet.

use std::time::Duration;

use sourcetrait_common::subsys;
use sourcetrait_common::testing::prelude::*;
use sourcetrait_grammar_operator::{
    Emergency, EngineReading, FromOperatorSys, HungThread, Observation, OperatorHandle,
    OperatorSysParams, OperatorSysPaths, OperatorSystem, SupervisorConfig, ThreadLane,
};
use tokio::sync::mpsc;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// The manager's side: each emergency the operator sends, kept for the test.
struct Collect(mpsc::UnboundedSender<Emergency>);

impl subsys::Handler<OperatorSystem> for Collect {
    fn on_packet(&mut self, pkt: subsys::Packet<FromOperatorSys>) -> impl Future<Output = Option<subsys::Packet<FromOperatorSys>>> + 'static + Send {
        let FromOperatorSys::Emergency(emergency) = pkt.take_msg();
        let _ = self.0.send(emergency);
        std::future::ready(None)
    }
}

/// Lines no level of the test process reaches, so only observations raise.
const LINES: SupervisorConfig = SupervisorConfig {
    cpu_warn_fraction: 1.0,
    ram_warn_fraction: 1.0,
    vram_warn_headroom_mib: 0,
    disk_warn_fraction: 1.0,
};

/// A thread on `lane`, cancelled at the epoch, so long past its grace.
fn hung(
    nonce: &str,
    lane: ThreadLane,
) -> HungThread {
    HungThread {
        nonce: nonce.to_string(),
        tool: "execute".to_string(),
        lane,
        started_at: 0,
        cancelled_at: 0,
    }
}

/// The engine holding `hung` against `permits_cap` permits, the Channel open.
fn observing(
    hung: Vec<HungThread>,
    permits_cap: u64,
) -> Observation {
    Observation {
        engine: Some(EngineReading {
            permits_held: hung.len() as u64,
            hung,
            permits_cap,
            jobs: 0,
        }),
        listening: true,
    }
}

/// The operator started on homes under `root`, its emergencies sent to `tx`.
async fn start(
    root: &std::path::Path,
    tx: mpsc::UnboundedSender<Emergency>,
) -> OperatorHandle {
    let paths = OperatorSysPaths::new(root.join("data"), root.join("cache"));
    let params = OperatorSysParams {
        test: false,
        supervisor: LINES,
    };
    OperatorHandle::start(paths, params, Collect(tx))
        .await
        .expect("the operator starts")
}

/// The next emergency, or none within half a second.
async fn next(rx: &mut mpsc::UnboundedReceiver<Emergency>) -> Option<Emergency> {
    tokio::time::timeout(Duration::from_millis(500), rx.recv())
        .await
        .ok()
        .flatten()
}

#[tested]
fn an_observed_hang_comes_back_once() {
    let t = testing::test!({ .using_temp_dir() });
    let rt = tokio::runtime::Runtime::new().expect("a runtime");
    rt.block_on(async {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let operator = start(t.temp_dir(), tx).await;
        operator.observe(observing(vec![hung("w1", ThreadLane::Stateful)], 8)).await.expect("observed");
        let raised = next(&mut rx).await.expect("the hang raised");
        assert!(matches!(&raised, Emergency::HungEngineThread(h) if h.nonce == "w1"), "got {raised:?}");
        operator.observe(Observation { engine: None, listening: true }).await.expect("observed");
        operator.observe(observing(vec![hung("w1", ThreadLane::Stateful)], 8)).await.expect("observed");
        assert_eq!(next(&mut rx).await, None, "a hang still active is not raised twice");
        operator.stop().await.expect("the operator stops");
    });
}

#[tested]
fn every_stateless_permit_hung_is_critical() {
    let t = testing::test!({ .using_temp_dir() });
    let rt = tokio::runtime::Runtime::new().expect("a runtime");
    rt.block_on(async {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let operator = start(t.temp_dir(), tx).await;
        operator.observe(observing(vec![hung("w1", ThreadLane::Stateless)], 1)).await.expect("observed");
        let mut kinds = Vec::new();
        while let Some(raised) = next(&mut rx).await {
            kinds.push(raised.kind().name());
        }
        kinds.sort_unstable();
        assert_eq!(kinds, vec!["critical", "hung_engine_thread"]);
        operator.stop().await.expect("the operator stops");
    });
}
