//! An await job: prepare in the call, execute as the job, status, and stop.
use crate::*;

/// The tool a job runs under, as the process list shows it.
pub(crate) const AWAIT_TOOL: &str = "await";

/// Each definition's timeout where the call names none, a call's own default.
const DEFAULT_PHASE_TIMEOUT_NS: i64 = DEFAULT_TIMEOUT_MS as i64 * 1_000_000;

/// The gaps between beats where the call names none, in seconds.
const DEFAULT_HEARTBEATS_SECS: [u64; 7] = [30, 60, 300, 900, 1800, 2700, 3600];

/// The fewest gaps a call's heartbeats name.
const MIN_HEARTBEATS: usize = 2;

/// A job's trace under its directory, one event a line.
pub const TRACE_FILE: &str = "trace.nuonl";

/// The jobs running, each under its await call's nonce.
pub(crate) type AwaitJobs = Arc<std::sync::Mutex<HashMap<String, Arc<AwaitJob>>>>;

/// One definition's bound in nanoseconds, `0` leaving execute unbounded.
#[derive(Debug, Clone, Copy)]
struct PhaseTimeout(i64);

impl PhaseTimeout {
    /// The bound in milliseconds, none where it is unbounded.
    fn ms(self) -> Option<u64> {
        (self.0 > 0).then(|| (self.0 as u64).div_ceil(1_000_000))
    }

    /// The literal a definition's `timeout?` takes.
    fn literal(self) -> EngineResult<String> {
        render_nuon(&nu::Value::duration(self.0, nu::Span::unknown()))
            .map_err(|reason| EngineError::TimeoutInvalid { reason })
    }
}

/// Each definition's bound, as the call resolves them.
#[derive(Debug, Clone, Copy)]
struct SuiteTimeouts {
    prepare: PhaseTimeout,
    execute: PhaseTimeout,
    status: PhaseTimeout,
    stop: PhaseTimeout,
}

/// Why a job's stop ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StopReason {
    /// execute ran past its timeout.
    Timeout,
    /// `interface process kill` named the job.
    Kill,
    /// The host's teardown.
    Teardown,
}

impl StopReason {
    /// The reason as a Stopped event names it.
    fn as_str(self) -> &'static str {
        match self {
            Self::Timeout => "timeout",
            Self::Kill => "kill",
            Self::Teardown => "teardown",
        }
    }
}

/// A job the engine holds while it runs.
pub(crate) struct AwaitJob {
    nonce: String,
    dir: PathBuf,
    suite: PathBuf,
    def: SuiteDef,
    /// The job's record, `{id, started}`.
    job: NuVal,
    /// The PREPARED record's literal, the data execute, status, and stop take.
    literal: String,
    timeouts: SuiteTimeouts,
    job_id: nu::JobId,
    started_at: u64,
    /// execute's interrupt flag, its tracker, and its thread's end.
    cancel: Arc<AtomicBool>,
    tracker: nu::ThreadJob,
    finished: Arc<AtomicBool>,
    /// Taken once, by execute's end or by the first interruption.
    settled: AtomicBool,
    /// Set once the job is over, so no beat starts after it.
    done: AtomicBool,
    /// Wakes the beats when the job is over.
    over: tk::Notify,
    /// Wakes execute's watch once a stop has killed the job.
    stopped: tk::Notify,
    /// Holds the trace's writes one at a time.
    trace_lock: std::sync::Mutex<()>,
}

impl AwaitJob {
    /// Whether this caller is the first to settle the job.
    fn settle(&self) -> bool {
        !self.settled.swap(true, Ordering::SeqCst)
    }

    /// The job over: no beat starts after this.
    fn end(&self) {
        if !self.done.swap(true, Ordering::SeqCst) {
            self.over.notify_one();
        }
    }

    /// An event appended to the trace, one NUON line.
    fn trace(
        &self,
        model: &str,
        data: &NuVal,
    ) {
        let line = NuVal::Record(vec![
            ("model".to_string(), NuVal::String(model.to_string())),
            ("at".to_string(), NuVal::Date(now_date())),
            ("data".to_string(), data.clone()),
        ])
        .to_nuon(NuonStyle::Raw)
        .replace('\n', "\\n")
        .replace('\r', "\\r");
        let _guard = self.trace_lock.lock().unwrap_or_else(|e| e.into_inner());
        let written = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.dir.join(TRACE_FILE))
            .and_then(|mut file| writeln!(file, "{line}"));
        if let Err(e) = written {
            eprintln!("grammar: an await job's trace under {} not written: {e}", self.dir.display());
        }
    }
}

/// An await call: its suite held to the contract, prepare run, the job started.
pub(crate) async fn serve_await(
    core: Arc<EngineCore>,
    req: AwaitRequest,
) -> EngineResult<NuVal> {
    local(req.host)?;
    let engine = core.lint_engine.current();
    let def = analyze_suite(&engine, &req.module)
        .map_err(|diagnostics| EngineError::SuiteViolations { diagnostics })?;
    let timeouts = suite_timeouts(req.timeouts.as_ref())?;
    let gaps = heartbeat_gaps(req.heartbeats.as_ref())?;
    let mut args = vec![hold_data(&req.data, &def.data_type)?];
    if def.takes_timeouts {
        args.push(given_literal(req.timeouts.as_ref(), "timeouts")?);
    }
    if def.takes_heartbeats {
        args.push(given_literal(req.heartbeats.as_ref(), "heartbeats")?);
    }
    let pair = req.nonce.into_pair();
    let nonce = pair.as_str().to_string();
    let dir = cache_dir(CacheKind::Await, &pair);
    let suite = dir.join(SUITE_FILE);
    fs::create_dir_all(&dir)
        .and_then(|()| fs::write(&suite, &req.module))
        .map_err(|e| EngineError::Internal {
            phase: "await::suite".to_string(),
            reason: format!("{}: {e}", suite.display()),
        })?;
    let source = build_suite_source(&suite, PREPARE, &args).map_err(|reason| EngineError::Internal {
        phase: "await::source".to_string(),
        reason,
    })?;
    let request_args = NuVal::Record(vec![
        ("data".to_string(), req.data.clone()),
        ("timeouts".to_string(), req.timeouts.clone().unwrap_or(NuVal::Nothing)),
        ("heartbeats".to_string(), req.heartbeats.clone().unwrap_or(NuVal::Nothing)),
    ]);
    let value = eval_pooled(
        &core,
        dir.join(PREPARE),
        source,
        &nonce,
        AWAIT_TOOL,
        timeouts.prepare.ms(),
        Some((request_args.clone(), InFlightKind::Await)),
    )
    .await?;
    check_value(&value, &def.prepared_type, "result")
        .map_err(|reason| EngineError::ResultInvalid { reason })?;
    let prepared = NuVal::try_from_nu(value).map_err(|e| EngineError::ResultInvalid {
        reason: format!("the result: {e}"),
    })?;
    let launch = Launch {
        nonce,
        dir,
        suite,
        def,
        prepared,
        timeouts,
        args: request_args,
    };
    start(core, launch, gaps).await
}

/// What a job's start takes from its call.
struct Launch {
    nonce: String,
    dir: PathBuf,
    suite: PathBuf,
    def: SuiteDef,
    prepared: NuVal,
    timeouts: SuiteTimeouts,
    args: NuVal,
}

/// The job started: registered, execute on its thread, the beats scheduled.
async fn start(
    core: Arc<EngineCore>,
    launch: Launch,
    gaps: Vec<tk::TkDuration>,
) -> EngineResult<NuVal> {
    let Launch {
        nonce,
        dir,
        suite,
        def,
        prepared,
        timeouts,
        args,
    } = launch;
    let cancel = Arc::new(AtomicBool::new(false));
    let tracker = make_tracker(cancel.clone());
    let job_id = core
        .env_jobs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .add_job(nu::Job::Thread(tracker.clone()));
    let execute_dir = dir.join(EXECUTE_EXPORT);
    let built = (|| -> EngineResult<(NuVal, NuVal, String, String)> {
        let job = NuVal::Record(vec![
            ("id".to_string(), NuVal::Int(i64::try_from(job_id.get()).unwrap_or(i64::MAX))),
            ("started".to_string(), NuVal::Date(now_date())),
        ]);
        let record = NuVal::Record(vec![
            ("job".to_string(), job.clone()),
            ("prepared".to_string(), prepared),
        ]);
        let internal = |reason: String| EngineError::Internal {
            phase: "await::start".to_string(),
            reason,
        };
        let value = record
            .clone()
            .try_into_nu(nu::Span::unknown())
            .map_err(|e| internal(e.to_string()))?;
        let literal = render_nuon(&value).map_err(internal)?;
        let execute_args = phase_args(&literal, def.execute.takes_timeout, timeouts.execute)?;
        let source = build_suite_source(&suite, EXECUTE_EXPORT, &execute_args).map_err(internal)?;
        fs::create_dir_all(&execute_dir).map_err(|e| internal(format!("{}: {e}", execute_dir.display())))?;
        Ok((job, record, literal, source))
    })();
    let (job, record, literal, source) = match built {
        Ok(built) => built,
        Err(error) => {
            core.env_jobs.lock().unwrap_or_else(|e| e.into_inner()).remove_job(job_id);
            return Err(error);
        }
    };
    let finished = Arc::new(AtomicBool::new(false));
    let started_at = now_millis();
    let state = Arc::new(AwaitJob {
        nonce: nonce.clone(),
        dir,
        suite,
        def,
        job,
        literal,
        timeouts,
        job_id,
        started_at,
        cancel: cancel.clone(),
        tracker: tracker.clone(),
        finished: finished.clone(),
        settled: AtomicBool::new(false),
        done: AtomicBool::new(false),
        over: tk::Notify::new(),
        stopped: tk::Notify::new(),
        trace_lock: std::sync::Mutex::new(()),
    });
    core.jobs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(nonce.clone(), state.clone());
    register_in_flight(
        &core.in_flight,
        &nonce,
        InFlightEntry {
            tool: AWAIT_TOOL,
            started_at,
            args,
            kind: InFlightKind::Await,
            cancel: cancel.clone(),
            tracker: tracker.clone(),
            finished: finished.clone(),
        },
    )
    .await;
    let started = NuVal::Record(vec![
        ("nonce".to_string(), NuVal::String(state.nonce.clone())),
        ("job".to_string(), state.job.clone()),
    ]);
    emit(&core, &state, AWAIT_START_MODEL, started).await;
    core.executor.refresh_base_if_stale();
    let mut engine = core.executor.take_clone();
    engine.current_job.id = job_id;
    engine.current_job.background_thread_job = Some(tracker);
    let eval = eval_stateless(
        engine,
        cancel,
        execute_dir,
        source,
        nonce,
        None,
        finished,
        core.host.clone(),
    );
    tk::spawn(run_execute(core.clone(), state.clone(), eval));
    tk::spawn(run_beats(core, state, gaps));
    Ok(record)
}

/// execute's eval watched: its end sent as the Result, its timeout a stop.
async fn run_execute(
    core: Arc<EngineCore>,
    job: Arc<AwaitJob>,
    eval: impl Future<Output = EngineResult<nu::Value>> + Send,
) {
    let deadline = job.timeouts.execute.ms();
    let timed_out = async move {
        match deadline {
            Some(ms) => tk::sleep(tk::TkDuration::from_millis(ms)).await,
            None => std::future::pending::<()>().await,
        }
    };
    tokio::select! {
        result = eval => {
            job.end();
            if job.settle() {
                let value = result.and_then(|value| {
                    check_value(&value, &job.def.execute.result_type, "result")
                        .map_err(|reason| EngineError::ResultInvalid { reason })?;
                    NuVal::try_from_nu(value).map_err(|e| EngineError::ResultInvalid {
                        reason: format!("the result: {e}"),
                    })
                });
                let data = event_data(&job, Vec::new(), value);
                emit(&core, &job, AWAIT_RESULT_MODEL, data).await;
            }
        }
        () = timed_out => stop_job(&core, &job, StopReason::Timeout).await,
        () = job.stopped.notified() => {}
    }
    finish(&core, &job).await;
}

/// The beats: status run at each gap while the job runs, each value an event.
async fn run_beats(
    core: Arc<EngineCore>,
    job: Arc<AwaitJob>,
    gaps: Vec<tk::TkDuration>,
) {
    let mut beat: usize = 0;
    loop {
        let gap = gaps[beat.min(gaps.len().saturating_sub(1))];
        if job.done.load(Ordering::SeqCst) {
            return;
        }
        tokio::select! {
            () = tk::sleep(gap) => {}
            () = job.over.notified() => return,
        }
        if job.done.load(Ordering::SeqCst) {
            return;
        }
        beat += 1;
        let value = run_phase(&core, &job, STATUS, &job.def.status, job.timeouts.status).await;
        let beat_field = ("beat".to_string(), NuVal::Int(i64::try_from(beat).unwrap_or(i64::MAX)));
        let data = event_data(&job, vec![beat_field], value);
        emit(&core, &job, AWAIT_STATUS_MODEL, data).await;
    }
}

/// A job's stop, run once: stop's eval, the job killed, then its Stopped event.
pub(crate) async fn stop_job(
    core: &EngineCore,
    job: &AwaitJob,
    reason: StopReason,
) {
    if !job.settle() {
        return;
    }
    job.end();
    let value = run_phase(core, job, STOP, &job.def.stop, job.timeouts.stop).await;
    job.cancel.store(true, Ordering::SeqCst);
    tree_kill(&job.tracker.collect_pids());
    register_hung(
        &core.hung,
        HungWatch {
            nonce: job.nonce.clone(),
            tool: AWAIT_TOOL,
            lane: Lane::Stateless,
            started_at: job.started_at,
            cancelled_at: now_millis(),
            finished: job.finished.clone(),
        },
    );
    let reason_field = ("reason".to_string(), NuVal::String(reason.as_str().to_string()));
    let data = event_data(job, vec![reason_field], value);
    if reason == StopReason::Teardown {
        job.trace(AWAIT_STOPPED_MODEL, &data);
    } else {
        emit(core, job, AWAIT_STOPPED_MODEL, data).await;
    }
    job.stopped.notify_one();
}

/// Every running job's stop at once, each bounded by its own timeout.
pub(crate) async fn stop_jobs(
    core: &Arc<EngineCore>,
    reason: StopReason,
) {
    let jobs: Vec<Arc<AwaitJob>> = core
        .jobs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .values()
        .cloned()
        .collect();
    let mut stops = tk::JoinSet::new();
    for job in jobs {
        let core = core.clone();
        stops.spawn(async move { stop_job(&core, &job, reason).await });
    }
    while stops.join_next().await.is_some() {}
}

/// The running job under a call's nonce, if any.
pub(crate) fn running_job(
    core: &EngineCore,
    nonce: &str,
) -> Option<Arc<AwaitJob>> {
    core.jobs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(nonce)
        .cloned()
}

/// A job over: out of the engine's jobs, the jobs table, and the calls in flight.
async fn finish(
    core: &EngineCore,
    job: &AwaitJob,
) {
    core.jobs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&job.nonce);
    core.env_jobs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove_job(job.job_id);
    remove_in_flight(&core.in_flight, &job.nonce, &job.finished).await;
}

/// One of status and stop run on the PREPARED record, its value held to its type.
async fn run_phase(
    core: &EngineCore,
    job: &AwaitJob,
    export: &'static str,
    phase: &PhaseDef,
    timeout: PhaseTimeout,
) -> EngineResult<NuVal> {
    let args = phase_args(&job.literal, phase.takes_timeout, timeout)?;
    let source = build_suite_source(&job.suite, export, &args).map_err(|reason| EngineError::Internal {
        phase: format!("await::{export}"),
        reason,
    })?;
    let value = eval_pooled(core, job.dir.join(export), source, &job.nonce, AWAIT_TOOL, timeout.ms(), None)
        .await?;
    check_value(&value, &phase.result_type, "result")
        .map_err(|reason| EngineError::ResultInvalid { reason })?;
    NuVal::try_from_nu(value).map_err(|e| EngineError::ResultInvalid {
        reason: format!("the result: {e}"),
    })
}

/// A definition's arguments: the PREPARED literal, then its timeout if taken.
fn phase_args(
    literal: &str,
    takes_timeout: bool,
    timeout: PhaseTimeout,
) -> EngineResult<Vec<String>> {
    let mut args = vec![literal.to_string()];
    if takes_timeout {
        args.push(timeout.literal()?);
    }
    Ok(args)
}

/// A parameter prepare takes as the call gave it, `null` where it is absent.
fn given_literal(
    given: Option<&NuVal>,
    holder: &str,
) -> EngineResult<String> {
    let Some(value) = given else {
        return Ok("null".to_string());
    };
    let internal = |reason: String| EngineError::Internal {
        phase: format!("await::{holder}"),
        reason,
    };
    let value = value
        .clone()
        .try_into_nu(nu::Span::unknown())
        .map_err(|e| internal(e.to_string()))?;
    render_nuon(&value).map_err(internal)
}

/// An event's data: the nonce, its fields, then a value or its error's rows.
fn event_data(
    job: &AwaitJob,
    fields: Vec<(String, NuVal)>,
    value: EngineResult<NuVal>,
) -> NuVal {
    let mut data = vec![("nonce".to_string(), NuVal::String(job.nonce.clone()))];
    data.extend(fields);
    match value {
        Ok(result) => data.push(("result".to_string(), result)),
        Err(error) => data.push(("error".to_string(), rows_value(error.rows()))),
    }
    NuVal::Record(data)
}

/// An error's rows as a value, as a tool's envelope carries them.
fn rows_value(rows: Vec<Diagnostic>) -> NuVal {
    nuval::to_nuval(&rows).unwrap_or_else(|e| {
        NuVal::List(vec![NuVal::Record(vec![
            ("kind".to_string(), NuVal::String("internal".to_string())),
            ("message".to_string(), NuVal::String(format!("the error's rows: {e}"))),
            ("source".to_string(), NuVal::Nothing),
            ("detail".to_string(), NuVal::Nothing),
        ])])
    })
}

/// An event traced, then sent to the host's Channel; a refusal is logged.
async fn emit(
    core: &EngineCore,
    job: &AwaitJob,
    model: &'static str,
    data: NuVal,
) {
    job.trace(model, &data);
    let host = core.host.clone();
    let event = AwaitEvent {
        model: model.to_string(),
        data,
    };
    match tk::spawn_blocking(move || host.ask(HostAsk::Await(event))).await {
        Ok(Ok(HostAnswer::Sent(_))) => {}
        Ok(Ok(HostAnswer::Refused(reason)) | Err(reason)) => {
            eprintln!("grammar: an await job's {model} not sent: {reason}");
        }
        Ok(Ok(_)) => eprintln!("grammar: an await job's {model} met an answer that is not a send's"),
        Err(e) => eprintln!("grammar: an await job's {model} not sent: {e}"),
    }
}

/// The call's timeouts resolved: null or a positive duration each, 120sec if null.
fn suite_timeouts(given: Option<&NuVal>) -> EngineResult<SuiteTimeouts> {
    let invalid = |reason: String| EngineError::TimeoutInvalid { reason };
    let mut nanos = [DEFAULT_PHASE_TIMEOUT_NS; 4];
    if let Some(given) = given {
        let NuVal::Record(fields) = given else {
            return Err(invalid(format!(
                "`timeouts` is a record of `{}`, each null or a duration, not `{}`",
                SUITE_EXPORTS.join("`, `"),
                given.to_nuon(NuonStyle::Default),
            )));
        };
        if let Some((key, _)) = fields.iter().find(|(key, _)| !SUITE_EXPORTS.contains(&key.as_str())) {
            return Err(invalid(format!("timeouts.{key}: an undeclared field")));
        }
        for (slot, name) in SUITE_EXPORTS.iter().enumerate() {
            match fields.iter().find(|(key, _)| key == name).map(|(_, value)| value) {
                None => return Err(invalid(format!("timeouts.{name}: a declared field is missing"))),
                Some(NuVal::Nothing) => {}
                Some(NuVal::Duration(ns)) if *ns > 0 => nanos[slot] = *ns,
                Some(NuVal::Duration(0)) if *name == EXECUTE_EXPORT => nanos[slot] = 0,
                Some(other) => {
                    let zero = if *name == EXECUTE_EXPORT { ", or 0ns" } else { "" };
                    return Err(invalid(format!(
                        "timeouts.{name} is `{}`, not null or a positive duration{zero}",
                        other.to_nuon(NuonStyle::Default),
                    )));
                }
            }
        }
    }
    let [prepare, execute, status, stop] = nanos.map(PhaseTimeout);
    Ok(SuiteTimeouts {
        prepare,
        execute,
        status,
        stop,
    })
}

/// The call's heartbeats as gaps: two or more positive durations, else the default.
fn heartbeat_gaps(given: Option<&NuVal>) -> EngineResult<Vec<tk::TkDuration>> {
    let invalid = |reason: String| EngineError::HeartbeatsInvalid { reason };
    let Some(given) = given else {
        return Ok(DEFAULT_HEARTBEATS_SECS
            .iter()
            .map(|secs| tk::TkDuration::from_secs(*secs))
            .collect());
    };
    let NuVal::List(items) = given else {
        return Err(invalid(format!(
            "`heartbeats` is a list of durations, not `{}`",
            given.to_nuon(NuonStyle::Default),
        )));
    };
    if items.len() < MIN_HEARTBEATS {
        return Err(invalid(format!(
            "`heartbeats` names {MIN_HEARTBEATS} intervals or more; it names {}",
            items.len(),
        )));
    }
    items
        .iter()
        .enumerate()
        .map(|(index, item)| match item {
            NuVal::Duration(ns) if *ns > 0 => Ok(tk::TkDuration::from_nanos(*ns as u64)),
            other => Err(invalid(format!(
                "heartbeats.{index} is `{}`, not a positive duration",
                other.to_nuon(NuonStyle::Default),
            ))),
        })
        .collect()
}

/// The moment now, at UTC.
fn now_date() -> DateData {
    let since = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    DateData {
        seconds: i64::try_from(since.as_secs()).unwrap_or(i64::MAX),
        seconds_fraction: since.subsec_nanos(),
        utc_offset_minus: 0,
    }
}
