pub(crate) mod blocked;
pub(crate) mod cache;
pub(crate) mod config_dirs;
pub(crate) mod cycle;
pub(crate) mod dispatch;
pub(crate) mod embed;
pub(crate) mod engine;
pub(crate) mod equip {
    pub(crate) mod lifecycle;
    pub(crate) mod purview;
    pub(crate) mod signature;
    pub(crate) mod store;
    pub(crate) mod validate;
}
pub(crate) mod execute;
pub(crate) mod executor;
pub(crate) mod lint;
pub(crate) mod mode;
pub(crate) mod msg;
pub(crate) mod nuapi {
    pub(crate) mod grimm {
        pub(crate) mod category;
        pub(crate) mod channel {
            pub(crate) mod send;
        }
        pub(crate) mod common;
        pub(crate) mod config;
        pub(crate) mod dbg;
        pub(crate) mod def;
        pub(crate) mod remote {
            pub(crate) mod channel {
                pub(crate) mod send;
            }
        }
    }
    pub(crate) mod print;
}
pub(crate) mod namepath;
pub(crate) mod nuin;
pub(crate) mod params;
pub(crate) mod parse_engine;
pub(crate) mod plugins;
pub(crate) mod schema;
pub(crate) mod send;
pub(crate) mod serve;
pub(crate) mod system;
pub(crate) mod teardown;
pub(crate) mod template;
#[cfg(feature = "test-hooks")]
pub(crate) mod test_hooks;
#[cfg(test)]
mod tests {
    mod equip;
    mod health;
    mod namepath;
    mod purview;
    mod schema;
}

pub(crate) use crate::{
    blocked::shadow_host_fatal_decls,
    cache::call_log_dir,
    config_dirs::{config_dirs, init_config_dirs},
    dispatch::{
        DEFAULT_TIMEOUT_MS, Dispatch, HungRegistry, HungWatch, InFlight, InFlightKind,
        READY_POOL_TARGET, dispatch_pooled, dispatch_stateful, hung_snapshot, now_millis,
        register_hung, teardown_all_in_flight,
    },
    embed::{StatefulEngine, eval_in_process, eval_stateless, merge_env_no_chdir},
    engine::startup,
    equip::{
        purview::{
            load_purviews, purview_add_rig, purview_configure, purview_list, purview_remove_rig,
            purview_resolve,
        },
        store::{
            EQUIP_DIR, EQUIP_KINDS, KIND_LIB, KIND_RIG, META_FILE, check_equip_name, equip_dir,
            index_to_nuon, is_valid_equip, is_valid_ident, is_valid_module_path, keypair_dir,
            lib_meta_path, load_lib_index, load_rig_source, meta_to_nuon, rig_docs_dir, run_git,
        },
        validate::{HELP_WORDS, NAME_DENYLIST, is_reserved_term, validate_equip_source},
    },
    execute::{EXECUTE, contract_signature},
    mode::Mode,
    nuapi::{
        grimm::{
            category::*,
            channel::send::GrimmChannelSend,
            common::{
                NuapiCall, STDERR_FILE, STDOUT_FILE, asked_host, data_shape, event_id_record,
                event_id_type, register_nuapi, require_record_or_table, sendable, unexpected,
            },
            config::{GrimmGetConfig, GrimmGetConfigAll, GrimmPinConfig},
            dbg::GrimmDbg,
            remote::channel::send::{GrimmRemoteChannelSend, GrimmRemoteChannelSendWith},
        },
        print::HostPrint,
    },
    nuin::{SignatureParameterTrait, SignatureTrait},
    params::{cache_home, data_home, params, started},
    parse_engine::{LIB_DIRS_CONST, PLUGIN_DIRS_CONST, dirs_value, set_dirs_const},
    plugins::{load_plugin_decls, registry_mtime, registry_path},
    schema::{data_typedef, result_typedef, signature_group},
    serve::{EngineCore, serve_call, serve_control, serve_execute, serve_rerun},
    system::HostLink,
    teardown::{kill_plugin_subprocesses, make_tracker, tree_kill},
    template::render_nuon,
};

pub(crate) use sourcetrait_grammar_protocol::{
    ChannelSendData, ChannelSpamErrorEmergency, PreparedSend, PurviewRow, PurviewView,
    RemoteSendData, RuntimeError, SendFile, reserved_prefix, safe_dest,
};

pub(crate) use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
    ops::ControlFlow,
    os::unix::fs::MetadataExt,
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
    process,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub(crate) mod nu {
    pub(crate) use nu_cmd_lang::create_default_context;
    pub(crate) use nu_cmd_extra::add_extra_command_context;
    pub(crate) use nu_cmd_plugin::add_plugin_command_context;
    pub(crate) use nu_command::add_shell_command_context;
    pub(crate) use nu_command::tls::CRYPTO_PROVIDER;
    pub(crate) use nu_config::{CliOverrides, EnvAccess, NushellConfigDirs, SystemEnv, resolve_paths};
    pub(crate) use nu_engine::convert_env_values;
    pub(crate) use nu_engine::eval_block;
    pub(crate) use nu_engine::CallExt;
    pub(crate) use nu_engine::command_prelude::Call;
    pub(crate) use nu_parser::parse;
    pub(crate) use nu_parser::{FlatShape, flatten_block};
    pub(crate) use nu_plugin_engine::load_plugin_file;
    pub(crate) use nu_protocol::{
        BlockId, ByteStreamSource, ByteStreamType, Category, CollectionColumns, Config, DeclId,
        IntoValue, Module, PipelineData, PluginRegistryFile, Example,
        PluginRegistryItemData, Record, ShellError, Signals, Signature, Span, SyntaxShape, Type,
        Value, record, write_all_and_flush,
        ast::{
            Argument, Block, Comparison, Expr, Expression, ExternalArgument, ListItem,
            Operator, Pattern, RecordItem,
        },
        debugger::WithoutDebug,
        engine::{Command, EngineState, Job, Jobs, Mail, Stack, StateWorkingSet, ThreadJob},
    };
    pub(crate) use nu_protocol::ast::Call as AstCall;
    pub(crate) use nu_protocol::shell_error::generic::GenericError;
    pub(crate) use nu_std::load_standard_library;
    pub(crate) use nuon::{ToNuonConfig, from_nuon, to_nuon};
}

pub(crate) use nu::{CallExt, IntoValue};

pub(crate) use nutype::{NuField, NuType, NuTypeError};

pub(crate) use nuval::{NuVal, NuonStyle, TryFromNu, TryIntoNu, wide};

pub(crate) use sourcetrait_common::{agnostic, cereal, datum, subsys};

pub(crate) mod ser {
    pub(crate) use ::serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
}

pub(crate) mod tk {
    pub(crate) use tokio::{
        spawn,
        task::spawn_blocking,
        sync::{
            Mutex as AsyncMutex, OwnedSemaphorePermit, RwLock as AsyncRwLock, Semaphore,
            oneshot,
            mpsc::{UnboundedSender, unbounded_channel},
        },
        time::{Duration as TkDuration, timeout},
    };
}

pub use crate::{
    cache::{CacheKind, DEFINITION_FILE, cache_dir, definition_file, remove_call_cache},
    cycle::detect_import_cycle,
    embed::EVAL_STACK_SIZE,
    engine::NU_VERSION,
    equip::{
        lifecycle::{
            check_equip, check_source_dir, commit_equip, establish_equip, install_equip,
            scaffold_leaf, scaffold_leaf_exists, uninstall_equip,
        },
        purview::{
            PURVIEW_ALL, PURVIEW_CORE, PURVIEW_CURRENT, PURVIEW_DEFAULT, PURVIEW_REF,
            ensure_default_purview, purview_ref, purviews_path, seed_builtin_purviews,
        },
        signature::{index_node, inspect_impl, render_signatures_within},
        store::{
            AlreadyRegistered, IndexFunction, IndexModule, LibIndex, RigIndex, RigLocks,
            ensure_equip_store, equip_namepath_of, equip_repo_lock, equip_root,
            index_from_nuon, install_builtins, lib_dir, load_index, registered_rig_names,
            require_rig, rig_dir, rig_meta_path,
        },
        validate::{DocEntry, ValidationResult},
    },
    execute::{ExecuteDef, analyze_definition},
    executor::{Executor, eval_concurrency_cap},
    lint::lint_block,
    msg::{FromEngine, HostAsk, PinAsk, ToEngine},
    namepath::{
        Namepath, NamepathPattern, NamepathRef, NamepathStr, PATTERN_ALL, PATTERN_CURRENT,
        resolve_builtin,
    },
    nuapi::grimm::common::{GRIMM, grimm_signatures},
    params::{EngineParams, init},
    parse_engine::{LintEngine, ParseEngine, file_start, span_text, wrap_as_module},
    plugins::{ensure_plugin_registry, list_registered_plugins},
    schema::{check_value, typedef_text},
    send::prepare_send,
    system::{EngineHandle, EngineSysConfig, EngineSysParams, EngineSysPaths, EngineSystem},
    template::{build_call_source, build_execute_source},
};

pub use sourcetrait_grammar_protocol::{
    BUILTINS, CallDoc, CallRequest, CheckSummary, CommitResult, ControlAnswer, ControlRequest,
    Diagnostic, EngineCommand, EngineError, EngineHealth, EngineResult, ExecuteRequest,
    FromEngineSys, Host, HostAnswer, HostStop, HungEval, InspectDoc, LINT_VIOLATION_CAP, Lane,
    ModuleDoc, PluginInfo, ProcessRow, PurviewAnswer, PurviewConfigureRequest, PurviewListRequest,
    PurviewRequest, PurviewResolution, PurviewResolveRequest, PurviewRigRequest, RerunRequest,
    RigDoc, Severity, SignaturesDoc, Source, ToEngineSys, ValueAnswer, builtin_equip,
    builtin_name, display_equip, span_to_line_col,
};
