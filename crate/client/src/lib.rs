pub(crate) mod endpoint;
pub(crate) mod session;
pub(crate) mod ssh;
pub(crate) mod system;

pub(crate) use std::{
    borrow::Cow,
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

pub(crate) use futures_util::{
    FutureExt, SinkExt, StreamExt,
    future::{BoxFuture, Shared, join_all},
};

pub(crate) use sourcetrait_common::{agnostic, cereal, subsys};
pub(crate) use sourcetrait_grammar_protocol::{
    AUTH_DEADLINE, ClientHello, DecodeBudget, Emergency, EngineRequest, EngineResponse, FrameCodec,
    HELLO_BUDGET, HelloReply, HostAnswer, Instance, LOCAL_HOST, Limits, McpFrame, PING_INTERVAL,
    Ping, REPLY_DEADLINE, SESSION_IDLE, SessionAsk, SessionClose, TransportError, WorkerFrame,
    WorkerHello, check_reply, decode_payload, payload, valid_name,
};

pub(crate) use russh::keys;

pub(crate) mod ser {
    pub(crate) use ::serde::{Deserialize, Serialize};
}

pub(crate) mod tk {
    pub(crate) use tokio::{
        io::{AsyncRead, AsyncWrite, AsyncWriteExt, DuplexStream, duplex},
        net::TcpStream,
        spawn,
        sync::{
            Mutex as AsyncMutex,
            mpsc::{Sender, UnboundedReceiver, UnboundedSender, unbounded_channel},
            oneshot,
        },
        task::JoinHandle,
        time::{sleep, timeout},
    };
}

pub(crate) mod tku {
    pub(crate) use tokio_util::bytes::Bytes;
    pub(crate) use tokio_util::codec::{FramedRead, FramedWrite};
}

pub use crate::{
    endpoint::{ENGINE_NAMESPACE, Endpoint, SSH_PORT},
    session::{Responder, Session, SessionEvent},
    ssh::{RemoteSession, open},
    system::{
        FromRemote, RemoteAsk, RemoteEmergency, RemoteHandle, RemoteRequest, RemoteResponse,
        RemoteSysConfig, RemoteSysParams, RemoteSysPaths, RemoteSystem, ToRemote,
    },
};
