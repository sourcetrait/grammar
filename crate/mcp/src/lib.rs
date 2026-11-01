pub(crate) mod engines;
pub(crate) mod error;
pub(crate) mod run;
pub(crate) mod server {
    pub(crate) mod cert;
    pub(crate) mod channel {
        pub(crate) mod face;
        pub(crate) mod hub;
    }
    pub(crate) mod controller;
    pub(crate) mod emergency;
    pub(crate) mod host;
    pub(crate) mod identity;
    pub(crate) mod oneshot;
    pub(crate) mod operator;
    pub(crate) mod purview;
    pub(crate) mod remote;
    pub(crate) mod run;
    pub(crate) mod tool {
        pub(crate) mod call;
        pub(crate) mod channel_close;
        pub(crate) mod channel_open;
        pub(crate) mod channel_sync;
        pub(crate) mod channel_verified;
        pub(crate) mod commit;
        pub(crate) mod config_channel;
        pub(crate) mod common;
        pub(crate) mod handler;
        pub(crate) mod info;
        pub(crate) mod inspect;
        pub(crate) mod interface;
        pub(crate) mod equip;
        pub(crate) mod execute;
        pub(crate) mod kill;
        pub(crate) mod new;
        pub(crate) mod processes;
        pub(crate) mod purview_configure;
        pub(crate) mod purview_extend;
        pub(crate) mod purviews;
        pub(crate) mod purview;
        pub(crate) mod remote_channel_close;
        pub(crate) mod remote_channel_connect;
        pub(crate) mod remote_channel_listen;
        pub(crate) mod remote_channel_set;
        pub(crate) mod remote_channels;
        pub(crate) mod remote_engine_connect;
        pub(crate) mod remote_engine_import;
        pub(crate) mod remote_engine_set;
        pub(crate) mod remote_engines;
        pub(crate) mod rerun;
    }
    #[cfg(test)]
    mod tests {
        mod emergency;
        mod face;
    }
}
#[cfg(test)]
mod tests {
    mod config;
    mod deny;
    mod engines;
}
pub(crate) mod cli;
pub(crate) mod config;
pub(crate) mod consts;
pub(crate) mod home;
pub(crate) mod scratch;
pub mod guts;

pub(crate) use crate::{
    config::{
        AGENT_CONFIG_FILE, Config, DEFAULT_NAMESPACE, DenySet, TEST_NAMESPACE, default_id,
        default_work_dir, expand_defaults, expand_path,
    },
    home::{
        TMP_DEFAULT, base_dirs, check_base_dirs, env_dir, home_var, home_vars, uenv_partner,
        uenv_standard,
    },
    scratch::{
        make_channel_dir, make_scratch_dirs, remove_channel_dir, remove_scratch_dirs, tmp_dir,
        transient_dir,
    },
    mcp::ServiceExt,
    engines::{
        EngineChange, EngineDefaults, EngineEntry, KIND_ENGINE_INVALID, KIND_ENGINE_NOT_OPEN,
        KnownHosts, engine_endpoint, import_ssh, read_engines, set_engine, unset_engine,
        write_known_hosts,
    },
    error::error_to_call_result,
    server::{
        cert::ensure_cert_profile,
        channel::{
            face::{
                ChannelFace, CloseSignal as ChannelCloseSignal, Hub, channel_failed, peer_failed,
                spawn_signal_sweep,
            },
            hub::{FROM_MCP, open_packet, start as start_channel_hub},
        },
        controller::{McpController, log_closed, log_status},
        emergency::{EmergencyPolicy, emergency_log_path},
        identity::{mint_nonce, process_identity},
        tool::{
            call::CallParams,
            common::{
                Route, envelope_text, now_millis, serde_to_nu, success, success_value,
                success_void,
            },
            interface::{
                INTERFACE_TOOL, InterfaceParams, interface_description, interface_engine_for,
                render_signatures,
            },
            execute::ExecuteParams,
            remote_engines::{RemoteEngineRow, engine_invalid},
            rerun::RerunParams,
        },
    },
    run::{
        lock_namespace, parse_deniable,
    },
};

pub(crate) use crate::server::{
    host::{HostRoute, ensure_substrate, init_engine},
    oneshot::run_oneshot,
    operator::{OperatorWatch, spawn_observer},
    purview::{Views, pattern_delta, unknown_purview},
    remote::RemoteRoute,
    run::run_server,
    tool::common::{NuSh, unexpected},
};

pub(crate) use sourcetrait_grammar_client::{
    Endpoint, FromRemote, RemoteAsk, RemoteEmergency, RemoteHandle, RemoteSysParams,
    RemoteSysPaths, RemoteSystem, SSH_PORT, SessionPhase, SessionState, ToRemote, WORKER_COMMAND,
    check_alias, check_command, check_host_keys, check_instance, check_key, check_path,
    check_word, known_host_keys, known_hosts_name, parse_address,
};

pub(crate) use sourcetrait_grammar_engine::{
    self as engine, EVAL_STACK_SIZE, EngineHandle, EngineParams, EngineSystem, FromEngine,
    GRIMM, HostAnswer, HostAsk, HostStop, LintEngine, NU_VERSION, NamepathPattern,
    NamepathStr, PURVIEW_ALL, PURVIEW_DEFAULT, ParseEngine, PinAsk, PluginInfo,
    PurviewConfigureRequest, PurviewListRequest, PurviewResolution, PurviewResolveRequest,
    PurviewRigRequest, ToEngine, analyze_definition, build_call_source,
    build_execute_source, definition_file, ensure_default_purview, ensure_equip_store,
    ensure_plugin_registry, equip_root, file_start, grimm_signatures, index_from_nuon,
    install_builtins, lib_dir, lint_block, list_registered_plugins, prepare_send, purview_ref,
    purviews_path, rig_dir, rig_meta_path, seed_builtin_purviews, typedef_text,
};

pub(crate) use sourcetrait_grammar_channel::{
    ChannelHandle, ChannelSendError, ChannelStatus, ChannelSyncError, ChannelSysParams,
    ChannelSysPaths, ChannelSystem, ChannelVerifyError, CloseHub, CloseRequest, EmitError,
    EmitRequest, ExpireRequest, FromChannelSys, HostRequest, HubNotice, Line, OpenRequest,
    ReadyRequest, SendRequest, SendVerdict, SpamThresholds, StatusRequest, StoppedNotice,
    SyncRequest, ThresholdsRequest, escape_line, mint_msg_id, render_nuon, render_packet,
};

pub(crate) use sourcetrait_grammar_interface::{
    ChannelCommand, ChannelOpenParams, CommitParams, ConfigChannelParams, EngineInterfaceCommand,
    EquipParams, InspectParams, InterfaceCommand, KillParams, NewParams, PeerCommand,
    PurviewConfigureParams, PurviewExtendParams, PurviewParams, RemoteAliasParams,
    RemoteChannelCloseParams, RemoteConnectParams, RemoteEngineCommand, RemoteEngineImportParams,
    RemoteEngineSetParams, RemoteListenParams, RemoteSetParams, parse_command,
};

pub(crate) use sourcetrait_grammar_peer::{
    ExpandDefault, FromPeerSys, IdentityKeys, InboxRequest, Payload, PeerHandle, PeerSysParams,
    PeerSysPaths, PeerSystem, REMOTES_FILE, Relay, RemoteCloseRequest, RemoteConnectRequest,
    RemoteGetRequest, RemoteListRequest, RemoteListenRequest, RemoteSendRequest,
    RemoteSetRequest, RemoteUnsetRequest, edit_remotes,
};

pub(crate) use sourcetrait_grammar_protocol::{
    ChannelEpoch, ChannelPhase, ChannelSendData, ControlRequest, EngineRequest, EngineResponse,
    FromEngineSys, LIMITS_DEFAULT, LOCAL_HOST, MAX_FRAME_BYTES, PreparedSend, PurviewAnswer,
    PurviewRequest, PurviewRow, PurviewView, RemoteResult, RemoteSendData, RuntimeError,
    SessionAsk, SpamThresholdsData, ToEngineSys, TransportError, WorkerInfo,
};

pub(crate) use sourcetrait_grammar_protocol::{
    COMMANDS, CallRequest, CheckSummary, CommitResult, ControlAnswer, Diagnostic, EngineCommand,
    EngineError, EngineHealth, EngineResult, ExecuteRequest, Host, INTERFACE, InspectDoc,
    RerunRequest, SignaturesDoc, ValueAnswer, command_by_words,
};

pub(crate) use sourcetrait_grammar_protocol::{Emergency, EmergencyKind, Observation};

pub(crate) use sourcetrait_grammar_operator::{
    FromOperatorSys, OperatorHandle, OperatorSysParams, OperatorSysPaths, OperatorSystem,
    SAMPLE_INTERVAL, SupervisorConfig, check_fraction, clear_pins, effective_supervisor,
    install_child_subreaper, pin,
};

pub(crate) use sourcetrait_grammar_scratch::{LOCK_FILE, LockRecord, NamespaceLock};

pub(crate) use nuval::{NuVal, NuonStyle, TryFromNu, TryIntoNu};

pub(crate) use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
    path::{PathBuf,Path},
    process,
    sync::{
        Arc, LazyLock, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

pub(crate) use clap::Parser;

pub(crate) use srcert_lib as lib_cert;

pub(crate) mod dirs {
    pub(crate) use directories::BaseDirs;
}

pub(crate) mod nu {
    pub(crate) use nu_parser::parse;
    pub(crate) use nu_protocol::{
        Record, Span, Value,
        engine::{EngineState, StateWorkingSet},
    };
    pub(crate) use nuon::{ToNuonConfig, from_nuon, to_nuon};
}

pub(crate) use sourcetrait_common::{datum, subsys};

pub(crate) mod ser {
    pub(crate) use ::serde::{Deserialize, Serialize};
}

pub(crate) mod schema {
    pub(crate) use schemars::JsonSchema;
}

pub(crate) mod mcp {
    pub(crate) use rmcp::{
        ErrorData, ServerHandler, ServiceExt,
        handler::server::router::tool::ToolRouter,
        handler::server::wrapper::Parameters,
        model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig},
        tool, tool_handler, tool_router,
        transport::stdio,
    };
}

pub(crate) mod tls {
    pub(crate) use tokio_rustls::TlsAcceptor;
    pub(crate) use tokio_rustls::rustls::ServerConfig;
    pub(crate) use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
}

pub(crate) mod ws {
    pub(crate) use tokio_tungstenite::accept_async;
    pub(crate) use tokio_tungstenite::tungstenite::Message;
    pub(crate) use tokio_tungstenite::tungstenite::protocol::CloseFrame;
    pub(crate) use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
}

pub(crate) use futures_util::{SinkExt, StreamExt};
pub(crate) use tokio_rustls::rustls::pki_types::pem::PemObject;

pub(crate) mod tk {
    pub(crate) use tokio::{
        spawn,
        net::{TcpListener, TcpStream},
        task::{JoinHandle, spawn_blocking},
        sync::{
            Mutex as AsyncMutex, oneshot,
            mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
        },
        time::{Duration as TkDuration, sleep, timeout},
    };
    #[cfg(unix)]
    pub(crate) use tokio::signal::unix::{SignalKind, signal};
}

pub(crate) mod json {
    pub(crate) use serde_json::{Value, to_value};
}

pub use crate::{
    cli::{
        Cli, CliEnv, CliTool, CliCmd,
    },
    config::{
        DeniableTool,
    },
    error::{
        GrammarMcpResult, GrammarMcpError,
    },
    run::{
        run_main, run_with
    },
};

pub use sourcetrait_grammar_protocol::ChannelMode;
