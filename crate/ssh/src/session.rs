//! grammar-ssh's session: the MCP's hello answered, its requests served, its end.
use crate::*;

/// Why a session ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEnd {
    /// The MCP closed it.
    Closed,
    /// The MCP's stream ended.
    StreamEnded,
    /// No frame came within the idle window.
    Idle,
    /// A stop signal, by its name.
    Signal(&'static str),
    /// The stream broke or carried a frame the session refuses.
    Broken(String),
}

impl fmt::Display for SessionEnd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => write!(f, "the MCP closed the session"),
            Self::StreamEnded => write!(f, "the MCP's stream ended"),
            Self::Idle => write!(f, "no frame from the MCP in {} s", SESSION_IDLE.as_secs()),
            Self::Signal(name) => write!(f, "{name}"),
            Self::Broken(reason) => write!(f, "the session broke: {reason}"),
        }
    }
}

/// Why a frame grammar-ssh meant to send wasn't queued.
#[derive(Debug)]
pub(crate) enum Unsent {
    /// The frame passes the session's ceiling.
    TooLarge { len: usize, ceiling: u32 },
    /// The session has ended.
    Ended,
}

impl fmt::Display for Unsent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { len, ceiling } => {
                write!(f, "a frame of {len} bytes passes the session's {ceiling}-byte ceiling")
            }
            Self::Ended => write!(f, "the session has ended"),
        }
    }
}

/// The frames a session writes, encoded and held to its bounds before they queue.
#[derive(Debug, Clone)]
pub(crate) struct Outbox {
    tx: tk::UnboundedSender<tku::Bytes>,
    ceiling: u32,
    budget: DecodeBudget,
}

impl Outbox {
    /// `frame` encoded and queued for the MCP, refused past its ceiling or its budget.
    pub(crate) fn send(&self, frame: &SshFrame) -> Result<(), Unsent> {
        let bytes = payload(frame);
        if bytes.len() > self.ceiling as usize || decode_payload::<SshFrame>(&bytes, self.budget).is_err() {
            return Err(Unsent::TooLarge {
                len: bytes.len(),
                ceiling: self.ceiling,
            });
        }
        self.tx.send(bytes).map_err(|_| Unsent::Ended)
    }
}

/// The way to the session's MCP: its outbox while attached, and the asks waiting.
#[derive(Debug, Default)]
pub(crate) struct Asker {
    out: std::sync::Mutex<Option<Outbox>>,
    next: AtomicU64,
    pending: std::sync::Mutex<HashMap<u64, tk::oneshot::Sender<HostAnswer>>>,
}

impl Asker {
    /// A session's outbox attached, the asks after it carried on it.
    pub(crate) fn attach(&self, out: Outbox) {
        *held(&self.out) = Some(out);
    }

    /// The session's outbox let go, each ask still waiting refused.
    pub(crate) fn detach(&self) {
        held(&self.out).take();
        held(&self.pending).clear();
    }

    /// Whether a session is attached.
    pub(crate) fn attached(&self) -> bool {
        held(&self.out).is_some()
    }

    /// The MCP's answer to the ask under `id`, to the eval waiting on it.
    pub(crate) fn answer(&self, id: u64, answer: HostAnswer) {
        if let Some(waiter) = held(&self.pending).remove(&id) {
            let _ = waiter.send(answer);
        }
    }

    /// An eval's ask of the MCP, refused without a session or an answer in time.
    pub(crate) async fn ask(&self, ask: SessionAsk) -> HostAnswer {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = tk::oneshot::channel();
        {
            let out = held(&self.out);
            let Some(out) = out.as_ref() else {
                return HostAnswer::Refused("no session carries the ask".to_string());
            };
            held(&self.pending).insert(id, tx);
            if let Err(unsent) = out.send(&SshFrame::Ask { id, ask }) {
                held(&self.pending).remove(&id);
                return HostAnswer::Refused(unsent.to_string());
            }
        }
        match tk::timeout(ASK_DEADLINE, rx).await {
            Ok(Ok(answer)) => answer,
            Ok(Err(_)) => HostAnswer::Refused("the session ended before the ask's answer".to_string()),
            Err(_) => {
                held(&self.pending).remove(&id);
                HostAnswer::Refused(format!("no answer to the ask in {} s", ASK_DEADLINE.as_secs()))
            }
        }
    }

    /// An emergency to the session's MCP, where one is attached.
    pub(crate) fn emergency(&self, emergency: Emergency) {
        if let Some(out) = held(&self.out).as_ref() {
            let _ = out.send(&SshFrame::Emergency(emergency));
        }
    }
}

/// A session's failure to open.
fn failed(reason: impl fmt::Display) -> GrammarSshError {
    GrammarSshError::Session {
        reason: reason.to_string(),
    }
}

/// The MCP's hello within its deadline, an undecodable one refused on the way.
async fn read_hello<R, W>(
    frames: &mut tku::FramedRead<R, FrameCodec>,
    sink: &mut tku::FramedWrite<W, FrameCodec>,
) -> GrammarSshResult<ClientHello>
where
    R: tk::AsyncRead + Unpin,
    W: tk::AsyncWrite + Unpin,
{
    let bytes = match tk::timeout(HELLO_DEADLINE, frames.next()).await {
        Err(_) => return Err(failed(format!("no hello in {} s", HELLO_DEADLINE.as_secs()))),
        Ok(None) => return Err(failed("the stream ended before its hello")),
        Ok(Some(Err(e))) => return Err(failed(e)),
        Ok(Some(Ok(bytes))) => bytes,
    };
    match decode_payload::<ClientHello>(&bytes, HELLO_BUDGET) {
        Ok(hello) => Ok(hello),
        Err(e) => {
            let _ = sink.send(payload(&HelloReply::Refused(e.transport()))).await;
            Err(failed(e))
        }
    }
}

/// A start that failed: the MCP's hello read and refused with why.
pub(crate) async fn refuse<R, W>(
    reader: R,
    writer: W,
    refusal: TransportError,
) where
    R: tk::AsyncRead + Unpin,
    W: tk::AsyncWrite + Unpin,
{
    let mut frames = tku::FramedRead::new(reader, FrameCodec::new());
    let mut sink = tku::FramedWrite::new(writer, FrameCodec::new());
    if read_hello(&mut frames, &mut sink).await.is_ok() {
        let _ = sink.send(payload(&HelloReply::Refused(refusal))).await;
        let _ = sink.close().await;
    }
}

/// The session on `reader` and `writer`: its hello answered, its frames served.
pub(crate) async fn serve<R, W>(
    reader: R,
    writer: W,
    profile: SshProfile,
    info: SshInfo,
    controller: &GrammarSshController,
) -> GrammarSshResult<SessionEnd>
where
    R: tk::AsyncRead + Unpin + Send + 'static,
    W: tk::AsyncWrite + Unpin + Send + 'static,
{
    let mut frames = tku::FramedRead::new(reader, FrameCodec::new());
    let mut sink = tku::FramedWrite::new(writer, FrameCodec::new());
    let hello = read_hello(&mut frames, &mut sink).await?;
    let reply = profile.reply(&hello);
    sink.send(payload(&reply)).await.map_err(failed)?;
    let limits = match reply {
        HelloReply::Accepted(accepted) => accepted.effective,
        HelloReply::Refused(refusal) => return Err(failed(refusal.message())),
    };
    frames.decoder_mut().set_ceiling(limits.frame);
    sink.encoder_mut().set_ceiling(limits.frame);
    let (tx, rx) = tk::unbounded_channel();
    let out = Outbox {
        tx,
        ceiling: limits.frame,
        budget: limits.decode_budget(),
    };
    let writer = tk::spawn(write_loop(sink, rx));
    controller.asker().attach(out.clone());
    let end = serve_frames(frames, limits, &out, Arc::new(info), controller).await;
    if matches!(end, SessionEnd::Idle | SessionEnd::Signal(_)) {
        let error = TransportError::SessionLost { reason: end.to_string() };
        let _ = out.send(&SshFrame::Close(SessionClose { error: Some(error) }));
    }
    drop(out);
    let _ = tk::timeout(DRAIN_DEADLINE, writer).await;
    Ok(end)
}

/// Each queued frame written in order, the stream closed once nothing can queue.
async fn write_loop<W>(
    mut sink: tku::FramedWrite<W, FrameCodec>,
    mut rx: tk::UnboundedReceiver<tku::Bytes>,
) where
    W: tk::AsyncWrite + Unpin,
{
    while let Some(bytes) = rx.recv().await {
        if sink.send(bytes).await.is_err() {
            return;
        }
    }
    let _ = sink.close().await;
}

/// The stop signals a session ends on, each by its name.
#[cfg(unix)]
struct Stops {
    term: tk::Signal,
    int: tk::Signal,
    hup: tk::Signal,
}

#[cfg(unix)]
impl Stops {
    /// The three stop signals taken, none where the process can't take them.
    fn take() -> Option<Self> {
        let take = |kind: tk::SignalKind| match tk::signal(kind) {
            Ok(signal) => Some(signal),
            Err(e) => {
                eprintln!("grammar: a stop signal can't be taken: {e}");
                None
            }
        };
        Some(Self {
            term: take(tk::SignalKind::terminate())?,
            int: take(tk::SignalKind::interrupt())?,
            hup: take(tk::SignalKind::hangup())?,
        })
    }

    /// The next stop signal's name.
    async fn next(&mut self) -> &'static str {
        tk::select! {
            _ = self.term.recv() => "SIGTERM",
            _ = self.int.recv() => "SIGINT",
            _ = self.hup.recv() => "SIGHUP",
        }
    }
}

/// The console events a session ends on: break, Ctrl-C, and the console's close.
#[cfg(windows)]
struct Stops {
    brk: tk::CtrlBreak,
    c: tk::CtrlC,
    close: tk::CtrlClose,
}

#[cfg(windows)]
impl Stops {
    /// The three console events taken, none where the process can't take them.
    fn take() -> Option<Self> {
        fn taken<T>(event: io::Result<T>) -> Option<T> {
            event
                .inspect_err(|e| eprintln!("grammar: a stop event can't be taken: {e}"))
                .ok()
        }
        Some(Self {
            brk: taken(tk::ctrl_break())?,
            c: taken(tk::ctrl_c())?,
            close: taken(tk::ctrl_close())?,
        })
    }

    /// The next console event's name.
    async fn next(&mut self) -> &'static str {
        tk::select! {
            _ = self.brk.recv() => "CTRL_BREAK",
            _ = self.c.recv() => "CTRL_C",
            _ = self.close.recv() => "CTRL_CLOSE",
        }
    }
}

/// The next stop signal's name, never where none could be taken.
async fn next_stop(stops: &mut Option<Stops>) -> &'static str {
    match stops {
        Some(stops) => stops.next().await,
        None => std::future::pending().await,
    }
}

/// The session's frames served until its end, then its calls cancelled and drained.
async fn serve_frames<R>(
    mut frames: tku::FramedRead<R, FrameCodec>,
    limits: Limits,
    out: &Outbox,
    info: Arc<SshInfo>,
    controller: &GrammarSshController,
) -> SessionEnd
where
    R: tk::AsyncRead + Unpin,
{
    let budget = limits.decode_budget();
    let calls = Arc::new(tk::Semaphore::new(limits.calls as usize));
    let control = Arc::new(tk::Semaphore::new(limits.control as usize));
    let in_flight = Arc::new(std::sync::Mutex::new(HashSet::new()));
    let mut tasks = tk::JoinSet::new();
    let mut stops = Stops::take();
    let end = loop {
        tk::select! {
            next = tk::timeout(SESSION_IDLE, frames.next()) => {
                let bytes = match next {
                    Err(_) => break SessionEnd::Idle,
                    Ok(None) => break SessionEnd::StreamEnded,
                    Ok(Some(Err(e))) => break SessionEnd::Broken(e.to_string()),
                    Ok(Some(Ok(bytes))) => bytes,
                };
                let frame = match decode_payload::<McpFrame>(&bytes, budget) {
                    Ok(frame) => frame,
                    Err(e) => {
                        let _ = out.send(&SshFrame::Close(SessionClose { error: Some(e.transport()) }));
                        break SessionEnd::Broken(e.to_string());
                    }
                };
                match frame {
                    McpFrame::Request { id, request } => {
                        if !held(&in_flight).insert(id) {
                            let error = TransportError::InvalidFrame { reason: format!("request {id} is already in flight") };
                            let _ = out.send(&SshFrame::Close(SessionClose { error: Some(error) }));
                            break SessionEnd::Broken(format!("request {id} sent again while in flight"));
                        }
                        let slots = if request.is_call() { &calls } else { &control };
                        let Ok(permit) = slots.clone().try_acquire_owned() else {
                            held(&in_flight).remove(&id);
                            send_response(out, id, EngineResponse::Refused(TransportError::Busy));
                            continue;
                        };
                        let engine = controller.engine();
                        let (out, info, in_flight) = (out.clone(), info.clone(), in_flight.clone());
                        tasks.spawn(async move {
                            let response = respond(request, engine, &info).await;
                            send_response(&out, id, response);
                            held(&in_flight).remove(&id);
                            drop(permit);
                        });
                    }
                    McpFrame::Answer { id, answer } => controller.asker().answer(id, answer),
                    McpFrame::Ping(ping) => {
                        let _ = out.send(&SshFrame::Pong(ping));
                    }
                    McpFrame::Close(_) => break SessionEnd::Closed,
                }
            }
            name = next_stop(&mut stops) => break SessionEnd::Signal(name),
            Some(_) = tasks.join_next(), if !tasks.is_empty() => {}
        }
    };
    controller.asker().detach();
    controller.teardown().await;
    let drained = tk::timeout(DRAIN_DEADLINE, async {
        while tasks.join_next().await.is_some() {}
    })
    .await;
    if drained.is_err() {
        eprintln!("grammar: calls still running after {} s are dropped", DRAIN_DEADLINE.as_secs());
        tasks.abort_all();
    }
    end
}

/// A response sent once, a result past the frame refused in its place.
fn send_response(
    out: &Outbox,
    id: u64,
    response: EngineResponse,
) {
    if let Err(Unsent::TooLarge { ceiling, .. }) = out.send(&SshFrame::Response { id, response }) {
        let refused = EngineResponse::Refused(TransportError::ResultTooLarge {
            limit: u64::from(ceiling),
        });
        let _ = out.send(&SshFrame::Response { id, response: refused });
    }
}

/// A request's answer from grammar-ssh's engine, or its info.
async fn respond(
    request: EngineRequest,
    engine: EngineResult<Arc<EngineHandle>>,
    info: &SshInfo,
) -> EngineResponse {
    let engine = match engine {
        Ok(engine) => engine,
        Err(_) if matches!(request, EngineRequest::Info) => return EngineResponse::Info(info.clone()),
        Err(e) => return EngineResponse::Refused(TransportError::Unavailable { reason: e.to_string() }),
    };
    match request {
        EngineRequest::Wire(request) => EngineResponse::Wire(wire(request, &engine).await),
        EngineRequest::Purview(request) => EngineResponse::Purview(purview(request, &engine).await),
        EngineRequest::Info => EngineResponse::Info(info.clone()),
    }
}

/// An eval's, a rerun's, a call's, or a command's answer, run here as local.
async fn wire(
    request: ToEngineSys,
    engine: &EngineHandle,
) -> FromEngineSys {
    match request {
        ToEngineSys::Execute(mut req) => {
            req.host = Host::Local;
            let nonce = req.nonce;
            FromEngineSys::Value(value(nonce, engine.request(req).await))
        }
        ToEngineSys::Rerun(mut req) => {
            req.host = Host::Local;
            let nonce = req.nonce;
            FromEngineSys::Value(value(nonce, engine.request(req).await))
        }
        ToEngineSys::Call(mut req) => {
            req.host = Host::Local;
            let nonce = req.nonce;
            FromEngineSys::Value(value(nonce, engine.request(req).await))
        }
        ToEngineSys::Control(mut req) => {
            req.host = Host::Local;
            FromEngineSys::Control(engine.request(req).await.and_then(|answer| answer))
        }
        ToEngineSys::Bed(mut req) => {
            req.host = Host::Local;
            let nonce = req.nonce;
            FromEngineSys::Value(value(nonce, engine.request(req).await))
        }
    }
}

/// A call's answer, or the engine's failure to take it as its result.
fn value(
    nonce: datum::Nonce,
    answer: EngineResult<ValueAnswer>,
) -> ValueAnswer {
    answer.unwrap_or_else(|e| ValueAnswer { nonce, result: Err(e) })
}

/// A purview request's answer, the engine's failure to take it as a runtime one.
async fn purview(
    request: PurviewRequest,
    engine: &EngineHandle,
) -> PurviewAnswer {
    fn flat<T>(answer: EngineResult<Result<T, RuntimeError>>) -> Result<T, RuntimeError> {
        answer
            .map_err(|e| RuntimeError::Internal {
                phase: "ssh::purview".to_string(),
                reason: e.to_string(),
            })
            .and_then(|answer| answer)
    }
    match request {
        PurviewRequest::List(req) => PurviewAnswer::Listed(flat(engine.request(req).await)),
        PurviewRequest::Configure(req) => PurviewAnswer::Rows(flat(engine.request(req).await)),
        PurviewRequest::Resolve(req) => PurviewAnswer::Resolved(flat(engine.request(req).await)),
        PurviewRequest::Rig(req) => PurviewAnswer::Rows(flat(engine.request(req).await)),
    }
}
