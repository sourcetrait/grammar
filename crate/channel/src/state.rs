use crate::*;

/// RFC 6455 policy violation - the peer was emitted to before verifying.
const CLOSE_UNVERIFIED_EMIT: u16 = 1008;
const UNVERIFIED_EMIT_REASON: &str = "emit before verification";

/// What a step sends its controller, under the epoch it concerns.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Up {
    pub(crate) epoch: ChannelEpoch,
    pub(crate) kind: UpKind,
}

/// One thing the Channel sends its controller unasked.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum UpKind {
    /// A packet line for the face's hub.
    Line(String),
    /// The hub's close frame.
    CloseHub { code: u16, reason: String },
    /// An emergency for the controller's policy.
    Emergency(Emergency),
}

/// Where a line goes once it passes the gate.
enum Sink {
    /// Up to the controller, for the face's hub.
    Hub,
    /// The hub's peer left, so a line has nowhere to go.
    Gone,
    /// The lines a sync takes, and how many the cap dropped since the last.
    Sync {
        lines: VecDeque<String>,
        dropped: u64,
    },
}

/// A body's send past its checks: its id, the files it writes, and its line.
pub(crate) struct PlannedSend {
    pub(crate) id: String,
    pub(crate) files: Vec<(PathBuf, String)>,
    pub(crate) line: String,
}

/// The logical Channel: its phase, its claim, its gate, its sink, and its spam.
pub(crate) struct Channel {
    phase: ChannelPhase,
    /// Set at an open, cleared at a close.
    mode: Option<ChannelMode>,
    /// The open's generation, kept past its close until the next open.
    epoch: ChannelEpoch,
    claimed: bool,
    sink: Option<Sink>,
    /// The inbox directory, set at open; a deliberate close takes it.
    inbox: Option<PathBuf>,
    spam: Spam,
    /// Mints every `MsgId` on this channel, so all ids share one counter.
    nonce_gen: datum::NonceGenerator,
    /// What the step in hand sends its controller, taken after each.
    ups: Vec<Up>,
}

impl Channel {
    /// A closed Channel on its starting policy.
    pub(crate) fn new(thresholds: SpamThresholds) -> Self {
        Self {
            phase: ChannelPhase::Closed,
            mode: None,
            epoch: ChannelEpoch(0),
            claimed: false,
            sink: None,
            inbox: None,
            spam: Spam::new(thresholds),
            nonce_gen: datum::NonceGenerator::new(),
            ups: Vec::new(),
        }
    }

    /// What the last steps send up, emptied.
    pub(crate) fn take_ups(&mut self) -> Vec<Up> {
        std::mem::take(&mut self.ups)
    }

    fn up(
        &mut self,
        kind: UpKind,
    ) {
        self.ups.push(Up {
            epoch: self.epoch,
            kind,
        });
    }

    /// The Channel's state and its inbox.
    pub(crate) fn status(&self) -> ChannelStatus {
        let sync_dropped = match &self.sink {
            Some(Sink::Sync { dropped, .. }) => *dropped,
            _ => 0,
        };
        ChannelStatus {
            state: ChannelState {
                phase: self.phase,
                mode: self.mode,
                epoch: self.epoch,
                claimed: self.claimed,
                sync_dropped,
            },
            inbox: self.inbox.as_ref().map(|dir| dir.to_string_lossy().into_owned()),
        }
    }

    /// Open in the request's mode under its epoch, a sync greeting queued first.
    pub(crate) fn open(
        &mut self,
        req: OpenRequest,
    ) {
        self.phase = ChannelPhase::Open;
        self.mode = Some(req.mode);
        self.epoch = req.epoch;
        self.claimed = false;
        self.sink = Some(match req.mode {
            ChannelMode::Async => Sink::Hub,
            ChannelMode::Sync => Sink::Sync {
                lines: req.greeting.into_iter().collect(),
                dropped: 0,
            },
        });
        self.inbox = Some(PathBuf::from(req.inbox));
    }

    /// Take a sync Channel's queued lines and its drop count, claiming it.
    pub(crate) fn take_sync(&mut self) -> Result<Synced, ChannelSyncError> {
        if matches!(self.phase, ChannelPhase::Closed) {
            return Err(ChannelSyncError::NotOpen);
        }
        let taken = match &mut self.sink {
            Some(Sink::Sync { lines, dropped }) => Synced {
                lines: lines.drain(..).collect(),
                dropped: std::mem::take(dropped),
            },
            _ => {
                return Err(ChannelSyncError::WrongMode(
                    self.mode.unwrap_or(ChannelMode::Async),
                ));
            }
        };
        self.claimed = true;
        Ok(taken)
    }

    /// The hub's claim or its peer's end, for the open it concerns alone.
    pub(crate) fn hub_notice(
        &mut self,
        notice: HubNotice,
    ) {
        match notice {
            HubNotice::Claimed(epoch) => {
                if epoch == self.epoch && !matches!(self.phase, ChannelPhase::Closed) {
                    self.claimed = true;
                }
            }
            HubNotice::PeerGone(epoch) => {
                if epoch == self.epoch && matches!(self.sink, Some(Sink::Hub)) {
                    self.sink = Some(Sink::Gone);
                }
            }
        }
    }

    /// Promote Open to Verified once a peer has claimed, answering the epoch.
    pub(crate) fn mark_verified(&mut self) -> Result<ChannelEpoch, ChannelVerifyError> {
        if matches!(self.phase, ChannelPhase::Closed) {
            return Err(ChannelVerifyError::NotOpen);
        }
        if !self.claimed {
            return Err(ChannelVerifyError::NotClaimed(
                self.mode.unwrap_or(ChannelMode::Async),
            ));
        }
        self.phase = ChannelPhase::Verified;
        Ok(self.epoch)
    }

    /// A close request: the Channel closed, and its inbox taken when asked.
    pub(crate) fn close_request(
        &mut self,
        req: CloseRequest,
    ) -> Closed {
        let closed = self.close(req.code, &req.reason);
        let inbox = if req.prune {
            self.take_inbox().map(|dir| dir.to_string_lossy().into_owned())
        } else {
            None
        };
        Closed { closed, inbox }
    }

    /// Tear the Channel down; a hub's close frame goes up with its code.
    pub(crate) fn close(
        &mut self,
        code: u16,
        reason: &str,
    ) -> bool {
        if matches!(self.phase, ChannelPhase::Closed) {
            return false;
        }
        self.close_now(code, reason);
        true
    }

    /// Close the open of `epoch` only while still unverified.
    pub(crate) fn close_if_unverified(
        &mut self,
        epoch: ChannelEpoch,
        code: u16,
        reason: &str,
    ) -> bool {
        if epoch != self.epoch || !matches!(self.phase, ChannelPhase::Open) {
            return false;
        }
        self.close_now(code, reason);
        true
    }

    /// The teardown itself, which every close shares.
    fn close_now(
        &mut self,
        code: u16,
        reason: &str,
    ) {
        if matches!(self.sink, Some(Sink::Hub | Sink::Gone)) {
            self.up(UpKind::CloseHub {
                code,
                reason: reason.to_string(),
            });
        }
        self.phase = ChannelPhase::Closed;
        self.mode = None;
        self.claimed = false;
        self.sink = None;
    }

    /// Take the inbox path, clearing it - the prune handoff for a close.
    pub(crate) fn take_inbox(&mut self) -> Option<PathBuf> {
        self.inbox.take()
    }

    /// One line into the sink the open installed.
    fn push(
        &mut self,
        line: String,
    ) -> Result<(), ChannelSendError> {
        match self.sink.as_mut() {
            Some(Sink::Sync { lines, dropped }) => {
                if lines.len() >= SYNC_QUEUE_CAP {
                    lines.pop_front();
                    *dropped += 1;
                }
                lines.push_back(line);
                return Ok(());
            }
            Some(Sink::Hub) => {}
            Some(Sink::Gone) | None => return Err(ChannelSendError::HubGone),
        }
        self.up(UpKind::Line(line));
        Ok(())
    }

    /// A host packet, bypassing the verification gate.
    pub(crate) fn send_control(
        &mut self,
        line: String,
    ) -> Result<(), ChannelSendError> {
        if matches!(self.phase, ChannelPhase::Closed) {
            return Err(ChannelSendError::NotOpen);
        }
        self.push(line)
    }

    /// The emit path; the phase is read here, at the moment of sending.
    pub(crate) fn emit(
        &mut self,
        line: String,
    ) -> Result<(), ChannelSendError> {
        match self.phase {
            ChannelPhase::Closed => Err(ChannelSendError::NotOpen),
            ChannelPhase::Open => Err(ChannelSendError::NotVerified),
            ChannelPhase::Verified => self.push(line),
        }
    }

    /// A host packet through the gate: rendered, its id minted, and emitted.
    pub(crate) fn emit_packet(
        &mut self,
        req: EmitRequest,
    ) -> Result<String, EmitError> {
        let EmitRequest {
            from,
            model,
            event,
            attached,
        } = req;
        let event = nu_value(event).map_err(EmitError::Render)?;
        let event_nuon = render_nuon(&event).map_err(EmitError::Render)?;
        let id = mint_msg_id(&self.nonce_gen, &from, &model, &event_nuon, attached.as_deref());
        let line = render_packet(id, &from, &model, &event, attached.as_deref())
            .map_err(EmitError::Render)?;
        self.emit(line).map_err(EmitError::Refused)?;
        Ok(id.to_string())
    }

    /// Apply a partial update and return what is now in force.
    pub(crate) fn set_thresholds(
        &mut self,
        update: ThresholdsRequest,
    ) -> Result<SpamThresholds, String> {
        let spam = self.spam.thresholds.updated(update)?;
        self.spam.thresholds = spam;
        Ok(spam)
    }

    /// A sender's stop of a spam offender, raised as the error's emergency.
    pub(crate) fn stopped(
        &mut self,
        stopped: ChannelSpamErrorEmergency,
    ) {
        self.up(UpKind::Emergency(Emergency::ChannelSpamError(stopped)));
    }

    /// A body's send up to its files: the checks, the render, and the id.
    pub(crate) fn plan_send(
        &mut self,
        data: ChannelSendData,
        now: Instant,
    ) -> Result<PlannedSend, SendVerdict> {
        let ChannelSendData {
            from,
            model,
            event,
            attached,
        } = data;
        if let Some(prefix) = reserved_prefix(&model) {
            return Err(SendVerdict::Refused(format!(
                "`{prefix}` is reserved for host-originated models; \
                 choose a model path outside it",
            )));
        }
        match self.phase {
            ChannelPhase::Closed => {
                return Err(refused(ChannelSendError::NotOpen));
            }
            ChannelPhase::Open => {
                self.close_now(CLOSE_UNVERIFIED_EMIT, UNVERIFIED_EMIT_REASON);
                return Err(refused(ChannelSendError::NotVerified));
            }
            ChannelPhase::Verified => {}
        }
        match self.spam.record_at(&from, now) {
            SpamVerdict::Clear => {}
            SpamVerdict::Warn { hits, window_secs } => {
                let rate = self.spam.thresholds.warn_rate;
                self.up(UpKind::Emergency(Emergency::ChannelSpamWarning(
                    ChannelSpamWarningEmergency {
                        origin: from.clone(),
                        hits,
                        window_secs,
                        rate,
                    },
                )));
            }
            SpamVerdict::Stop {
                hits,
                window_secs,
                notify,
            } => {
                let rate = self.spam.thresholds.error_rate;
                return Err(SendVerdict::Stop(SpamStop {
                    hits,
                    window_secs,
                    rate,
                    notify,
                    reason: format!(
                        "channel send rate exceeded: {hits} in {window_secs}s from {from} \
                         (limit {rate}); the channel carries STATE SUMMARIES, not data - \
                         put bulk in `attached`, or write a nuon file",
                    ),
                }));
            }
        }

        let event = nu_value(event).map_err(SendVerdict::Refused)?;
        let event_nuon = render_nuon(&event).map_err(SendVerdict::Refused)?;
        let attached_nuon = match attached {
            Some(value) => {
                let value = nu_value(value).map_err(SendVerdict::Refused)?;
                Some(render_nuon(&value).map_err(SendVerdict::Refused)?)
            }
            None => None,
        };
        let id = mint_msg_id(
            &self.nonce_gen,
            &from,
            &model,
            &event_nuon,
            attached_nuon.as_deref(),
        );

        let mut files = Vec::new();
        let attached_name = match attached_nuon {
            Some(nuon) => {
                let name = format!("{id}.nuon");
                files.push((self.inbox_path(&name)?, nuon));
                Some(name)
            }
            None => None,
        };
        let wire_event = if event_overflows(&event_nuon) {
            let name = format!("{id}.event.nuon");
            let pointer = event_spill_pointer(&name, event_nuon.len());
            files.push((self.inbox_path(&name)?, event_nuon));
            pointer
        } else {
            event
        };
        let line = render_packet(id, &from, &model, &wire_event, attached_name.as_deref())
            .map_err(SendVerdict::Refused)?;
        Ok(PlannedSend {
            id: id.to_string(),
            files,
            line,
        })
    }

    /// A planned send's emit, once its files are written.
    pub(crate) fn finish_send(
        &mut self,
        planned: PlannedSend,
    ) -> SendVerdict {
        match self.emit(planned.line) {
            Ok(()) => SendVerdict::Sent(planned.id),
            Err(e) => refused(e),
        }
    }

    /// Where an inbox file named `name` goes, or why there is no inbox.
    fn inbox_path(
        &self,
        name: &str,
    ) -> Result<PathBuf, SendVerdict> {
        match &self.inbox {
            Some(dir) => Ok(dir.join(name)),
            None => Err(SendVerdict::Refused(
                "the channel has no inbox; run `interface channel open` first".to_string(),
            )),
        }
    }
}

/// A send refused for the gate's reason.
fn refused(e: ChannelSendError) -> SendVerdict {
    SendVerdict::Refused(e.message().to_string())
}

/// Write each planned file, its directory made first, stopping at a failure.
pub(crate) fn write_files(files: Vec<(PathBuf, String)>) -> Result<(), String> {
    for (path, nuon) in files {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
        }
        fs::write(&path, nuon.as_bytes()).map_err(|e| format!("write {}: {e}", path.display()))?;
    }
    Ok(())
}
