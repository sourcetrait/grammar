pub(crate) mod cork {
    pub(crate) mod consts;
    pub(crate) mod error;
    pub(crate) mod fs;
}
pub(crate) mod error;
pub(crate) mod plugin;
pub(crate) mod render;
pub(crate) mod soak_dir;
pub(crate) mod soak_str;
pub(crate) mod str_soak;

pub(crate) use crate::cork::{
    consts,
    error::{CorkError, ErrorMeta, ErrorTrait, PathKind, PathOp},
    fs::PathSpan,
};

pub(crate) use crate::{
    error::{NuType, SoakError, SoakResult},
    render::{render_dir, render_str},
    soak_dir::SoakDirCommand,
    soak_str::SoakStrCommand,
    str_soak::StrSoakCommand,
};

pub(crate) use std::{
    collections::HashMap,
    ffi::OsString,
    fmt::Display,
    fs,
    io,
    path::{Path, PathBuf},
    time::Duration,
};

pub(crate) mod nu {
    pub(crate) use nu_path::{expand_path, expand_path_with};
    pub(crate) use nu_plugin::{
        EngineInterface, EvaluatedCall, Plugin, PluginCommand, SimplePluginCommand,
    };
    pub(crate) use nu_protocol::{
        Category, Example, LabeledError, Record, ShellError, Signature, Span, SyntaxShape, Type,
        Value,
        shell_error::io::{ErrorKind as IoErrorKind, IoError},
    };
}

pub use crate::plugin::SoakPlugin;
