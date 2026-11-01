use crate::*;

/// Parameters for `interface help`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct InspectParams {
    /// A namepath, a pattern ending in `/` or `:`, `*`, `.`, `interface`, or `grimm`.
    pub namepath: String,
}

/// Parameters for `interface process kill`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct KillParams {
    /// The nonce of the in-flight call to cancel.
    pub nonce: String,
}

/// Parameters for `interface purview set`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct PurviewParams {
    /// The purview ids to put in view, replacing the current set.
    pub purviews: Vec<String>,
}

/// Parameters for `interface purview extend`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct PurviewExtendParams {
    /// Purview ids to add to the current view; additive.
    pub purviews: Vec<String>,
}

/// Parameters for `interface purview configure`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct PurviewConfigureParams {
    /// The purview to set, created if absent; a path-like snake label.
    pub purview: String,
    /// Its namepath patterns; an empty list deletes the purview.
    pub namepaths: Vec<String>,
}

/// Parameters for `interface channel open`.
#[derive(Debug, ser::Serialize)]
pub struct ChannelOpenParams {
    /// `sync` or `async`: how the Channel's packets reach the agent.
    pub mode: ChannelMode,
}

/// Parameters for `interface channel config`, a partial update.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct ConfigChannelParams {
    /// Window for the soft threshold, in whole seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spam_warn_window_secs: Option<u64>,
    /// Sends within that window before one warning is issued for an origin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spam_warn_rate: Option<u32>,
    /// Window for the hard threshold, in whole seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spam_error_window_secs: Option<u64>,
    /// Sends within that window before the origin is refused and stopped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spam_error_rate: Option<u32>,
}

/// Parameters for a remote channel command that names one alias alone.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct RemoteAliasParams {
    /// The remote's alias.
    pub alias: String,
}

/// Parameters for `interface remote channel set`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct RemoteSetParams {
    /// The remote to make or change.
    pub alias: String,
    /// The `ip:port` a connect dials by default.
    pub address: Option<String>,
    /// The `ip` or `ip:port` a listen binds by default.
    pub listen: Option<String>,
    /// The peer's certificate, PEM.
    pub cert: Option<String>,
}

/// Parameters for `interface remote channel listen`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct RemoteListenParams {
    /// The remote whose peer to listen for.
    pub alias: String,
    /// The `ip` or `ip:port` to bind, in place of the entry's.
    pub on: Option<String>,
}

/// Parameters for `interface remote channel connect`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct RemoteConnectParams {
    /// The remote whose peer to dial.
    pub alias: String,
    /// The `ip:port` to dial, in place of the entry's.
    pub address: Option<String>,
}

/// Parameters for `interface remote channel close`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct RemoteChannelCloseParams {
    /// The alias of the open link to close.
    pub alias: String,
}

/// Parameters for `interface remote engine set`: each field given is written.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct RemoteEngineSetParams {
    /// The engine to make or change.
    pub alias: String,
    /// The host's name or address.
    pub host: Option<String>,
    /// The host's SSH port.
    pub port: Option<u16>,
    /// The account the session logs in as.
    pub user: Option<String>,
    /// The private key file the session authenticates with.
    pub key: Option<String>,
    /// The host's public keys as OpenSSH lines.
    pub host_keys: Option<Vec<String>>,
    /// The worker's absolute path on the host.
    pub command: Option<String>,
    /// The worker's id.
    pub id: Option<String>,
    /// The worker's namespace.
    pub namespace: Option<String>,
    /// The worker's absolute working directory on the host.
    pub workdir: Option<String>,
}

/// Parameters for `interface remote engine import`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct RemoteEngineImportParams {
    /// The engine whose entry the import makes or changes.
    pub alias: String,
    /// The ssh configuration's host alias to read; the engine's alias when absent.
    pub from: Option<String>,
    /// The ssh configuration file to read, in place of the user's.
    pub ssh_config: Option<String>,
}

/// Parameters for each `interface equip` action on a source.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct EquipParams {
    /// The equip's source: an `equip/<author>/<name>` directory.
    pub source: String,
}

/// Parameters for `interface equip scaffold`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct NewParams {
    /// Namepaths to scaffold into installed equips' rigs: a module or a function.
    pub namepaths: Vec<String>,
}

/// Parameters for `interface equip commit`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct CommitParams {
    /// The equip to commit: its `<author>/<name>`, or a builtin's name.
    pub namepath: String,
}
