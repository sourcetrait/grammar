//! An await job: prepare in the call, execute as the job, status, and stop.
use crate::*;

/// The tool a job's evals run under, as its logs and the hung watch name it.
pub(crate) const AWAIT_TOOL: &str = "await";

/// Each definition's timeout where the call names none, a call's own default.
const DEFAULT_PHASE_TIMEOUT_NS: i64 = DEFAULT_TIMEOUT_MS as i64 * 1_000_000;

/// The gaps between beats where the call names none, in seconds.
const DEFAULT_HEARTBEATS_SECS: [u64; 7] = [30, 60, 300, 900, 1800, 2700, 3600];

/// The fewest gaps a call's heartbeats name.
const MIN_HEARTBEATS: usize = 2;

/// A job's trace under its directory, one event a line.
pub const TRACE_FILE: &str = "trace.nuonl";

/// The field each event's data names the call's nonce by.
const AWAIT_FIELD: &str = "await";

/// The field a Status's data holds status's value in.
const STATUS_FIELD: &str = "status";

/// The field a Result's or a Stop's data holds its definition's value in.
const RESULT_FIELD: &str = "result";

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
    /// `interface await stop` named the job, a stop by hand.
    Manual,
    /// The host's teardown.
    Teardown,
}

impl StopReason {
    /// The reason as a Stop event names it.
    fn as_str(self) -> &'static str {
        match self {
            Self::Timeout => "timeout",
            Self::Manual => "stop",
            Self::Teardown => "teardown",
        }
    }
}

/// A job the engine holds while it runs.
pub(crate) struct AwaitJob {
    nonce: String,
    dir: PathBuf,
    /// The module's text, each phase's source carrying it inline.
    module: String,
    def: SuiteDef,
    /// The PREPARED record's literal, the data execute, status, and stop take.
    literal: String,
    timeouts: SuiteTimeouts,
    job_id: nu::JobId,
    /// When the job started, its PREPARED record's `job.started`.
    started: DateData,
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
    /// The job's end of its event queue, which closes the queue as the job goes.
    events: EventSender,
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

/// A job's events waiting for its sender, in order, one Status at most.
#[derive(Default)]
pub(crate) struct EventQueue {
    waiting: std::sync::Mutex<Waiting>,
    /// Wakes the sender when an event is queued or the queue closes.
    ready: tk::Notify,
}

/// The events waiting, and whether the job has let the queue go.
#[derive(Default)]
struct Waiting {
    events: VecDeque<AwaitEvent>,
    closed: bool,
}

impl EventQueue {
    /// An event queued, a Status replacing any older Status still waiting.
    pub(crate) fn push(
        &self,
        event: AwaitEvent,
    ) {
        let mut waiting = self.waiting.lock().unwrap_or_else(|e| e.into_inner());
        if event.model == AWAIT_STATUS_MODEL {
            waiting.events.retain(|queued| queued.model != AWAIT_STATUS_MODEL);
        }
        waiting.events.push_back(event);
        drop(waiting);
        self.ready.notify_one();
    }

    /// The queue closed: its sender takes what waits, then ends.
    pub(crate) fn close(&self) {
        self.waiting.lock().unwrap_or_else(|e| e.into_inner()).closed = true;
        self.ready.notify_one();
    }

    /// The oldest event waiting, awaited; none once the queue is closed and empty.
    pub(crate) async fn next(&self) -> Option<AwaitEvent> {
        loop {
            {
                let mut waiting = self.waiting.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(event) = waiting.events.pop_front() {
                    return Some(event);
                }
                if waiting.closed {
                    return None;
                }
            }
            self.ready.notified().await;
        }
    }
}

/// The job's end of its event queue.
struct EventSender(Arc<EventQueue>);

impl Drop for EventSender {
    fn drop(&mut self) {
        self.0.close();
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
    let mut args = vec![Some(hold_data(&req.data, &def.data_type)?)];
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
    let source = build_suite_source(&req.module, PREPARE, &args).map_err(|reason| EngineError::Internal {
        phase: "await::source".to_string(),
        reason,
    })?;
    let value = eval_pooled(&core, dir.join(PREPARE), source, &nonce, AWAIT_TOOL, timeouts.prepare.ms(), true)
        .await?;
    check_value(&value, &def.prepared_type, "result")
        .map_err(|reason| EngineError::ResultInvalid { reason })?;
    let prepared = NuVal::try_from_nu(value).map_err(|e| EngineError::ResultInvalid {
        reason: format!("the result: {e}"),
    })?;
    let launch = Launch {
        nonce,
        dir,
        module: req.module,
        def,
        prepared,
        timeouts,
    };
    start(core, launch, gaps).await
}

/// What a job's start takes from its call.
struct Launch {
    nonce: String,
    dir: PathBuf,
    module: String,
    def: SuiteDef,
    prepared: NuVal,
    timeouts: SuiteTimeouts,
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
        module,
        def,
        prepared,
        timeouts,
    } = launch;
    let cancel = Arc::new(AtomicBool::new(false));
    let (tracker, mailbox) = make_job_tracker(cancel.clone(), &nonce);
    let job_id = core
        .env_jobs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .add_job(nu::Job::Thread(tracker.clone()));
    let execute_dir = dir.join(EXECUTE_EXPORT);
    let beats_from = tk::TkInstant::now();
    let started = now_date();
    let built = (|| -> EngineResult<(NuVal, String, String)> {
        let job = NuVal::Record(vec![
            ("id".to_string(), NuVal::Int(job_number(job_id))),
            ("started".to_string(), NuVal::Date(started.clone())),
        ]);
        let record = NuVal::Record(vec![
            ("job".to_string(), job),
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
        let source = build_suite_source(&module, EXECUTE_EXPORT, &execute_args).map_err(internal)?;
        fs::create_dir_all(&execute_dir).map_err(|e| internal(format!("{}: {e}", execute_dir.display())))?;
        Ok((record, literal, source))
    })();
    let (record, literal, source) = match built {
        Ok(built) => built,
        Err(error) => {
            core.env_jobs.lock().unwrap_or_else(|e| e.into_inner()).remove_job(job_id);
            return Err(error);
        }
    };
    let finished = Arc::new(AtomicBool::new(false));
    let started_at = now_millis();
    let queue = Arc::new(EventQueue::default());
    tk::spawn(send_events(core.host.clone(), queue.clone()));
    let state = Arc::new(AwaitJob {
        nonce: nonce.clone(),
        dir,
        module,
        def,
        literal,
        timeouts,
        job_id,
        started,
        started_at,
        cancel: cancel.clone(),
        tracker: tracker.clone(),
        finished: finished.clone(),
        settled: AtomicBool::new(false),
        done: AtomicBool::new(false),
        over: tk::Notify::new(),
        stopped: tk::Notify::new(),
        trace_lock: std::sync::Mutex::new(()),
        events: EventSender(queue),
    });
    {
        let mut jobs = core.jobs.lock().unwrap_or_else(|e| e.into_inner());
        if core.stopping.load(Ordering::SeqCst) {
            drop(jobs);
            core.env_jobs.lock().unwrap_or_else(|e| e.into_inner()).remove_job(job_id);
            return Err(EngineError::Internal {
                phase: "await::start".to_string(),
                reason: "the engine is stopping: no await job starts once its teardown has begun".to_string(),
            });
        }
        jobs.insert(nonce.clone(), state.clone());
    }
    register_in_flight(
        &core.in_flight,
        &nonce,
        InFlightEntry {
            cancel: cancel.clone(),
            tracker: tracker.clone(),
            finished: finished.clone(),
        },
    )
    .await;
    core.executor.refresh_base_if_stale();
    let mut engine = core.executor.take_clone();
    engine.current_job = nu::CurrentJob {
        id: job_id,
        background_thread_job: Some(tracker),
        mailbox: Arc::new(std::sync::Mutex::new(mailbox)),
    };
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
    tk::spawn(run_beats(core, state, gaps, beats_from));
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
                let data = event_data(&job, Vec::new(), RESULT_FIELD, value);
                emit(&job, AWAIT_RESULT_MODEL, data);
            }
        }
        () = timed_out => stop_job(&core, &job, StopReason::Timeout).await,
        () = job.stopped.notified() => {}
    }
    finish(&core, &job).await;
}

/// The beats: status run at each time of the schedule while the job runs.
async fn run_beats(
    core: Arc<EngineCore>,
    job: Arc<AwaitJob>,
    gaps: Vec<tk::TkDuration>,
    from: tk::TkInstant,
) {
    let gap = |beat: usize| gaps[beat.min(gaps.len().saturating_sub(1))];
    let mut beat: usize = 0;
    let mut at = from + gap(beat);
    loop {
        if job.done.load(Ordering::SeqCst) {
            return;
        }
        tokio::select! {
            () = tk::sleep_until(at) => {}
            () = job.over.notified() => return,
        }
        if job.done.load(Ordering::SeqCst) {
            return;
        }
        let value = run_phase(&core, &job, STATUS, &job.def.status, job.timeouts.status).await;
        let data = event_data(&job, Vec::new(), STATUS_FIELD, value);
        emit(&job, AWAIT_STATUS_MODEL, data);
        beat += 1;
        at += gap(beat);
        (beat, at) = catch_up(&gaps, beat, at, tk::TkInstant::now());
    }
}

/// The next beat once one has run: every time passed skipped but the latest.
pub(crate) fn catch_up(
    gaps: &[tk::TkDuration],
    mut beat: usize,
    mut at: tk::TkInstant,
    now: tk::TkInstant,
) -> (usize, tk::TkInstant) {
    let last = gaps.len().saturating_sub(1);
    let gap = |beat: usize| gaps[beat.min(last)];
    while beat < last && at <= now && at + gap(beat + 1) <= now {
        beat += 1;
        at += gap(beat);
    }
    if beat >= last && at <= now {
        let elapsed = (now - at).as_nanos();
        let step = gap(last).as_nanos().max(1);
        beat = beat.saturating_add(usize::try_from(elapsed / step).unwrap_or(usize::MAX));
        at = now -tk::TkDuration::from_nanos(u64::try_from(elapsed % step).unwrap_or(0));
    }
    (beat, at)
}

/// A job's stop, run once: stop's eval, the job killed, then its Stop event.
async fn stop_job(
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
    let data = event_data(job, vec![reason_field], RESULT_FIELD, value);
    if reason == StopReason::Teardown {
        job.trace(AWAIT_STOP_MODEL, &data);
    } else {
        emit(job, AWAIT_STOP_MODEL, data);
    }
    job.stopped.notify_one();
}

/// Every running job's stop at once, each bounded by its own timeout.
pub(crate) async fn stop_jobs(
    core: &Arc<EngineCore>,
    reason: StopReason,
) {
    let jobs: Vec<Arc<AwaitJob>> = {
        let held = core.jobs.lock().unwrap_or_else(|e| e.into_inner());
        if reason == StopReason::Teardown {
            core.stopping.store(true, Ordering::SeqCst);
        }
        held.values().cloned().collect()
    };
    let mut stops = tk::JoinSet::new();
    for job in jobs {
        let core = core.clone();
        stops.spawn(async move { stop_job(&core, &job, reason).await });
    }
    while stops.join_next().await.is_some() {}
}

/// The job running under a call's nonce, none once it is over.
fn running_job(
    core: &EngineCore,
    nonce: &str,
) -> Option<Arc<AwaitJob>> {
    core.jobs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(nonce)
        .filter(|job| !job.done.load(Ordering::SeqCst))
        .cloned()
}

/// Every job running, `{await, job, started}` a row, in the order they started.
pub(crate) fn await_list(core: &EngineCore) -> NuVal {
    let mut jobs: Vec<Arc<AwaitJob>> = core
        .jobs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .values()
        .filter(|job| !job.done.load(Ordering::SeqCst))
        .cloned()
        .collect();
    jobs.sort_by_key(|job| job.job_id.get());
    let rows = jobs
        .iter()
        .map(|job| {
            NuVal::Record(vec![
                (AWAIT_FIELD.to_string(), NuVal::String(job.nonce.clone())),
                ("job".to_string(), NuVal::Int(job_number(job.job_id))),
                ("started".to_string(), NuVal::Date(job.started.clone())),
            ])
        })
        .collect();
    NuVal::List(rows)
}

/// A job's status run now, `{await, status}` or `{await, error}`, sending nothing.
pub(crate) async fn await_status(
    core: &EngineCore,
    nonce: &str,
) -> EngineResult<NuVal> {
    let job = running_job(core, nonce).ok_or_else(|| unknown_await(nonce))?;
    let value = run_phase(core, &job, STATUS, &job.def.status, job.timeouts.status).await;
    Ok(event_data(&job, Vec::new(), STATUS_FIELD, value))
}

/// A job stopped by hand: its stop run, the job killed, its Stop packet sent.
pub(crate) async fn await_stop(
    core: &EngineCore,
    nonce: &str,
) -> EngineResult<()> {
    let job = running_job(core, nonce).ok_or_else(|| unknown_await(nonce))?;
    stop_job(core, &job, StopReason::Manual).await;
    Ok(())
}

/// The refusal of an await command naming no running job.
fn unknown_await(nonce: &str) -> EngineError {
    EngineError::AwaitUnknown {
        nonce: nonce.to_string(),
    }
}

/// A job's id as its records carry it.
fn job_number(job_id: nu::JobId) -> i64 {
    i64::try_from(job_id.get()).unwrap_or(i64::MAX)
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
    let source = build_suite_source(&job.module, export, &args).map_err(|reason| EngineError::Internal {
        phase: format!("await::{export}"),
        reason,
    })?;
    let value = eval_pooled(core, job.dir.join(export), source, &job.nonce, AWAIT_TOOL, timeout.ms(), false)
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
) -> EngineResult<Vec<Option<String>>> {
    let mut args = vec![Some(literal.to_string())];
    if takes_timeout {
        args.push(Some(timeout.literal()?));
    }
    Ok(args)
}

/// A parameter prepare takes as the call gave it, `None` where it is absent.
fn given_literal(
    given: Option<&NuVal>,
    holder: &str,
) -> EngineResult<Option<String>> {
    let Some(value) = given else {
        return Ok(None);
    };
    let internal = |reason: String| EngineError::Internal {
        phase: format!("await::{holder}"),
        reason,
    };
    let value = value
        .clone()
        .try_into_nu(nu::Span::unknown())
        .map_err(|e| internal(e.to_string()))?;
    render_nuon(&value).map(Some).map_err(internal)
}

/// An event's data: `await`, its fields, then a value or its error's rows.
fn event_data(
    job: &AwaitJob,
    fields: Vec<(String, NuVal)>,
    value_field: &str,
    value: EngineResult<NuVal>,
) -> NuVal {
    let mut data = vec![(AWAIT_FIELD.to_string(), NuVal::String(job.nonce.clone()))];
    data.extend(fields);
    match value {
        Ok(value) => data.push((value_field.to_string(), value)),
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

/// An event traced, then queued for the job's sender; its caller goes on.
fn emit(
    job: &AwaitJob,
    model: &'static str,
    data: NuVal,
) {
    job.trace(model, &data);
    job.events.0.push(AwaitEvent {
        model: model.to_string(),
        data,
    });
}

/// A job's events sent to its host one at a time, in the order they were queued.
async fn send_events(
    host: HostLink,
    queue: Arc<EventQueue>,
) {
    while let Some(event) = queue.next().await {
        let model = event.model.clone();
        let host = host.clone();
        match tk::spawn_blocking(move || host.ask(HostAsk::Await(event))).await {
            Ok(Ok(HostAnswer::Sent(_))) => {}
            Ok(Ok(HostAnswer::Refused(reason)) | Err(reason)) => {
                eprintln!("grammar: an await job's {model} not sent: {reason}");
            }
            Ok(Ok(_)) => eprintln!("grammar: an await job's {model} met an answer that is not a send's"),
            Err(e) => eprintln!("grammar: an await job's {model} not sent: {e}"),
        }
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
