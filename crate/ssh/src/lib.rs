#[cfg(feature = "service")]
pub(crate) mod cli;
pub(crate) mod controller;
pub(crate) mod error;
pub(crate) mod instance {
    pub(crate) mod config;
    pub(crate) mod homes;
    pub(crate) mod name;
}
#[cfg(feature = "service")]
pub(crate) mod run;
pub(crate) mod runtime {
    pub(crate) mod scratch;
    pub(crate) mod start;
}
pub(crate) mod session;

#[cfg(feature = "service")]
pub(crate) use crate::cli::{GrammarSshCli, GrammarSshCmd};

pub(crate) use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt, fs, io,
    path::{Path, PathBuf},
    process,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

#[cfg(feature = "service")]
pub(crate) use clap::Parser;

pub(crate) use futures_util::{SinkExt, StreamExt};

pub(crate) use sourcetrait_common::{datum, subsys};
pub(crate) use sourcetrait_grammar_engine::{
    self as engine, ControlAnswer, EngineCommand, EngineError, EngineHandle, EngineHealth,
    EngineResult, EngineSystem, FromEngine, HostAsk, PinAsk, ToEngine, prepare_send,
};
pub(crate) use sourcetrait_grammar_operator::{
    FromOperatorSys, OperatorHandle, OperatorSysParams, OperatorSysPaths, OperatorSystem,
    SAMPLE_INTERVAL, SupervisorConfig, check_fraction, effective_supervisor, pin,
};
pub(crate) use sourcetrait_grammar_protocol::{
    ASK_DEADLINE, ClientHello, DRAIN_DEADLINE, DecodeBudget, Emergency, EngineRequest, EngineResponse,
    FrameCodec, FromEngineSys, HELLO_BUDGET, HELLO_DEADLINE, HelloReply, Host, HostAnswer,
    Instance, LIMITS_DEFAULT, Limits, McpFrame, NuVal, Observation, PurviewAnswer,
    PurviewRequest, RuntimeError, SESSION_IDLE, SessionAsk, SessionClose, ToEngineSys,
    TransportError, ValueAnswer, WorkerFrame, WorkerInfo, WorkerProfile, decode_payload, payload,
    valid_name,
};
pub(crate) use sourcetrait_grammar_scratch::{
    HostProbe, LOCK_FILE, LockError, LockRecord, NamespaceLock, OwnedDir, Owner, current_uid, sweep,
};

pub(crate) use crate::{
    controller::GrammarSshController,
    session::{Outbox, refuse, serve},
};

pub(crate) mod ser {
    pub(crate) use ::serde::Deserialize;
}

pub(crate) mod tk {
    pub(crate) use tokio::{
        fs::File,
        io::{AsyncRead, AsyncWrite, Stdin, stdin},
        runtime,
        select,
        spawn,
        sync::{
            Semaphore,
            mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
            oneshot,
        },
        task::{JoinHandle, JoinSet, spawn_blocking},
        time::{sleep, timeout},
    };
    #[cfg(unix)]
    pub(crate) use tokio::signal::unix::{Signal, SignalKind, signal};
    #[cfg(windows)]
    pub(crate) use tokio::signal::windows::{
        CtrlBreak, CtrlC, CtrlClose, ctrl_break, ctrl_c, ctrl_close,
    };
}

pub(crate) mod tku {
    pub(crate) use tokio_util::bytes::Bytes;
    pub(crate) use tokio_util::codec::{FramedRead, FramedWrite};
}

pub use crate::{
    error::{GrammarSshError, GrammarSshErrorKind, GrammarSshResult},
    instance::{
        config::{CONFIG_FILE, InstanceConfig, InstancePaths},
        homes::InstanceHomes,
        name::InstanceName,
    },
    runtime::{
        scratch::WorkerScratch,
        start::{Runtime, RuntimeIdentity},
    },
    session::SessionEnd,
};

#[cfg(feature = "service")]
pub use crate::run::run_main;
