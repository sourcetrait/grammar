use crate::*;
use crate::state::Up;

/// A permissive policy, so a test not about the thresholds never trips them.
fn test_policy() -> SpamThresholds {
    SpamThresholds {
        warn_window: Duration::from_secs(10),
        warn_rate: 1_000,
        error_window: Duration::from_secs(10),
        error_rate: 10_000,
    }
}

const EPOCH: ChannelEpoch = ChannelEpoch(1);

fn open_request(mode: ChannelMode) -> OpenRequest {
    OpenRequest {
        epoch: EPOCH,
        mode,
        inbox: "/tmp/grammar.channel.x".to_string(),
        greeting: matches!(mode, ChannelMode::Sync).then(|| "greeting".to_string()),
    }
}

/// An async Channel open under `EPOCH`, nothing claiming it yet.
fn open_async() -> Channel {
    let mut channel = Channel::new(test_policy());
    channel.open(open_request(ChannelMode::Async));
    channel
}

/// The ordinary case: a peer has connected and claimed.
fn claimed_async() -> Channel {
    let mut channel = open_async();
    channel.hub_notice(HubNotice::Claimed(EPOCH));
    channel
}

fn verified_async() -> Channel {
    let mut channel = claimed_async();
    assert_eq!(channel.mark_verified(), Ok(EPOCH));
    channel
}

fn line(text: &str) -> Up {
    Up {
        epoch: EPOCH,
        kind: UpKind::Line(text.to_string()),
    }
}

fn close_hub(code: u16, reason: &str) -> Up {
    Up {
        epoch: EPOCH,
        kind: UpKind::CloseHub {
            code,
            reason: reason.to_string(),
        },
    }
}

#[test]
fn a_fresh_channel_is_closed() {
    let status = Channel::new(test_policy()).status();
    assert_eq!(status.state.phase, ChannelPhase::Closed);
    assert_eq!(status.state.mode, None);
    assert!(!status.state.claimed);
    assert!(status.inbox.is_none());
}

#[test]
fn an_open_records_its_mode_and_inbox_and_a_notice_brings_the_claim() {
    let mut channel = open_async();
    let status = channel.status();
    assert_eq!(status.state.phase, ChannelPhase::Open);
    assert_eq!(status.state.mode, Some(ChannelMode::Async));
    assert_eq!(status.state.epoch, EPOCH);
    assert_eq!(status.inbox.as_deref(), Some("/tmp/grammar.channel.x"));
    assert!(!status.state.claimed, "nothing has connected yet");
    channel.hub_notice(HubNotice::Claimed(EPOCH));
    assert!(channel.status().state.claimed, "the hub's claim reaches the state");
}

#[test]
fn a_notice_for_an_earlier_open_changes_nothing() {
    let mut channel = open_async();
    channel.hub_notice(HubNotice::Claimed(ChannelEpoch(0)));
    assert!(!channel.status().state.claimed, "a stale claim is not this open's");
    channel.hub_notice(HubNotice::Claimed(EPOCH));
    assert_eq!(channel.mark_verified(), Ok(EPOCH));
    channel.hub_notice(HubNotice::PeerGone(ChannelEpoch(0)));
    assert_eq!(channel.emit("x".to_string()), Ok(()), "a stale peer's end leaves the hub");
}

#[test]
fn verification_requires_a_claim() {
    assert_eq!(
        Channel::new(test_policy()).mark_verified(),
        Err(ChannelVerifyError::NotOpen),
    );
    let mut channel = open_async();
    assert_eq!(
        channel.mark_verified(),
        Err(ChannelVerifyError::NotClaimed(ChannelMode::Async)),
        "verifying with nothing connected has no claim to prove ownership of, and \
         would leave a verified channel whose queue nothing drains",
    );
    assert_eq!(channel.status().state.phase, ChannelPhase::Open);
    channel.hub_notice(HubNotice::Claimed(EPOCH));
    assert_eq!(channel.mark_verified(), Ok(EPOCH));
    assert_eq!(channel.status().state.phase, ChannelPhase::Verified);
}

#[test]
fn emit_is_refused_until_verified_and_the_reasons_are_distinguishable() {
    assert_eq!(
        Channel::new(test_policy()).emit("x".to_string()),
        Err(ChannelSendError::NotOpen),
        "no channel at all",
    );
    let mut channel = claimed_async();
    assert_eq!(
        channel.emit("x".to_string()),
        Err(ChannelSendError::NotVerified),
        "connected but unproven: telemetry must not flow",
    );
    assert_eq!(channel.mark_verified(), Ok(EPOCH));
    assert_eq!(channel.emit("x".to_string()), Ok(()));
    assert_eq!(channel.take_ups(), vec![line("x")], "an async line goes up for the hub");
}

#[test]
fn a_control_packet_bypasses_the_verification_gate() {
    let mut channel = open_async();
    assert_eq!(channel.send_control("greeting".to_string()), Ok(()));
    assert_eq!(channel.take_ups(), vec![line("greeting")]);
    assert_eq!(
        Channel::new(test_policy()).send_control("x".to_string()),
        Err(ChannelSendError::NotOpen),
        "a closed channel still refuses control packets",
    );
}

/// A sync channel's handshake is async's: its greeting first, the take its claim.
#[test]
fn a_sync_channel_queues_its_greeting_and_a_sync_claims_it() {
    let mut channel = Channel::new(test_policy());
    channel.open(open_request(ChannelMode::Sync));
    let status = channel.status();
    assert_eq!(status.state.phase, ChannelPhase::Open);
    assert_eq!(status.state.mode, Some(ChannelMode::Sync));
    assert!(!status.state.claimed);
    assert_eq!(
        channel.mark_verified(),
        Err(ChannelVerifyError::NotClaimed(ChannelMode::Sync)),
    );
    assert_eq!(channel.emit("early".to_string()), Err(ChannelSendError::NotVerified));
    let synced = channel.take_sync().expect("a sync channel");
    assert_eq!((synced.lines, synced.dropped), (vec!["greeting".to_string()], 0));
    assert!(channel.status().state.claimed, "taking the greeting claims the channel");
    assert_eq!(channel.mark_verified(), Ok(EPOCH));
    assert_eq!(channel.emit("a".to_string()), Ok(()));
    assert_eq!(channel.send_control("b".to_string()), Ok(()));
    let synced = channel.take_sync().expect("a sync channel");
    assert_eq!(synced.lines, vec!["a".to_string(), "b".to_string()]);
    assert!(channel.take_ups().is_empty(), "a sync line waits here, never going up");
    let synced = channel.take_sync().expect("a sync channel");
    assert!(synced.lines.is_empty(), "a sync empties the queue");
}

#[test]
fn a_sync_queue_drops_its_oldest_past_the_cap() {
    let mut channel = Channel::new(test_policy());
    channel.open(open_request(ChannelMode::Sync));
    assert!(channel.take_sync().is_ok());
    assert_eq!(channel.mark_verified(), Ok(EPOCH));
    for n in 0..SYNC_QUEUE_CAP + 2 {
        assert_eq!(channel.emit(n.to_string()), Ok(()));
    }
    assert_eq!(channel.status().state.sync_dropped, 2, "the state counts the drops");
    let synced = channel.take_sync().expect("a sync channel");
    assert_eq!(synced.lines.len(), SYNC_QUEUE_CAP);
    assert_eq!(synced.lines.first().map(String::as_str), Some("2"), "the oldest two are gone");
    assert_eq!(synced.lines.last().map(String::as_str), Some("1001"));
    assert_eq!(synced.dropped, 2);
    let synced = channel.take_sync().expect("a sync channel");
    assert_eq!(synced.dropped, 0, "the count is since the last sync");
}

#[test]
fn a_sync_is_refused_on_a_closed_or_an_async_channel() {
    assert_eq!(
        Channel::new(test_policy()).take_sync().map(|s| s.lines),
        Err(ChannelSyncError::NotOpen),
    );
    let mut channel = open_async();
    assert_eq!(
        channel.take_sync().map(|s| s.lines),
        Err(ChannelSyncError::WrongMode(ChannelMode::Async)),
    );
    assert!(!channel.status().state.claimed, "a refused sync claims nothing");
}

#[test]
fn a_close_drops_a_sync_channel_and_its_queue() {
    let mut channel = Channel::new(test_policy());
    channel.open(open_request(ChannelMode::Sync));
    assert!(channel.close(1000, "channel closed"));
    let status = channel.status();
    assert_eq!(status.state.phase, ChannelPhase::Closed);
    assert_eq!(status.state.mode, None);
    assert_eq!(channel.take_sync().map(|s| s.lines), Err(ChannelSyncError::NotOpen));
    assert!(channel.take_ups().is_empty(), "a sync channel has no hub to close");
}

#[test]
fn a_gone_peer_is_reported_as_hub_gone() {
    let mut channel = verified_async();
    channel.hub_notice(HubNotice::PeerGone(EPOCH));
    assert_eq!(channel.emit("x".to_string()), Err(ChannelSendError::HubGone));
    assert_eq!(
        channel.send_control("x".to_string()),
        Err(ChannelSendError::HubGone),
        "a control send detects the gone peer the same way - that is how channel_open \
         learns the connection died",
    );
    assert_eq!(
        channel.status().state.phase,
        ChannelPhase::Verified,
        "the logical Channel stays open until the agent closes it",
    );
}

#[test]
fn an_async_close_sends_its_hub_the_frame_after_what_was_queued() {
    let mut channel = open_async();
    assert_eq!(channel.send_control("queued".to_string()), Ok(()));
    assert!(channel.close(1000, "done"));
    assert_eq!(
        channel.take_ups(),
        vec![line("queued"), close_hub(1000, "done")],
        "the close does not disturb what was already queued, and carries its code",
    );
    assert_eq!(channel.status().state.phase, ChannelPhase::Closed);
    assert!(!channel.close(1000, "again"), "closing twice is a no-op");
    assert!(channel.take_ups().is_empty());
}

#[test]
fn a_gone_peers_channel_still_closes_its_hub() {
    let mut channel = verified_async();
    channel.hub_notice(HubNotice::PeerGone(EPOCH));
    assert!(channel.close(1000, "channel closed"));
    assert_eq!(channel.take_ups(), vec![close_hub(1000, "channel closed")]);
}

#[test]
fn close_if_unverified_only_fires_while_unproven() {
    let mut channel = open_async();
    assert!(
        channel.close_if_unverified(EPOCH, 1008, "expired"),
        "an open, unverified channel is exactly what the timer tears down",
    );
    assert_eq!(channel.status().state.phase, ChannelPhase::Closed);
    assert_eq!(channel.take_ups(), vec![close_hub(1008, "expired")]);

    let mut channel = verified_async();
    assert!(
        !channel.close_if_unverified(EPOCH, 1008, "expired"),
        "verification can land between the sleep elapsing and the expiry arriving; the \
         re-check is what stops the timer killing a proven channel",
    );
    assert_eq!(channel.status().state.phase, ChannelPhase::Verified);
}

#[test]
fn an_expiry_for_an_earlier_open_closes_nothing() {
    let mut channel = open_async();
    assert!(
        !channel.close_if_unverified(ChannelEpoch(0), 1008, "expired"),
        "a timer armed for an earlier open must not close this one",
    );
    assert_eq!(channel.status().state.phase, ChannelPhase::Open);
}

#[test]
fn a_deliberate_close_takes_the_inbox_to_prune_it() {
    let mut channel = Channel::new(test_policy());
    assert!(
        channel.take_inbox().is_none(),
        "a channel that never opened wrote nothing, so there is nothing to prune",
    );
    channel.open(open_request(ChannelMode::Async));
    let kept = channel.close_request(CloseRequest {
        code: 1008,
        reason: "expired".to_string(),
        prune: false,
    });
    assert!(kept.closed);
    assert_eq!(kept.inbox, None, "a host-initiated close leaves the inbox readable");
    let pruned = channel.close_request(CloseRequest {
        code: 1000,
        reason: "channel closed".to_string(),
        prune: true,
    });
    assert!(!pruned.closed, "already closed");
    assert_eq!(
        pruned.inbox.as_deref(),
        Some("/tmp/grammar.channel.x"),
        "the deliberate-close path takes the path so it can prune it",
    );
    assert!(
        channel.take_inbox().is_none(),
        "taking clears it, so a second close cannot prune a dir a later open recreated",
    );
}

#[test]
fn thresholds_update_partially_and_reject_zero() {
    let mut channel = Channel::new(test_policy());
    let updated = channel
        .set_thresholds(ThresholdsRequest {
            warn_rate: Some(7),
            ..Default::default()
        })
        .expect("partial update");
    assert_eq!(updated.warn_rate, 7, "the supplied field moves");
    assert_eq!(updated.error_rate, 10_000, "and an omitted one does not");
    assert_eq!(updated.warn_window.as_secs(), 10);
    assert!(
        channel
            .set_thresholds(ThresholdsRequest {
                warn_rate: Some(0),
                ..Default::default()
            })
            .is_err(),
        "a zero rate would forbid the first send rather than police a rate",
    );
    assert!(
        channel
            .set_thresholds(ThresholdsRequest {
                warn_window_secs: Some(0),
                warn_rate: Some(9),
                ..Default::default()
            })
            .is_err(),
        "a zero window is not a rate at all, and the update applies whole or not at all",
    );
    let read = channel.set_thresholds(ThresholdsRequest::default()).expect("a read");
    assert_eq!(read.warn_rate, 7, "a rejected update must not have partially applied");
}

/// A body's send of `event` from `from`, nothing attached.
fn send_data(from: &str, model: &str, event: NuVal) -> ChannelSendData {
    ChannelSendData {
        from: from.to_string(),
        model: model.to_string(),
        event,
        attached: None,
    }
}

fn small_event() -> NuVal {
    NuVal::Record(vec![("n".to_string(), NuVal::Int(1))])
}

#[test]
fn a_send_refuses_a_reserved_model_before_reading_the_channel() {
    let mut channel = Channel::new(test_policy());
    for model in ["mcp/channel/Open", "remote/channel/x"] {
        match channel.plan_send(send_data("thread/a", model, small_event()), Instant::now()) {
            Err(SendVerdict::Refused(reason)) => assert!(reason.contains("reserved"), "{reason}"),
            _ => panic!("`{model}` lies under a reservation"),
        }
    }
}

#[test]
fn an_early_send_closes_an_open_but_unverified_channel() {
    let mut channel = claimed_async();
    match channel.plan_send(send_data("thread/a", "foo/Bar", small_event()), Instant::now()) {
        Err(SendVerdict::Refused(reason)) => {
            assert_eq!(reason, ChannelSendError::NotVerified.message());
        }
        _ => panic!("an unverified channel refuses a body's send"),
    }
    assert_eq!(
        channel.status().state.phase,
        ChannelPhase::Closed,
        "a peer emitted to unproven is not a peer we keep",
    );
    assert_eq!(channel.take_ups(), vec![close_hub(1008, "emit before verification")]);
}

#[test]
fn a_send_on_a_verified_channel_goes_up_under_its_id() {
    let mut channel = verified_async();
    let planned = channel
        .plan_send(send_data("thread/a", "foo/Bar", small_event()), Instant::now())
        .unwrap_or_else(|_| panic!("a verified channel takes a body's send"));
    assert!(planned.files.is_empty(), "a small event without attachment writes nothing");
    let id = planned.id.clone();
    let SendVerdict::Sent(sent) = channel.finish_send(planned) else {
        panic!("the planned send emits");
    };
    assert_eq!(sent, id);
    let ups = channel.take_ups();
    let [Up { kind: UpKind::Line(line), .. }] = ups.as_slice() else {
        panic!("one line goes up: {ups:?}");
    };
    let value = nuon::from_nuon(line, None).expect("NUON");
    let record = value.as_record().expect("record");
    assert_eq!(record.get("id").and_then(|v| v.as_str().ok()), Some(id.as_str()));
    assert_eq!(record.get("from").and_then(|v| v.as_str().ok()), Some("thread/a"));
}

#[test]
fn an_oversized_event_spills_and_an_attachment_rides_by_name() {
    let mut channel = verified_async();
    let big = NuVal::Record(vec![("text".to_string(), NuVal::String("x".repeat(1_000)))]);
    let data = ChannelSendData {
        attached: Some(small_event()),
        ..send_data("thread/a", "foo/Bar", big)
    };
    let planned = channel
        .plan_send(data, Instant::now())
        .unwrap_or_else(|_| panic!("a verified channel takes a body's send"));
    let names: Vec<String> = planned
        .files
        .iter()
        .map(|(path, _)| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        names,
        [format!("{}.nuon", planned.id), format!("{}.event.nuon", planned.id)],
        "the attachment, then the spill, each named by the packet's id",
    );
    let line = nuon::from_nuon(&planned.line, None).expect("NUON");
    let record = line.as_record().expect("record");
    let expected_attached = format!("{}.nuon", planned.id);
    assert_eq!(
        record.get("attached").and_then(|v| v.as_str().ok()),
        Some(expected_attached.as_str()),
    );
    let event = record.get("event").and_then(|v| v.as_record().ok()).expect("event");
    let expected_spill = format!("{}.event.nuon", planned.id);
    assert_eq!(
        event.get(EVENT_SPILL_KEY).and_then(|v| v.as_str().ok()),
        Some(expected_spill.as_str()),
        "the pointer rides the wire in the event's place",
    );
}

#[test]
fn the_send_path_warns_once_and_then_stops() {
    let mut channel = verified_async();
    let _ = channel.set_thresholds(ThresholdsRequest {
        warn_rate: Some(2),
        error_rate: Some(3),
        ..Default::default()
    });
    let now = Instant::now();
    let send = |channel: &mut Channel| {
        channel.plan_send(send_data("thread/a", "foo/Bar", small_event()), now)
    };
    assert!(send(&mut channel).is_ok());
    assert!(send(&mut channel).is_ok(), "the warning still lets the send through");
    let ups = channel.take_ups();
    assert!(
        ups.iter().any(|up| matches!(
            &up.kind,
            UpKind::Emergency(Emergency::ChannelSpamWarning(w)) if w.origin == "thread/a" && w.rate == 2
        )),
        "the soft threshold goes up as a warning: {ups:?}",
    );
    match send(&mut channel) {
        Err(SendVerdict::Stop(stop)) => {
            assert_eq!((stop.hits, stop.rate, stop.notify), (3, 3, true));
            assert!(stop.reason.contains("rate exceeded"), "{}", stop.reason);
        }
        _ => panic!("the hard threshold stops the sender"),
    }
    channel.stopped(ChannelSpamErrorEmergency {
        origin: "thread/a".to_string(),
        hits: 3,
        window_secs: 10,
        rate: 3,
        action: "signals triggered".to_string(),
    });
    assert!(
        matches!(
            channel.take_ups().as_slice(),
            [Up { kind: UpKind::Emergency(Emergency::ChannelSpamError(e)), .. }] if e.action == "signals triggered"
        ),
        "the sender's stop goes up as the error, its action with it",
    );
}

#[test]
fn a_host_packet_through_the_gate_is_rendered_and_minted_here() {
    let mut channel = open_async();
    let req = || EmitRequest {
        from: "mcp".to_string(),
        model: "mcp/supervisor/CpuWarning".to_string(),
        event: small_event(),
        attached: None,
    };
    assert_eq!(
        channel.emit_packet(req()),
        Err(EmitError::Refused(ChannelSendError::NotVerified)),
        "a host announcement waits on verification like any emit",
    );
    channel.hub_notice(HubNotice::Claimed(EPOCH));
    assert_eq!(channel.mark_verified(), Ok(EPOCH));
    let id = channel.emit_packet(req()).expect("emitted");
    let ups = channel.take_ups();
    let [Up { kind: UpKind::Line(line), .. }] = ups.as_slice() else {
        panic!("one line goes up: {ups:?}");
    };
    assert!(line.contains(&id), "the line carries its minted id");
}

#[test]
fn the_planned_files_land_in_the_inbox() {
    let dir = std::env::temp_dir().join(format!("grammar_channel_files_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let files = vec![
        (dir.join("a.nuon"), "{a: 1}".to_string()),
        (dir.join("b.event.nuon"), "{b: 2}".to_string()),
    ];
    write_files(files).expect("written");
    assert_eq!(fs::read_to_string(dir.join("a.nuon")).ok().as_deref(), Some("{a: 1}"));
    assert_eq!(fs::read_to_string(dir.join("b.event.nuon")).ok().as_deref(), Some("{b: 2}"));
    let _ = fs::remove_dir_all(&dir);
}
