//! The MCP's end of a session: its hello, its requests, the worker's asks, its end.
use crate::*;

/// What a session tells its owner beside the responses to its requests.
#[derive(Debug)]
pub enum SessionEvent {
    /// An eval's ask of the MCP, answered through its responder.
    Ask { ask: SessionAsk, responder: Responder },
    /// An emergency the worker's operator raised.
    Emergency(Emergency),
    /// The session ended, and why.
    Ended(TransportError),
}

/// The way back for one ask's answer, which it sends at most once.
#[derive(Debug)]
pub struct Responder {
    id: u64,
    out: Outbox,
}

impl Responder {
    /// The ask answered; nothing is sent once the session has ended.
    pub fn answer(self, answer: HostAnswer) {
        let _ = self.out.frame(&McpFrame::Answer { id: self.id, answer });
    }
}

/// What the writer takes next: a frame, the stream's last frame, or its stop.
#[derive(Debug)]
enum Out {
    Frame(tku::Bytes),
    /// The close's frame, after which the stream ends.
    Last(tku::Bytes),
    /// The stream's end, with nothing more written.
    Stop,
}

/// The frames a session writes, encoded and held to its ceiling before they queue.
#[derive(Debug, Clone)]
struct Outbox {
    tx: tk::UnboundedSender<Out>,
    ceiling: u32,
}

impl Outbox {
    /// `frame` encoded and queued, refused past the ceiling or after the end.
    fn frame(&self, frame: &McpFrame) -> Result<(), TransportError> {
        self.queue(frame, Out::Frame)
    }

    /// `frame` queued as the stream's last.
    fn last(&self, frame: &McpFrame) -> Result<(), TransportError> {
        self.queue(frame, Out::Last)
    }

    /// The writer stopped once what is queued before is written.
    fn stop(&self) {
        let _ = self.tx.send(Out::Stop);
    }

    /// `frame` encoded, held to the ceiling, and queued as `kind` makes it.
    fn queue(&self, frame: &McpFrame, kind: fn(tku::Bytes) -> Out) -> Result<(), TransportError> {
        let bytes = payload(frame);
        if bytes.len() > self.ceiling as usize {
            return Err(TransportError::InvalidFrame {
                reason: format!("a frame of {} bytes passes the session's {}-byte ceiling", bytes.len(), self.ceiling),
            });
        }
        self.tx.send(kind(bytes)).map_err(|_| TransportError::SessionLost {
            reason: "the session has ended".to_string(),
        })
    }
}

/// What a session's tasks share: its requests in flight and its end.
#[derive(Debug, Default)]
struct Shared {
    pending: std::sync::Mutex<HashMap<u64, tk::oneshot::Sender<EngineResponse>>>,
    ended: std::sync::Mutex<Option<TransportError>>,
}

impl Shared {
    /// Why the session ended, once it has.
    fn end_reason(&self) -> TransportError {
        let ended = self.ended.lock().unwrap_or_else(|e| e.into_inner());
        match &*ended {
            Some(TransportError::SessionLost { reason }) => TransportError::SessionLost { reason: reason.clone() },
            Some(other) => TransportError::SessionLost { reason: other.message() },
            None => TransportError::SessionLost {
                reason: "the session ended".to_string(),
            },
        }
    }

    /// The end recorded once, every request still in flight let go with it.
    fn end(&self, reason: TransportError) -> bool {
        let mut ended = self.ended.lock().unwrap_or_else(|e| e.into_inner());
        if ended.is_some() {
            return false;
        }
        *ended = Some(reason);
        drop(ended);
        self.pending.lock().unwrap_or_else(|e| e.into_inner()).clear();
        true
    }

    /// Whether the session has ended.
    fn has_ended(&self) -> bool {
        self.ended.lock().unwrap_or_else(|e| e.into_inner()).is_some()
    }
}

/// The MCP's end of one session with a worker.
#[derive(Debug)]
pub struct Session {
    worker: WorkerHello,
    next: AtomicU64,
    out: Outbox,
    shared: Arc<Shared>,
    /// The reader and the pinger, which the drop stops; the writer flushes its close.
    tasks: Vec<tk::JoinHandle<()>>,
    /// The writer, which a finish awaits so the close leaves before the stream ends.
    writer: std::sync::Mutex<Option<tk::JoinHandle<()>>>,
}

impl Session {
    /// A session opened over `reader` and `writer`: the hello sent, the reply checked.
    pub async fn open<R, W>(
        reader: R,
        writer: W,
        hello: ClientHello,
    ) -> Result<(Self, tk::UnboundedReceiver<SessionEvent>), TransportError>
    where
        R: tk::AsyncRead + Unpin + Send + 'static,
        W: tk::AsyncWrite + Unpin + Send + 'static,
    {
        let mut frames = tku::FramedRead::new(reader, FrameCodec::new());
        let mut sink = tku::FramedWrite::new(writer, FrameCodec::new());
        sink.send(payload(&hello)).await.map_err(|e| lost(e.transport()))?;
        let reply = match tk::timeout(REPLY_DEADLINE, frames.next()).await {
            Err(_) => return Err(lost_text(format!("no reply from the worker in {} s", REPLY_DEADLINE.as_secs()))),
            Ok(None) => return Err(lost_text("the worker's stream ended before its reply".to_string())),
            Ok(Some(Err(e))) => return Err(lost(e.transport())),
            Ok(Some(Ok(bytes))) => decode_payload::<HelloReply>(&bytes, HELLO_BUDGET).map_err(|e| e.transport())?,
        };
        let worker = check_reply(reply, &hello)?;
        let limits = worker.effective;
        frames.decoder_mut().set_ceiling(limits.frame);
        sink.encoder_mut().set_ceiling(limits.frame);
        let (tx, rx) = tk::unbounded_channel();
        let out = Outbox { tx, ceiling: limits.frame };
        let shared = Arc::new(Shared::default());
        let (events_tx, events) = tk::unbounded_channel();
        let writer = tk::spawn(write_loop(sink, rx, shared.clone(), events_tx.clone()));
        let tasks = vec![
            tk::spawn(read_loop(frames, limits.decode_budget(), out.clone(), shared.clone(), events_tx)),
            tk::spawn(ping_loop(out.clone(), shared.clone())),
        ];
        Ok((
            Self {
                worker,
                next: AtomicU64::new(1),
                out,
                shared,
                tasks,
                writer: std::sync::Mutex::new(Some(writer)),
            },
            events,
        ))
    }

    /// The worker this session reached, as its reply named it.
    pub fn worker(&self) -> &WorkerHello {
        &self.worker
    }

    /// A request's response, or the session's loss where it ends first.
    pub async fn request(&self, request: EngineRequest) -> Result<EngineResponse, TransportError> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = tk::oneshot::channel();
        {
            let mut pending = self.shared.pending.lock().unwrap_or_else(|e| e.into_inner());
            if self.shared.has_ended() {
                return Err(self.shared.end_reason());
            }
            pending.insert(id, tx);
        }
        if let Err(e) = self.out.frame(&McpFrame::Request { id, request }) {
            self.shared.pending.lock().unwrap_or_else(|e| e.into_inner()).remove(&id);
            return Err(e);
        }
        rx.await.map_err(|_| self.shared.end_reason())
    }

    /// Whether the session has ended.
    pub fn has_ended(&self) -> bool {
        self.shared.has_ended()
    }

    /// The session ended by the MCP: its close the last frame, every request let go.
    pub fn close(&self) {
        if !self.shared.has_ended() {
            let _ = self.out.last(&McpFrame::Close(SessionClose { error: None }));
        }
        self.shared.end(lost_text("the MCP closed the session".to_string()));
    }

    /// The session closed, its close written and its stream ended within `deadline`.
    pub async fn finish(&self, deadline: Duration) {
        self.close();
        let writer = self.writer.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(writer) = writer {
            let _ = tk::timeout(deadline, writer).await;
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.close();
        for task in &self.tasks {
            task.abort();
        }
    }
}

/// A loss of the session for `error`'s reason.
fn lost(error: TransportError) -> TransportError {
    lost_text(error.message())
}

/// A loss of the session for `reason`.
fn lost_text(reason: String) -> TransportError {
    TransportError::SessionLost { reason }
}

/// The session's end told its owner, once.
fn ended(shared: &Shared, events: &tk::UnboundedSender<SessionEvent>, reason: TransportError) {
    if shared.end(reason.clone()) {
        let _ = events.send(SessionEvent::Ended(reason));
    }
}

/// Frames written in queue order until the last or the stop, then the stream ended.
async fn write_loop<W>(
    mut sink: tku::FramedWrite<W, FrameCodec>,
    mut rx: tk::UnboundedReceiver<Out>,
    shared: Arc<Shared>,
    events: tk::UnboundedSender<SessionEvent>,
) where
    W: tk::AsyncWrite + Unpin + Send + 'static,
{
    while let Some(out) = rx.recv().await {
        let (bytes, last) = match out {
            Out::Frame(bytes) => (bytes, false),
            Out::Last(bytes) => (bytes, true),
            Out::Stop => break,
        };
        if let Err(e) = sink.send(bytes).await {
            ended(&shared, &events, lost(e.transport()));
            return;
        }
        if last {
            break;
        }
    }
    let _ = sink.close().await;
}

/// The worker's frames, each sent to the request or the owner it is for.
async fn read_loop<R>(
    mut frames: tku::FramedRead<R, FrameCodec>,
    budget: DecodeBudget,
    out: Outbox,
    shared: Arc<Shared>,
    events: tk::UnboundedSender<SessionEvent>,
) where
    R: tk::AsyncRead + Unpin + Send + 'static,
{
    let reason = loop {
        let bytes = match tk::timeout(SESSION_IDLE, frames.next()).await {
            Err(_) => break lost_text(format!("no frame from the worker in {} s", SESSION_IDLE.as_secs())),
            Ok(None) => break lost_text("the worker's stream ended".to_string()),
            Ok(Some(Err(e))) => break lost(e.transport()),
            Ok(Some(Ok(bytes))) => bytes,
        };
        let frame = match decode_payload::<WorkerFrame>(&bytes, budget) {
            Ok(frame) => frame,
            Err(e) => break e.transport(),
        };
        match frame {
            WorkerFrame::Response { id, response } => {
                let waiter = shared.pending.lock().unwrap_or_else(|e| e.into_inner()).remove(&id);
                if let Some(waiter) = waiter {
                    let _ = waiter.send(response);
                }
            }
            WorkerFrame::Ask { id, ask } => {
                let responder = Responder { id, out: out.clone() };
                let _ = events.send(SessionEvent::Ask { ask, responder });
            }
            WorkerFrame::Emergency(emergency) => {
                let _ = events.send(SessionEvent::Emergency(emergency));
            }
            WorkerFrame::Pong(_) => {}
            WorkerFrame::Close(close) => {
                break close.error.unwrap_or_else(|| lost_text("the worker closed the session".to_string()));
            }
        }
    };
    ended(&shared, &events, reason);
    out.stop();
}

/// A ping each interval while the session lasts, so a quiet one stays live.
async fn ping_loop(out: Outbox, shared: Arc<Shared>) {
    let mut nonce = 0u64;
    loop {
        tk::sleep(PING_INTERVAL).await;
        if shared.has_ended() {
            return;
        }
        nonce += 1;
        if out.frame(&McpFrame::Ping(Ping { nonce })).is_err() {
            return;
        }
    }
}
