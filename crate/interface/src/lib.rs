pub(crate) mod contract;
pub(crate) mod grammar;
pub(crate) mod params;
#[cfg(test)]
mod tests {
    mod contract;
}

pub(crate) use std::collections::HashMap;

pub(crate) use nuval::wide;

pub(crate) use sourcetrait_grammar_protocol::{
    ArgShape, ArgSpec, COMMANDS, ChannelMode, CommandKind, CommandSpec, Diagnostic, INTERFACE,
    LINT_VIOLATION_CAP, Source, command_spec, span_to_line_col,
};

pub(crate) mod nu {
    pub(crate) use nu_parser::parse;
    pub(crate) use nu_protocol::shell_error::generic::GenericError;
    pub(crate) use nu_protocol::{
        Category, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
        ast::{Argument, Block, Expr, Expression, ListItem},
        engine::{Call, Command, EngineState, Stack, StateWorkingSet},
    };
}

pub(crate) mod ser {
    pub(crate) use ::serde::{Deserialize, Serialize};
}

pub use crate::{
    contract::{
        ChannelCommand, EngineInterfaceCommand, InterfaceCommand, KIND_INTERFACE, PeerCommand,
        RemoteEngineCommand, parse_command,
    },
    grammar::interface_engine,
    params::{
        ChannelOpenParams, CommitParams, ConfigChannelParams, EquipParams, InspectParams,
        KillParams, NewParams, PurviewConfigureParams, PurviewExtendParams, PurviewParams,
        RemoteAliasParams, RemoteChannelCloseParams, RemoteConnectParams, RemoteEngineImportParams,
        RemoteEngineSetParams, RemoteListenParams, RemoteSetParams,
    },
};
