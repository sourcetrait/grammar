//! The namespace's purviews: their rows, views, and the engine's requests of them.
use crate::*;

/// One configured purview: a label bound to what it puts in view.
#[cereal::derived(Data)]
pub struct PurviewRow {
    pub id: String,
    pub namepath_patterns: Vec<String>,
}

/// One `[id, namepath_patterns]` pair as the tools report it.
#[cereal::derived(Data)]
pub struct PurviewView(pub String, pub Vec<String>);

/// The engine's requests of its namespace's purviews.
#[cereal::derived(Data)]
pub enum PurviewRequest {
    List(PurviewListRequest),
    Configure(PurviewConfigureRequest),
    Resolve(PurviewResolveRequest),
    Rig(PurviewRigRequest),
}

/// Each purview request's answer.
#[cereal::derived(Data)]
pub enum PurviewAnswer {
    Listed(Result<Vec<PurviewView>, RuntimeError>),
    Rows(Result<Vec<PurviewRow>, RuntimeError>),
    Resolved(Result<PurviewResolution, RuntimeError>),
}

/// Each configured purview, its patterns as stored.
#[cereal::derived(Data, Copy, Eq)]
pub struct PurviewListRequest;

/// An id's patterns set, an empty list deleting it; the rows after.
#[cereal::derived(Data, Eq)]
pub struct PurviewConfigureRequest {
    pub id: String,
    pub patterns: Vec<String>,
}

/// Ids resolved to what they put in view, each reference expanded.
#[cereal::derived(Data, Eq)]
pub struct PurviewResolveRequest {
    pub ids: Vec<String>,
}

/// A rig installed or uninstalled, the rows adjusted for it; the rows after.
#[cereal::derived(Data, Eq)]
pub struct PurviewRigRequest {
    pub rig: String,
    pub installed: bool,
    /// The session's view, which a rig installed joins.
    pub current: Vec<String>,
}

/// What a set of ids puts in view.
#[cereal::derived(Data)]
pub struct PurviewResolution {
    /// The patterns, in order and deduplicated, each reference expanded.
    pub patterns: Vec<String>,
    /// Each id beside the patterns it names itself.
    pub views: Vec<PurviewView>,
    /// The ids no purview answers to.
    pub unknown: Vec<String>,
}
