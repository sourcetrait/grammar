use crate::*;

/// The lane a watched thread ran on.
pub type ThreadLane = Lane;

/// A cancelled eval whose thread had not ended when the engine was read.
pub type HungThread = HungEval;

/// The engine as the supervisor reads it: its watched threads, permits, and jobs.
pub type EngineReading = EngineHealth;

#[cereal::derived(Data)]
pub enum ToOperatorSys {
    Observe(Observation),
}

#[cereal::derived(Data)]
pub enum FromOperatorSys {
    Emergency(Emergency),
}
