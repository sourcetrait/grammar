pub(crate) mod check;
pub(crate) mod error;
#[cfg(feature = "nushell")]
pub(crate) mod shape;
pub(crate) mod text {
    pub(crate) mod read;
    pub(crate) mod write;
}
pub(crate) mod ty;
#[cfg(all(test, feature = "nushell"))]
mod tests {
    mod check;
    mod shape;
    mod text;
}

pub(crate) use std::{fmt, str::FromStr};

pub(crate) use serde::{de, ser};

pub(crate) use nuval::NuVal;

#[cfg(feature = "nushell")]
pub(crate) mod nu {
    pub(crate) use nu_protocol::{SyntaxShape, Type};
}

pub(crate) use sourcetrait_common::cereal;

pub use crate::{
    error::{NuTypeError, NuTypeResult},
    ty::{NuField, NuType, NuTypeFile},
};
