//! The interface's engine commands, run on a host's engine within that host's view.
use crate::*;

/// The name `interface info` reports for every host.
const NAME: &str = "grammar";

/// What a host lends an interface engine command: its engine, identity, and view.
pub trait Desk: Sync {
    /// The host's engine, or why it has none.
    fn engine(&self) -> EngineResult<Arc<EngineHandle>>;

    /// The host's identity, homes, and plugins.
    fn info(&self) -> HostInfo;

    /// The purview ids this host's session has in view on the engine.
    fn view(&self) -> &CurrentPurview;

    /// `interface help interface`'s block, the host's denied commands left out.
    fn interface_signatures(&self) -> String;
}

/// The current purview: which ids a session has in view on one engine.
pub struct CurrentPurview {
    ids: std::sync::Mutex<Vec<String>>,
}

impl Default for CurrentPurview {
    fn default() -> Self {
        Self::new()
    }
}

impl CurrentPurview {
    /// A fresh view, `default` alone.
    pub fn new() -> Self {
        Self {
            ids: std::sync::Mutex::new(vec![PURVIEW_DEFAULT.to_string()]),
        }
    }

    /// The view's lock, a poisoned one taken as it stands.
    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<String>> {
        self.ids.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The ids in view.
    pub fn ids(&self) -> Vec<String> {
        self.lock().clone()
    }

    /// Add ids to the current view; additive and order-preserving.
    pub fn extend(
        &self,
        add: &[String],
    ) -> Vec<String> {
        let mut ids = self.lock();
        for id in add {
            if !ids.contains(id) {
                ids.push(id.clone());
            }
        }
        ids.clone()
    }

    /// Replace what is in view; an empty list means `default`.
    pub fn set(
        &self,
        want: &[String],
    ) -> Vec<String> {
        let mut ids = self.lock();
        *ids = if want.is_empty() {
            vec![PURVIEW_DEFAULT.to_string()]
        } else {
            want.to_vec()
        };
        ids.clone()
    }

    /// Drop ids the rows no longer hold.
    pub fn retain_known(
        &self,
        rows: &[PurviewRow],
    ) {
        let mut ids = self.lock();
        ids.retain(|id| id == PURVIEW_ALL || rows.iter().any(|row| &row.id == id));
        if ids.is_empty() {
            *ids = vec![PURVIEW_DEFAULT.to_string()];
        }
    }
}

/// What a call's error reports, as rows.
type Rows = Vec<Diagnostic>;

/// The interface contract's parser: the interface commands, nothing else.
#[derive(Clone)]
pub struct InterfaceParser(Arc<nu::EngineState>);

impl InterfaceParser {
    /// A parser leaving out the commands `denied` names.
    pub fn new(denied: impl Fn(&str) -> bool) -> Self {
        Self(Arc::new(interface_engine(denied)))
    }
}

/// An interface call's subcommand, parsed here and run on the desk's engine.
///
/// Only an engine command runs: the Channel's, the remote links', the chat's, and
/// the remote engines' commands are the MCP's own, whatever host a call names.
pub async fn run_interface(
    desk: &impl Desk,
    parser: &InterfaceParser,
    subcommand: &str,
) -> Result<NuVal, Rows> {
    match parse_command(&parser.0, subcommand)? {
        InterfaceCommand::Engine(command) => run_engine_command(desk, command).await,
        _ => Err(vec![Diagnostic::error(
            KIND_INTERFACE,
            None,
            format!("`{}` runs on the MCP, not on a remote engine", subcommand.trim()),
        )]),
    }
}

/// `interface info`'s result.
#[derive(Debug, ser::Serialize)]
struct InfoResult {
    name: String,
    version: String,
    nu_version: String,
    /// The `--id` the engine's host was configured with.
    id: String,
    namespace: String,
    /// The engine's host process's nom, stable for its life: $env.EQUIP_MCP_NOM.
    nom: String,
    /// The engine's working directory, exported as $env.EQUIP_WORK_DIR.
    work_dir: String,
    /// The engine's bulk scratch directory, exported as $env.EQUIP_TMP.
    tmp: String,
    /// An agent's transient scratch, `ai`, and the engine's own, `grammar`.
    transient: TransientPaths,
    plugins: Vec<PluginInfo>,
    /// The callable surface within the reported purview, as one block.
    signatures: String,
    /// The purview `signatures` was rendered for.
    purview: Vec<PurviewView>,
}

/// `interface help`'s result.
#[derive(Debug, ser::Serialize)]
struct InspectResult {
    /// An exact namepath's documentation, or a pattern's `signatures` block.
    doc: InspectDoc,
}

/// `interface purview list`'s result.
#[derive(Debug, ser::Serialize)]
struct PurviewsResult {
    /// Every configured purview, as persisted.
    purviews: Vec<PurviewView>,
    /// The purview ids in view right now - the keys alone.
    current: Vec<String>,
}

/// `interface purview set`'s result.
#[derive(Debug, ser::Serialize)]
struct PurviewSetResult {
    /// `info`'s `signatures` for what this call brought into view, or null.
    revealed: Option<String>,
}

/// `interface purview extend`'s result: what a change to the current view did.
#[derive(Debug, ser::Serialize)]
struct PurviewDeltaResult {
    /// `info`'s `signatures` for what this call brought into view, or null.
    revealed: Option<String>,
    /// The purview ids in view now - the keys alone.
    current: Vec<String>,
}

/// `interface purview configure`'s result, the state after the write.
#[derive(Debug, ser::Serialize)]
struct PurviewConfigureResult {
    /// Everything this purview reveals; null when the call deleted it.
    signatures: Option<String>,
}

/// `interface equip new`'s summary: the paths scaffolded for the fresh equip's rig.
#[derive(Debug, ser::Serialize)]
struct NewSummary {
    created: Vec<String>,
}

/// The per-action summary, `install`'s its commit's; `uninstall` returns none.
#[derive(Debug, ser::Serialize)]
#[serde(untagged)]
enum EquipSummary {
    New(NewSummary),
    Install(CommitResult),
    Check(CheckSummary),
}

/// Each `interface equip` action's result on a source.
#[derive(Debug, ser::Serialize)]
struct EquipResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<EquipSummary>,
}

/// `interface equip scaffold`'s result: every path the batch made.
#[derive(Debug, ser::Serialize)]
struct ScaffoldResult {
    created: Vec<String>,
}

/// An engine command's value on the desk's engine, or the rows its refusal reports.
pub async fn run_engine_command(
    desk: &impl Desk,
    command: EngineInterfaceCommand,
) -> Result<NuVal, Rows> {
    use EngineInterfaceCommand as E;
    let engine = desk.engine().map_err(EngineError::rows)?;
    let engine = &*engine;
    match command {
        E::Info => info(desk, engine).await,
        E::Help(p) => help(desk, engine, p).await,
        E::ProcessList => process_list(engine).await,
        E::ProcessKill(p) => {
            control(engine, EngineCommand::Kill { nonce: p.nonce }).await?;
            Ok(NuVal::Nothing)
        }
        E::PurviewList => {
            let purviews = flat(engine.request(PurviewListRequest).await)?;
            value(&PurviewsResult {
                purviews,
                current: desk.view().ids(),
            })
        }
        E::PurviewSet(p) => purview_set(desk, engine, p).await,
        E::PurviewExtend(p) => purview_extend(desk, engine, p).await,
        E::PurviewConfigure(p) => purview_configure(desk, engine, p).await,
        E::EquipNew(p) => match control(engine, EngineCommand::EquipNew { source: p.source }).await? {
            ControlAnswer::Created(created) => value(&EquipResult {
                summary: Some(EquipSummary::New(NewSummary { created })),
            }),
            _ => Err(unexpected("equip_new")),
        },
        E::EquipInstall(p) => match control(engine, EngineCommand::EquipInstall { source: p.source }).await? {
            ControlAnswer::Installed { equip, result } => {
                purview_rig(desk, engine, equip, true).await;
                value(&EquipResult {
                    summary: Some(EquipSummary::Install(result)),
                })
            }
            _ => Err(unexpected("equip_install")),
        },
        E::EquipCheck(p) => match control(engine, EngineCommand::EquipCheck { source: p.source }).await? {
            ControlAnswer::Checked(summary) => value(&EquipResult {
                summary: Some(EquipSummary::Check(summary)),
            }),
            _ => Err(unexpected("equip_check")),
        },
        E::EquipUninstall(p) => match control(engine, EngineCommand::EquipUninstall { source: p.source }).await? {
            ControlAnswer::Uninstalled { equip } => {
                if let Some(equip) = equip {
                    purview_rig(desk, engine, equip, false).await;
                }
                value(&EquipResult { summary: None })
            }
            _ => Err(unexpected("equip_uninstall")),
        },
        E::EquipScaffold(p) => {
            match control(engine, EngineCommand::EquipScaffold { namepaths: p.namepaths }).await? {
                ControlAnswer::Created(created) => value(&ScaffoldResult { created }),
                _ => Err(unexpected("equip_scaffold")),
            }
        }
        E::EquipCommit(p) => match control(engine, EngineCommand::EquipCommit { namepath: p.namepath }).await? {
            ControlAnswer::Committed(result) => value(&result),
            _ => Err(unexpected("equip_commit")),
        },
    }
}

/// A control command's answer, or the rows its failure reports.
async fn control(
    engine: &EngineHandle,
    command: EngineCommand,
) -> Result<ControlAnswer, Rows> {
    engine.control(command).await.map_err(EngineError::rows)
}

/// A purview request's answer, the engine's failure and the purview's as rows.
fn flat<T>(answer: EngineResult<Result<T, RuntimeError>>) -> Result<T, Rows> {
    answer.map_err(EngineError::rows)?.map_err(RuntimeError::rows)
}

/// What a set of ids puts in view, as the engine resolves it.
async fn resolve(
    engine: &EngineHandle,
    ids: Vec<String>,
) -> Result<PurviewResolution, Rows> {
    flat(engine.request(PurviewResolveRequest { ids }).await)
}

/// The signature block within pattern strings, as the engine renders it.
async fn signatures(
    engine: &EngineHandle,
    patterns: Vec<String>,
) -> Result<String, Rows> {
    match control(engine, EngineCommand::Signatures { patterns }).await? {
        ControlAnswer::Signatures(block) => Ok(block),
        _ => Err(unexpected("signatures")),
    }
}

/// The signatures of what `after` puts in view that `before` didn't, or null.
async fn revealed(
    engine: &EngineHandle,
    before: &PurviewResolution,
    after: &PurviewResolution,
) -> Result<Option<String>, Rows> {
    let added: Vec<String> = after
        .patterns
        .iter()
        .filter(|p| !before.patterns.contains(p))
        .cloned()
        .collect();
    if added.is_empty() {
        return Ok(None);
    }
    signatures(engine, added).await.map(Some)
}

/// `interface info`: the host's versions, plugins, and signatures within its view.
async fn info(
    desk: &impl Desk,
    engine: &EngineHandle,
) -> Result<NuVal, Rows> {
    let view = resolve(engine, desk.view().ids()).await?;
    let signatures = signatures(engine, view.patterns).await?;
    let host = desk.info();
    value(&InfoResult {
        name: NAME.to_string(),
        version: host.version,
        nu_version: host.nu_version,
        id: host.id,
        namespace: host.namespace,
        nom: host.nom,
        work_dir: host.work_dir,
        tmp: host.tmp,
        transient: host.transient,
        plugins: host.plugins,
        signatures,
        purview: view.views,
    })
}

/// `interface help`: a rig's, a module's, or a call's documentation, or a block.
async fn help(
    desk: &impl Desk,
    engine: &EngineHandle,
    p: InspectParams,
) -> Result<NuVal, Rows> {
    let words = match p.namepath.as_str() {
        INTERFACE => Some(desk.interface_signatures()),
        GRMR => Some(grmr_signatures()),
        _ => None,
    };
    if let Some(signatures) = words {
        return value(&InspectResult {
            doc: InspectDoc::Signatures(SignaturesDoc { signatures }),
        });
    }
    let pattern = match NamepathStr::parse(&p.namepath).map_err(EngineError::rows)? {
        NamepathStr::Pattern(pattern) => pattern,
        NamepathStr::Namepath(_) => {
            return match control(engine, EngineCommand::Help { namepath: p.namepath }).await? {
                ControlAnswer::Help(doc) => value(&InspectResult { doc }),
                _ => Err(unexpected("help")),
            };
        }
    };
    let patterns = match pattern {
        NamepathPattern::Current => resolve(engine, desk.view().ids()).await?.patterns,
        _ => vec![p.namepath],
    };
    let signatures = signatures(engine, patterns).await?;
    value(&InspectResult {
        doc: InspectDoc::Signatures(SignaturesDoc { signatures }),
    })
}

/// `interface process list`: every call in flight on the engine.
async fn process_list(engine: &EngineHandle) -> Result<NuVal, Rows> {
    let rows = match control(engine, EngineCommand::Processes).await? {
        ControlAnswer::Processes(rows) => rows,
        _ => return Err(unexpected("processes")),
    };
    let processes = rows
        .into_iter()
        .map(|row| {
            let mut fields = vec![
                ("nonce".to_string(), NuVal::String(row.nonce)),
                ("tool".to_string(), NuVal::String(row.tool)),
                ("started_at".to_string(), NuVal::Int(row.started_at as i64)),
                ("args".to_string(), row.args),
            ];
            if let Some(source_nonce) = row.source_nonce {
                fields.push(("source_nonce".to_string(), NuVal::String(source_nonce)));
            }
            if let Some(path) = row.path {
                fields.push(("path".to_string(), NuVal::String(path)));
            }
            NuVal::Record(fields)
        })
        .collect();
    Ok(NuVal::Record(vec![("processes".to_string(), NuVal::List(processes))]))
}

/// `interface purview set`: the view set to these purviews.
async fn purview_set(
    desk: &impl Desk,
    engine: &EngineHandle,
    p: PurviewParams,
) -> Result<NuVal, Rows> {
    let want: Vec<String> = p
        .purviews
        .iter()
        .map(|id| purview_ref(id).unwrap_or(id).to_string())
        .collect();
    let wanted = resolve(engine, want.clone()).await?;
    if let Some(id) = wanted.unknown.first() {
        return Err(unknown_purview(id));
    }
    let current = desk.view();
    let before = resolve(engine, current.ids()).await?;
    let ids = current.set(&want);
    let after = if ids == want { wanted } else { resolve(engine, ids).await? };
    let revealed = revealed(engine, &before, &after).await?;
    value(&PurviewSetResult { revealed })
}

/// `interface purview extend`: more purviews brought into the view.
async fn purview_extend(
    desk: &impl Desk,
    engine: &EngineHandle,
    p: PurviewExtendParams,
) -> Result<NuVal, Rows> {
    let asked = resolve(engine, p.purviews.clone()).await?;
    if let Some(id) = asked.unknown.first() {
        return Err(unknown_purview(id));
    }
    let current = desk.view();
    let before = resolve(engine, current.ids()).await?;
    let ids = current.extend(&p.purviews);
    let after = resolve(engine, ids.clone()).await?;
    let revealed = revealed(engine, &before, &after).await?;
    value(&PurviewDeltaResult { revealed, current: ids })
}

/// `interface purview configure`: a purview's patterns, an empty list deleting it.
async fn purview_configure(
    desk: &impl Desk,
    engine: &EngineHandle,
    p: PurviewConfigureParams,
) -> Result<NuVal, Rows> {
    let configure = PurviewConfigureRequest {
        id: p.purview.clone(),
        patterns: p.namepaths,
    };
    let rows = flat(engine.request(configure).await)?;
    desk.view().retain_known(&rows);
    let signatures = if rows.iter().any(|row| row.id == p.purview) {
        let view = resolve(engine, vec![p.purview]).await?;
        Some(signatures(engine, view.patterns).await?)
    } else {
        None
    };
    value(&PurviewConfigureResult { signatures })
}

/// A rig installed or removed: the engine's purviews adjusted, the view kept.
async fn purview_rig(
    desk: &impl Desk,
    engine: &EngineHandle,
    rig: String,
    installed: bool,
) {
    let current = desk.view();
    let adjust = PurviewRigRequest {
        rig: rig.clone(),
        installed,
        current: current.ids(),
    };
    match flat(engine.request(adjust).await) {
        Ok(rows) if !installed => current.retain_known(&rows),
        Ok(_) => {}
        Err(rows) => {
            let reasons: Vec<&str> = rows.iter().map(|row| row.message.as_str()).collect();
            eprintln!("grammar: purview not updated for {rig}: {}", reasons.join("; "));
        }
    }
}

/// The refusal of a view naming an id no purview answers to.
fn unknown_purview(id: &str) -> Rows {
    RuntimeError::PurviewInvalidId {
        id: id.to_string(),
        reason: "no such purview; configure it first".to_string(),
    }
    .rows()
}

/// The rows of an answer of another command's kind.
fn unexpected(command: &str) -> Rows {
    RuntimeError::Internal {
        phase: format!("engine::{command}"),
        reason: "the engine answered another command".to_string(),
    }
    .rows()
}

/// A command's result as its value.
fn value<T: ser::Serialize>(result: &T) -> Result<NuVal, Rows> {
    nuval::to_nuval(result).map_err(|e| {
        RuntimeError::Internal {
            phase: "interface::result".to_string(),
            reason: e.to_string(),
        }
        .rows()
    })
}
