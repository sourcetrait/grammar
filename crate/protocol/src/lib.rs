pub(crate) mod channel;
pub(crate) mod codec;
pub(crate) mod command;
pub(crate) mod engine;
pub(crate) mod error;
pub mod fixture;
pub(crate) mod hello;
pub(crate) mod limits;
pub(crate) mod operator;
pub(crate) mod peer;
pub(crate) mod purview;
pub(crate) mod session;
pub(crate) mod value;

pub(crate) use std::{fmt, io};

pub(crate) use nuval::wide;

pub(crate) use sourcetrait_common::{cereal, datum, subsys};

pub(crate) mod tk {
    pub(crate) use tokio::io::{AsyncRead, AsyncWrite};
}

pub(crate) mod tku {
    pub(crate) use tokio_util::bytes::{Buf, Bytes, BytesMut};
    pub(crate) use tokio_util::codec::{Decoder, Encoder, Framed};
}

pub use crate::{
    channel::{
        ChannelEpoch, ChannelMode, ChannelPhase, ChannelSendData, ChannelState, EVENT_BYTES_KEY,
        EVENT_SPILL_DEST, EVENT_SPILL_KEY, MAX_FRAME_BYTES, MCP_RESERVED_PREFIX, NOTIFICATION_CAP,
        REMOTE_RESERVED_PREFIX, RemoteSendData, SYNC_QUEUE_CAP, SpamThresholdsData, reserved_prefix,
    },
    codec::{
        CODEC_ID, FrameCodec, FrameError, FrameStage, PREFACE, SCHEMA_ID, check_codec,
        decode_payload, framed, payload,
    },
    command::{
        ArgShape, ArgSpec, COMMANDS, CommandKind, CommandSpec, HELP_WORDS, INTERFACE,
        command_by_words, command_spec, render_command_list, render_signatures,
    },
    engine::{
        BUILTINS, CallDoc, CallRequest, CheckSummary, CommitResult, ControlAnswer, ControlRequest,
        Diagnostic, EngineCommand, EngineError, EngineHealth, EngineResult, ExecuteRequest,
        FromEngineSys, Host, HungEval, InspectDoc, LINT_VIOLATION_CAP, Lane, ModuleDoc,
        ProcessRow, RerunRequest, RigDoc, Severity, SignaturesDoc, Source, ToEngineSys,
        ValueAnswer, builtin_equip, builtin_name, display_equip, span_to_line_col,
    },
    error::{LOCAL_HOST, RuntimeError, TransportError},
    hello::{
        AUTH_DEADLINE, ClientHello, HELLO_BUDGET, HELLO_DEADLINE, HelloReply, Instance,
        LIMITS_DEFAULT, LIMITS_HARD, Limits, PROTOCOL_VERSION, ProtocolVersion, REPLY_DEADLINE,
        WorkerHello, WorkerProfile, check_reply, valid_name,
    },
    limits::{DECODE_BUDGET_DEFAULT, DECODE_HEAP_HARD, DecodeBudget, FRAME_DEFAULT, FRAME_HARD, HELLO_FRAME_MAX},
    operator::{
        BackgroundJobsWarningEmergency, ChannelSpamErrorEmergency, ChannelSpamWarningEmergency,
        CpuWarningEmergency, CriticalEmergency, DiskWarningEmergency, Emergency, EmergencyKind,
        HungEngineThreadEmergency, Observation, RamWarningEmergency, VramWarningEmergency,
    },
    peer::{
        MODEL_CONNECTED, MODEL_DISCONNECTED, MODEL_SENT, MODEL_UNSENT, PEER_ID_MAX, REMOTE_COMMANDS,
        RemoteChannelEvent, RemoteChannelRow, RemoteFailure, RemoteLinkState, RemoteListenResult,
        RemotePacket, RemoteResult, safe_dest, valid_peer_id,
    },
    purview::{
        PurviewAnswer, PurviewConfigureRequest, PurviewListRequest, PurviewRequest,
        PurviewResolution, PurviewResolveRequest, PurviewRigRequest, PurviewRow, PurviewView,
    },
    session::{
        ASK_DEADLINE, DRAIN_DEADLINE, EngineRequest, EngineResponse, HostAnswer, HostStop, McpFrame,
        PING_INTERVAL, Ping, PluginInfo, PreparedSend, SESSION_IDLE, SendFile, SessionAsk,
        SessionClose, WorkerFrame, WorkerInfo,
    },
    value::{ValueLimitError, ValueLimitErrorKind, ValueLimits, check_error, check_value},
};

pub use nuval::{
    Bounded, CellPathMemberData, CompileErrorKind, ConfigErrorKind, ConfigWarningKind, DateData,
    GlobData, IoErrorKind, JobErrorKind, NetworkErrorKind, NuVal, NuValError, NuValErrorKind,
    NuValErrorLabel, NuValErrorSource, NuValResult, NuValSeverity, ParseErrorKind,
    ParseWarningKind, RangeData, ShellErrorKind, ShellWarningKind, SpanData, StdIoErrorKind,
    TypedRangeData,
};
