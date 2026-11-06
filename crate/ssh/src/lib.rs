pub(crate) mod cli;
pub(crate) mod config;
pub(crate) mod controller;
pub(crate) mod emergency;
pub(crate) mod error;
pub mod guts;
pub(crate) mod home;
pub(crate) mod host;
pub(crate) mod operator;
pub(crate) mod run;
pub(crate) mod scratch;
pub(crate) mod session;

pub(crate) use crate::{
    cli::Cli,
    config::{Config, DEFAULT_NAMESPACE},
    controller::{GrammarSshController, held, log_closed, log_status},
    emergency::EmergencyPolicy,
    home::Homes,
    host::{HostRoute, ensure_substrate, init_engine},
    operator::{OperatorWatch, spawn_observer},
    scratch::ScratchDirs,
    session::{Asker, SessionEnd, refuse, serve},
};

pub(crate) use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt, fs, io,
    path::{Path, PathBuf},
    process,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) use clap::Parser;

pub(crate) use futures_util::{SinkExt, StreamExt};

pub(crate) use sourcetrait_common::{datum, subsys};
pub(crate) use sourcetrait_grammar_engine::{
    self as engine, ControlAnswer, EVAL_STACK_SIZE, EngineCommand, EngineError, EngineHandle,
    EngineHealth, EngineIdentity, EngineResult, EngineSystem, FromEngine, HostAsk, PinAsk, ToEngine,
    prepare_send,
};
pub(crate) use sourcetrait_grammar_operator::{
    FromOperatorSys, OperatorHandle, OperatorSysParams, OperatorSysPaths, OperatorSystem,
    SAMPLE_INTERVAL, SupervisorConfig, append_line, check_fraction, effective_supervisor,
    emergency_line, emergency_log_path, pin,
};
pub(crate) use sourcetrait_grammar_protocol::{
    AI_TRANSIENT_DIR, ASK_DEADLINE, ClientHello, DRAIN_DEADLINE, DecodeBudget, Emergency, EngineRequest,
    EngineResponse, FrameCodec, FromEngineSys, HELLO_BUDGET, HELLO_DEADLINE, HelloReply, Host,
    HostAnswer, Instance, LIMITS_DEFAULT, Limits, McpFrame, NuVal, Observation, PurviewAnswer,
    PurviewRequest, RuntimeError, SESSION_IDLE, SessionAsk, SessionClose, SshFrame, SshInfo,
    SshProfile, ToEngineSys, TransientPaths, TransportError, ValueAnswer, decode_payload, payload,
    valid_name,
};
pub(crate) use sourcetrait_grammar_scratch::{
    HostProbe, LOCK_FILE, LockError, LockRecord, NamespaceLock, OwnedDir, Owner, current_uid, sweep,
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
    config::CONFIG_FILE,
    error::{GrammarSshError, GrammarSshErrorKind, GrammarSshResult},
    run::run_main,
};
