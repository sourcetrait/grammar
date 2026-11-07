//! The engine's answers to its requests: execute, rerun, call, and control.
use crate::*;

/// What every request shares: the lanes, the calls in flight, the equips' locks.
pub(crate) struct EngineCore {
    pub(crate) executor: Executor,
    pub(crate) stateful: tk::AsyncMutex<Option<StatefulEngine>>,
    /// The one jobs table every eval's clone and the stateful engine share.
    pub(crate) env_jobs: Arc<std::sync::Mutex<nu::Jobs>>,
    pub(crate) in_flight: InFlight,
    pub(crate) hung: HungRegistry,
    /// The await jobs running, under their calls' nonces.
    pub(crate) jobs: AwaitJobs,
    pub(crate) rig_locks: RigLocks,
    /// The contract's, the lint's, and the validator's engine.
    pub(crate) lint_engine: LintEngine,
    /// The way an eval's `grmr` asks reach the engine's controller.
    pub(crate) host: HostLink,
    /// The instance's nom, every eval's `$env.EQUIP_MCP_NOM`.
    pub(crate) nom: String,
}

impl EngineCore {
    /// The core under `nom`: the TLS provider, the stateless base, the locks, the host.
    pub(crate) async fn start(
        host: HostLink,
        nom: String,
    ) -> io::Result<Self> {
        nu::CRYPTO_PROVIDER.default();
        let env_jobs = Arc::new(std::sync::Mutex::new(nu::Jobs::default()));
        let rig_locks = RigLocks::new();
        rig_locks.hydrate_from_disk().await?;
        Ok(Self {
            executor: Executor::new(env_jobs.clone(), READY_POOL_TARGET, nom.clone()),
            stateful: tk::AsyncMutex::new(None),
            env_jobs,
            in_flight: Arc::new(tk::AsyncMutex::new(HashMap::new())),
            hung: Arc::new(std::sync::Mutex::new(HashMap::new())),
            jobs: Arc::new(std::sync::Mutex::new(HashMap::new())),
            rig_locks,
            lint_engine: LintEngine::new(),
            host,
            nom,
        })
    }
}

/// Refuse a request for a host whose engine this process doesn't serve.
pub(crate) fn local(host: Host) -> EngineResult<()> {
    match host {
        Host::Local => Ok(()),
        Host::Remote(nom) => Err(EngineError::HostUnserved {
            host: format!("remote/{}", nom.into_pair().as_str()),
        }),
    }
}

/// A tool's timeout, its NUON text taken as the value it names, where it has one.
pub fn nuon_timeout(text: Option<&str>) -> EngineResult<Option<NuVal>> {
    let Some(text) = text else {
        return Ok(None);
    };
    NuVal::from_nuon(text).map(Some).map_err(|e| EngineError::TimeoutInvalid {
        reason: format!("`{text}` is not a Nu duration: {e}"),
    })
}

/// An await call's timeouts, its NUON text taken as the value it names, if given.
pub fn nuon_timeouts(text: Option<&str>) -> EngineResult<Option<NuVal>> {
    let Some(text) = text else {
        return Ok(None);
    };
    NuVal::from_nuon(text).map(Some).map_err(|e| EngineError::TimeoutInvalid {
        reason: format!("`timeouts` is not NUON: {e}"),
    })
}

/// An await call's heartbeats, its NUON text taken as the value it names, if given.
pub fn nuon_heartbeats(text: Option<&str>) -> EngineResult<Option<NuVal>> {
    let Some(text) = text else {
        return Ok(None);
    };
    NuVal::from_nuon(text).map(Some).map_err(|e| EngineError::HeartbeatsInvalid {
        reason: format!("`heartbeats` is not NUON: {e}"),
    })
}

/// A call's timeout as a positive Nu duration: its milliseconds and its value.
fn parse_timeout(timeout: Option<&NuVal>) -> EngineResult<(u64, nu::Value)> {
    let span = nu::Span::unknown();
    let Some(timeout) = timeout else {
        let nanos = DEFAULT_TIMEOUT_MS as i64 * 1_000_000;
        return Ok((DEFAULT_TIMEOUT_MS, nu::Value::duration(nanos, span)));
    };
    let invalid = |reason: String| EngineError::TimeoutInvalid { reason };
    let value = timeout
        .clone()
        .try_into_nu(span)
        .map_err(|e| invalid(format!("the timeout: {e}")))?;
    match value {
        nu::Value::Duration { val, .. } if val > 0 => {
            Ok(((val as u64).div_ceil(1_000_000), nu::Value::duration(val, span)))
        }
        other => Err(invalid(format!(
            "`{}` is typed {}, not a positive duration such as 30sec",
            timeout.to_nuon(NuonStyle::Raw),
            other.get_type(),
        ))),
    }
}

/// An `execute` call ready to dispatch: its contract, its source, its timeout.
struct PreparedExecute {
    def: ExecuteDef,
    source: String,
    timeout_ms: u64,
}

/// Hold a call's definition, data, and timeout to the contract; build its source.
fn prepare_execute(
    engine: &ParseEngine,
    definition: &str,
    data: &NuVal,
    timeout: Option<&NuVal>,
    lint: bool,
) -> EngineResult<PreparedExecute> {
    let def = analyze_definition(engine, definition, lint)
        .map_err(|diagnostics| EngineError::DefinitionViolations { diagnostics })?;
    let data_literal = hold_data(data, &def.data_type)?;
    let (timeout_ms, timeout_literal) = hold_timeout(timeout, def.takes_timeout)?;
    let source = build_execute_source(definition, &data_literal, timeout_literal.as_deref());
    Ok(PreparedExecute {
        def,
        source,
        timeout_ms,
    })
}

/// A tool's data, its NUON text taken as the value it names, `null` for nothing.
pub fn nuon_data(
    text: &str,
    holder: &str,
) -> EngineResult<NuVal> {
    let invalid = |reason: String| EngineError::DataInvalid { reason };
    if text.trim().is_empty() {
        return Err(invalid(format!(
            "data is NUON text, `null` for a `data: nothing` {holder}"
        )));
    }
    NuVal::from_nuon(text).map_err(|e| invalid(format!("data is not NUON: {e}")))
}

/// A request's data held to its typedef, answered as its source's literal.
pub(crate) fn hold_data(
    data: &NuVal,
    typedef: &NuType,
) -> EngineResult<String> {
    let invalid = |reason: String| EngineError::DataInvalid { reason };
    let value = data
        .clone()
        .try_into_nu(nu::Span::unknown())
        .map_err(|e| invalid(format!("data: {e}")))?;
    check_value(&value, typedef, "data").map_err(invalid)?;
    render_nuon(&value).map_err(invalid)
}

/// A request's timeout in milliseconds, with its literal if the signature takes it.
fn hold_timeout(
    timeout: Option<&NuVal>,
    takes_timeout: bool,
) -> EngineResult<(u64, Option<String>)> {
    let (timeout_ms, value) = parse_timeout(timeout)?;
    if !takes_timeout {
        return Ok((timeout_ms, None));
    }
    let literal = render_nuon(&value).map_err(|reason| EngineError::TimeoutInvalid { reason })?;
    Ok((timeout_ms, Some(literal)))
}

/// Dispatch a prepared call on its lane and hold its result to the contract.
async fn run_prepared(
    core: &EngineCore,
    nonce: datum::Nonce,
    prepared: PreparedExecute,
    definition: &str,
    tool: &'static str,
) -> EngineResult<NuVal> {
    let PreparedExecute {
        def,
        source,
        timeout_ms,
    } = prepared;
    let pair = nonce.into_pair();
    let call = Dispatch {
        nonce: &pair,
        source,
        tool,
        definition: Some(definition),
        timeout_ms: Some(timeout_ms),
    };
    let value = match def.lane {
        Lane::Stateless => dispatch_pooled(core, cache_dir(CacheKind::Execute, &pair), call).await?,
        Lane::Stateful => dispatch_stateful(core, call).await?,
    };
    check_value(&value, &def.result_type, "result")
        .map_err(|reason| EngineError::ResultInvalid { reason })?;
    NuVal::try_from_nu(value).map_err(|e| EngineError::ResultInvalid {
        reason: format!("the result: {e}"),
    })
}

/// An `execute` call's value, its definition held to the contract and linted.
pub(crate) async fn serve_execute(
    core: Arc<EngineCore>,
    req: ExecuteRequest,
) -> EngineResult<NuVal> {
    local(req.host)?;
    let engine = core.lint_engine.current();
    let definition = render_definition(&req.definition);
    let prepared = prepare_execute(&engine, &definition, &req.data, req.timeout.as_ref(), false)?;
    run_prepared(&core, req.nonce, prepared, &definition, "execute").await
}

/// A rerun's value: an earlier call's cached definition on fresh data.
pub(crate) async fn serve_rerun(
    core: Arc<EngineCore>,
    req: RerunRequest,
) -> EngineResult<NuVal> {
    local(req.host)?;
    if !cereal::is_base62(&req.source) {
        return Err(EngineError::RerunInvalidNonce {
            nonce: req.source.clone(),
            reason: "nonce must be base62".to_string(),
        });
    }
    let Ok(definition) = fs::read_to_string(definition_file(&req.source)) else {
        return Err(EngineError::RerunBodyMissing {
            nonce: req.source.clone(),
        });
    };
    let engine = core.lint_engine.current();
    let prepared =
        prepare_execute(&engine, &definition, &req.data, req.timeout.as_ref(), false)?;
    run_prepared(&core, req.nonce, prepared, &definition, "rerun").await
}

/// A call's value: its data and its result each held to `main`'s contract.
pub(crate) async fn serve_call(
    core: Arc<EngineCore>,
    req: CallRequest,
) -> EngineResult<NuVal> {
    local(req.host)?;
    let (rig, module_path, name) = match Namepath(req.namepath.clone()).validate()? {
        NamepathRef::Function {
            rig,
            module_path,
            name,
        } => (rig, module_path, name),
        _ => {
            return Err(EngineError::NamepathInvalid {
                namepath: req.namepath.clone(),
                reason: "call requires a function namepath: rig:module/path:function".to_string(),
            });
        }
    };
    let Some(lock) = core.rig_locks.lookup(&rig).await else {
        return Err(EngineError::EquipNotRegistered { equip: rig });
    };
    let _guard = lock.read().await;
    require_rig(&rig)?;
    let index = load_index(&rig).map_err(|e| EngineError::Internal {
        phase: "call::load_index".to_string(),
        reason: e.to_string(),
    })?;
    let Some(function) = index_node(&index, &module_path)
        .and_then(|(fns, _)| fns.iter().find(|f| f.name == name).cloned())
    else {
        return Err(EngineError::FunctionNotDefined {
            rig,
            module_path,
            name,
        });
    };
    let data_literal = hold_data(&req.data, &function.data_type)?;
    let (timeout_ms, timeout_literal) = hold_timeout(req.timeout.as_ref(), function.takes_timeout)?;
    let source = build_call_source(
        &equip_root(),
        &rig,
        &module_path,
        &name,
        &data_literal,
        timeout_literal.as_deref(),
    )
    .map_err(|reason| EngineError::Internal {
        phase: "call::source".to_string(),
        reason,
    })?;
    let pair = req.nonce.into_pair();
    let call = Dispatch {
        nonce: &pair,
        source,
        tool: "call",
        definition: None,
        timeout_ms: Some(timeout_ms),
    };
    let value = dispatch_pooled(&core, cache_dir(CacheKind::Calls, &pair), call).await?;
    check_value(&value, &function.result_type, "result")
        .map_err(|reason| EngineError::ResultInvalid { reason })?;
    NuVal::try_from_nu(value).map_err(|e| EngineError::ResultInvalid {
        reason: format!("the result: {e}"),
    })
}

/// A bed call's value: its command held to the rules, then run as a call.
pub(crate) async fn serve_bed(
    core: Arc<EngineCore>,
    req: BedRequest,
) -> EngineResult<NuVal> {
    local(req.host)?;
    let engine = core.lint_engine.current();
    let source = analyze_bed(&engine, &req.command, &req.subcommand)
        .map_err(|diagnostics| EngineError::BedViolations { diagnostics })?;
    let pair = req.nonce.into_pair();
    let call = Dispatch {
        nonce: &pair,
        source,
        tool: "bed",
        definition: None,
        timeout_ms: None,
    };
    let value = dispatch_pooled(&core, cache_dir(CacheKind::Calls, &pair), call).await?;
    NuVal::try_from_nu(value).map_err(|e| EngineError::ResultInvalid {
        reason: format!("the result: {e}"),
    })
}

/// A command's answer.
pub(crate) async fn serve_control(
    core: Arc<EngineCore>,
    req: ControlRequest,
) -> EngineResult<ControlAnswer> {
    local(req.host)?;
    match req.command {
        EngineCommand::AwaitList => Ok(ControlAnswer::Value(await_list(&core))),
        EngineCommand::AwaitStatus { nonce } => await_status(&core, &nonce).await.map(ControlAnswer::Value),
        EngineCommand::AwaitStop { nonce } => {
            await_stop(&core, &nonce).await?;
            Ok(ControlAnswer::Done)
        }
        EngineCommand::Health => Ok(ControlAnswer::Health(health(&core))),
        EngineCommand::Teardown => {
            teardown_all_in_flight(&core).await;
            Ok(ControlAnswer::Done)
        }
        EngineCommand::EquipNew { source } => equip_new(&core, &source).await,
        EngineCommand::EquipInstall { source } => equip_install(&core, &source).await,
        EngineCommand::EquipCheck { source } => equip_check(&core, &source).await,
        EngineCommand::EquipUninstall { source } => equip_uninstall(&core, &source).await,
        EngineCommand::EquipScaffold { namepaths } => equip_scaffold(&core, &namepaths).await,
        EngineCommand::EquipCommit { namepath } => equip_commit(&core, &namepath).await,
        EngineCommand::Signatures { patterns } => {
            Ok(ControlAnswer::Signatures(signatures(&core, &patterns).await))
        }
        EngineCommand::Help { namepath } => help(&core, &namepath).await,
    }
}

/// The engine's health: its hung evals, its permits, and its jobs.
fn health(core: &EngineCore) -> EngineHealth {
    let hung = hung_snapshot(&core.hung);
    let cap = eval_concurrency_cap();
    let held = cap.saturating_sub(core.executor.semaphore().available_permits());
    let jobs = core
        .env_jobs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .count();
    EngineHealth {
        hung,
        permits_held: held as u64,
        permits_cap: cap as u64,
        jobs: jobs as u64,
    }
}

/// Register an equip at its source and seed its rig.
async fn equip_new(
    core: &EngineCore,
    source: &str,
) -> EngineResult<ControlAnswer> {
    let source_dir = equip_source_dir(source);
    let equip = equip_namepath_of(&source_dir)?;
    let Ok(lock) = core.rig_locks.register(&equip).await else {
        return Err(EngineError::EquipAlreadyRegistered { equip });
    };
    let _guard = lock.write().await;
    match establish_equip(&equip, &source_dir) {
        Ok(created) => Ok(ControlAnswer::Created(created)),
        Err(e) => {
            core.rig_locks.unregister(&equip).await;
            Err(e)
        }
    }
}

/// Validate an equip's whole source and install it.
async fn equip_install(
    core: &EngineCore,
    source: &str,
) -> EngineResult<ControlAnswer> {
    let source_dir = equip_source_dir(source);
    let equip = equip_namepath_of(&source_dir)?;
    let Ok(lock) = core.rig_locks.register(&equip).await else {
        return Err(EngineError::EquipAlreadyRegistered { equip });
    };
    let _guard = lock.write().await;
    let engine = core.lint_engine.current();
    match install_equip(&equip, &source_dir, &engine) {
        Ok(result) => Ok(ControlAnswer::Installed { equip, result }),
        Err(e) => {
            core.rig_locks.unregister(&equip).await;
            Err(e)
        }
    }
}

/// Validate an installed equip's source, writing nothing.
async fn equip_check(
    core: &EngineCore,
    source: &str,
) -> EngineResult<ControlAnswer> {
    let source_dir = equip_source_dir(source);
    let equip = equip_namepath_of(&source_dir)?;
    let Some(lock) = core.rig_locks.lookup(&equip).await else {
        return Err(EngineError::EquipNotRegistered { equip });
    };
    let _guard = lock.read().await;
    check_source_dir(&equip, &source_dir)?;
    let engine = core.lint_engine.current();
    let result = check_equip(&equip, &engine)?;
    let (errors, warnings) = Diagnostic::bucket(result.diagnostics);
    Ok(ControlAnswer::Checked(CheckSummary {
        ok: errors.is_empty(),
        errors,
        warnings,
    }))
}

/// Remove an installed equip; its source stays.
async fn equip_uninstall(
    core: &EngineCore,
    source: &str,
) -> EngineResult<ControlAnswer> {
    let source_dir = equip_source_dir(source);
    let equip = equip_namepath_of(&source_dir)?;
    let Some(lock) = core.rig_locks.lookup(&equip).await else {
        return Ok(ControlAnswer::Uninstalled { equip: None });
    };
    let guard = lock.write().await;
    check_source_dir(&equip, &source_dir)?;
    uninstall_equip(&equip)?;
    drop(guard);
    core.rig_locks.unregister(&equip).await;
    Ok(ControlAnswer::Uninstalled { equip: Some(equip) })
}

/// Scaffold modules and calls into installed equips' rigs, the batch whole.
async fn equip_scaffold(
    core: &EngineCore,
    namepaths: &[String],
) -> EngineResult<ControlAnswer> {
    let mut targets: Vec<(String, String, Option<String>)> = Vec::new();
    for np in namepaths {
        match Namepath(np.clone()).validate()? {
            NamepathRef::Module {
                rig,
                module_path,
            } => targets.push((rig, module_path, None)),
            NamepathRef::Function {
                rig,
                module_path,
                name,
            } => targets.push((rig, module_path, Some(name))),
            NamepathRef::Rig { .. } => {
                return Err(EngineError::NamepathInvalid {
                    namepath: np.clone(),
                    reason: "scaffold needs a module or function namepath; use \
                             `interface equip new` to create an equip"
                        .to_string(),
                });
            }
        }
    }

    let mut libs: Vec<String> = targets.iter().map(|(l, _, _)| l.clone()).collect();
    libs.sort();
    libs.dedup();
    let mut locks = Vec::new();
    for lib in &libs {
        match core.rig_locks.lookup(lib).await {
            Some(l) => locks.push(l),
            None => {
                return Err(EngineError::EquipNotRegistered {
                    equip: lib.clone(),
                });
            }
        }
    }
    let mut guards = Vec::new();
    for l in &locks {
        guards.push(l.write().await);
    }

    for (rig, module_path, name) in &targets {
        if scaffold_leaf_exists(rig, module_path, name.as_deref())? {
            return Err(EngineError::RigInvalidName {
                name: module_path.clone(),
                reason: format!(
                    "`{}` already exists; edit it instead of scaffolding over it",
                    name.clone().unwrap_or_else(|| module_path.clone()),
                ),
            });
        }
    }

    let mut created = Vec::new();
    for (rig, module_path, name) in &targets {
        created.append(&mut scaffold_leaf(rig, module_path, name.as_deref())?);
    }
    Ok(ControlAnswer::Created(created))
}

/// Promote an equip's registered source into the namespace.
async fn equip_commit(
    core: &EngineCore,
    namepath: &str,
) -> EngineResult<ControlAnswer> {
    let equip = resolve_builtin(namepath);
    let Some(lock) = core.rig_locks.lookup(&equip).await else {
        return Err(EngineError::EquipNotRegistered {
            equip: namepath.to_string(),
        });
    };
    let _guard = lock.write().await;
    let engine = core.lint_engine.current();
    Ok(ControlAnswer::Committed(commit_equip(&equip, &engine)?))
}

/// The signature block within a set of patterns, each parsed or dropped.
async fn signatures(
    core: &EngineCore,
    patterns: &[String],
) -> String {
    let parsed: Vec<NamepathStr> = patterns
        .iter()
        .filter_map(|pattern| NamepathStr::parse(pattern).ok())
        .collect();
    render_signatures_within(&core.rig_locks, &parsed).await
}

/// The documentation of a rig, a module, or a call, by its exact namepath.
async fn help(
    core: &EngineCore,
    namepath: &str,
) -> EngineResult<ControlAnswer> {
    let (rig, module_path, name) = match Namepath(namepath.to_string()).validate()? {
        NamepathRef::Rig { rig } => (rig, String::new(), None),
        NamepathRef::Module {
            rig,
            module_path,
        } => (rig, module_path, None),
        NamepathRef::Function {
            rig,
            module_path,
            name,
        } => (rig, module_path, Some(name)),
    };
    let Some(lock) = core.rig_locks.lookup(&rig).await else {
        return Err(EngineError::EquipNotRegistered { equip: rig });
    };
    let _guard = lock.read().await;
    Ok(ControlAnswer::Help(inspect_impl(&rig, &module_path, name.as_deref())?))
}
