pub(crate) mod endpoint;
pub(crate) mod known_hosts;
pub(crate) mod session;
pub(crate) mod ssh;
pub(crate) mod system;
#[cfg(test)]
mod tests {
    mod known_hosts;
    mod ssh;
}

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

pub(crate) use crate::endpoint::pinned_key;

pub(crate) use sourcetrait_common::{agnostic, cereal, subsys};
pub(crate) use sourcetrait_grammar_protocol::{
    AUTH_DEADLINE, ClientHello, DecodeBudget, Emergency, EngineRequest, EngineResponse, FrameCodec,
    HELLO_BUDGET, HelloReply, HostAnswer, Instance, LOCAL_HOST, Limits, McpFrame, PING_INTERVAL,
    Ping, REPLY_DEADLINE, SESSION_IDLE, SessionAsk, SessionClose, TransportError, WorkerFrame,
    WorkerHello, check_reply, decode_payload, payload, valid_name,
};

pub(crate) use russh::keys;

pub(crate) mod kh {
    pub(crate) use russh::keys::ssh_key::known_hosts::{Entry, HostPatterns, Marker};
    pub(crate) use russh::keys::ssh_key::public::KeyData;
}

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
    endpoint::{
        Endpoint, SSH_PORT, WORKER_COMMAND, check_alias, check_command, check_host_keys,
        check_instance, check_key, check_path, check_port, check_word, known_hosts_name,
        parse_address,
    },
    known_hosts::known_host_keys,
    session::{Responder, Session, SessionEvent},
    ssh::{RemoteSession, open},
    system::{
        CloseSessionRequest, ConnectAnswer, ConnectRequest, FromRemote, RemoteAsk, RemoteEmergency,
        RemoteHandle, RemoteRequest, RemoteResponse, RemoteSysConfig, RemoteSysParams,
        RemoteSysPaths, RemoteSystem, SessionClosed, SessionPhase, SessionState, SessionStates,
        StatesRequest, ToRemote,
    },
};
