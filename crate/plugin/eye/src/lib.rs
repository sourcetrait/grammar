pub(crate) mod dir;
pub(crate) mod error;
pub(crate) mod git;
pub(crate) mod path;
pub(crate) mod plugin;
pub(crate) mod tree;
#[cfg(test)]
mod tests {
    mod tree;
}

pub(crate) use crate::{
    dir::EyeDirCommand,
    error::{GlobSnafu, ReadSnafu, TreeResult, WalkSnafu, labeled_error},
    git::EyeGitCommand,
    tree::{Branch, Leaf, Listing, render},
};

#[cfg(test)]
pub(crate) use crate::tree::{size_label, wrap};

pub(crate) use snafu::ResultExt;

pub(crate) use std::{
    collections::HashMap,
    fmt::Display,
    fs,
    io,
    path::{Path, PathBuf},
};

pub(crate) mod nu {
    pub(crate) use nu_path::{expand_path, expand_path_with};
    pub(crate) use nu_plugin::{
        EngineInterface, EvaluatedCall, Plugin, PluginCommand, SimplePluginCommand,
    };
    pub(crate) use nu_protocol::{
        Category, Example, LabeledError, Signature, Span, SyntaxShape, Value,
    };
}

pub use crate::plugin::EyePlugin;
