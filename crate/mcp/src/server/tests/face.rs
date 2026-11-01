//! The face's hub slot: its verify timer, hub lanes, shutdown ack, and policy.
use crate::server::channel::face::HubSlot;
use crate::*;
use sourcetrait_grammar_protocol::ChannelSpamWarningEmergency;

const EPOCH: ChannelEpoch = ChannelEpoch(1);

/// A hub-shaped slot entry without a hub: its senders, the receivers held here.
fn installed(
    slot: &HubSlot,
    epoch: ChannelEpoch,
) -> (
    tk::UnboundedReceiver<String>,
    tk::oneshot::Receiver<ChannelCloseSignal>,
    tk::oneshot::Receiver<()>,
) {
    let (packets, packet_rx) = tk::unbounded_channel::<String>();
    let (close, close_rx) = tk::oneshot::channel::<ChannelCloseSignal>();
    let (shutdown, shutdown_rx) = tk::oneshot::channel::<()>();
    let hub = Hub {
        epoch,
        url: "wss://127.0.0.1:1".to_string(),
        packets,
        close,
        _shutdown: shutdown,
    };
    assert!(slot.install(hub).is_ok(), "an open epoch's hub is installed");
    (packet_rx, close_rx, shutdown_rx)
}

#[test]
fn each_open_takes_a_fresh_epoch() {
    let slot = HubSlot::default();
    let first = slot.next_epoch();
    let second = slot.next_epoch();
    assert_ne!(first, second);
    assert_ne!(first, ChannelEpoch(0), "the Channel's own closed epoch is never minted");
}

#[test]
fn a_line_reaches_only_the_hub_of_its_open() {
    let slot = HubSlot::default();
    let (mut packets, _close, _shutdown) = installed(&slot, EPOCH);
    slot.deliver(ChannelEpoch(9), "stale".to_string());
    slot.deliver(EPOCH, "x".to_string());
    assert_eq!(packets.try_recv().ok().as_deref(), Some("x"), "the stale line was dropped");
    assert!(packets.try_recv().is_err());
    assert_eq!(slot.url(EPOCH).as_deref(), Some("wss://127.0.0.1:1"));
    assert_eq!(slot.url(ChannelEpoch(9)), None);
}

#[test]
fn a_gone_peer_shows_as_its_lane_closing() {
    let slot = HubSlot::default();
    let (packets, _close, _shutdown) = installed(&slot, EPOCH);
    assert!(!slot.peer_gone(EPOCH));
    drop(packets);
    assert!(
        slot.peer_gone(EPOCH),
        "the peer task drops its lane when it ends - that is how channel_open learns",
    );
    assert!(slot.peer_gone(ChannelEpoch(9)), "an open with no hub here has no peer");
}

#[test]
fn a_hub_close_signals_on_its_own_lane_and_ends_the_accept_loop() {
    let slot = HubSlot::default();
    let (_packets, mut close, mut shutdown) = installed(&slot, EPOCH);
    slot.close(ChannelEpoch(9), 1000, "stale".to_string());
    assert!(close.try_recv().is_err(), "a close for another open leaves this hub");
    slot.close(EPOCH, 1000, "done".to_string());
    let (code, reason, done) = close.try_recv().expect("the close rides its own lane");
    assert_eq!(
        (code, reason.as_str()),
        (1000, "done"),
        "a planned close must never wait behind traffic, or the agent sees a bare 1006",
    );
    assert!(done.is_none(), "a plain close asks for no flush ack");
    assert!(
        matches!(shutdown.try_recv(), Err(tk::oneshot::error::TryRecvError::Closed)),
        "the hub's shutdown sender dropped with it, which ends the accept loop",
    );
    assert_eq!(slot.url(EPOCH), None, "the slot is empty after");
}

#[test]
fn a_hub_starting_after_its_open_closed_is_refused() {
    let slot = HubSlot::default();
    let epoch = slot.next_epoch();
    slot.close(epoch, 1000, "closed while its hub started".to_string());
    let (packets, _packet_rx) = tk::unbounded_channel::<String>();
    let (close, _close_rx) = tk::oneshot::channel::<ChannelCloseSignal>();
    let (shutdown, mut shutdown_rx) = tk::oneshot::channel::<()>();
    let late = Hub {
        epoch,
        url: "wss://127.0.0.1:1".to_string(),
        packets,
        close,
        _shutdown: shutdown,
    };
    let refused = slot.install(late);
    assert!(refused.is_err(), "the closed open's hub is handed back");
    drop(refused);
    assert!(
        matches!(shutdown_rx.try_recv(), Err(tk::oneshot::error::TryRecvError::Closed)),
        "dropping the refused hub ends its accept loop",
    );
    assert_eq!(slot.url(epoch), None, "no hub serves the closed open");
    let next = slot.next_epoch();
    let (_packets, _close, _shutdown) = installed(&slot, next);
    assert!(slot.url(next).is_some(), "a later open's hub still installs");
}

#[test]
fn a_shutdown_close_hands_back_a_flush_ack() {
    let slot = HubSlot::default();
    let (_packets, mut close, _shutdown) = installed(&slot, EPOCH);
    let mut done_rx = slot
        .close_now(1001, "host shutting down")
        .expect("an installed hub has something to close");
    let (code, reason, done_tx) = close.try_recv().expect("the close was signalled");
    assert_eq!((code, reason.as_str()), (1001, "host shutting down"));
    done_tx
        .expect("a shutdown close carries an ack sender")
        .send(())
        .expect("ack");
    assert!(done_rx.try_recv().is_ok(), "the waiter observes the flush");
    assert!(
        slot.close_now(1001, "host shutting down").is_none(),
        "no hub means no frame to wait for, so shutdown must not block on one",
    );
}

#[test]
fn arming_the_timer_again_stands_the_previous_one_down() {
    let slot = HubSlot::default();
    let (first, mut first_rx) = tk::oneshot::channel::<()>();
    slot.arm_verify(EPOCH, first);
    assert!(
        matches!(first_rx.try_recv(), Err(tk::oneshot::error::TryRecvError::Empty)),
        "still armed while the slot holds the sender",
    );
    let (second, _second_rx) = tk::oneshot::channel::<()>();
    slot.arm_verify(ChannelEpoch(2), second);
    assert!(
        matches!(first_rx.try_recv(), Err(tk::oneshot::error::TryRecvError::Closed)),
        "re-opening drops the previous sender, so two timers never race one channel",
    );
}

#[test]
fn verification_and_a_hub_close_stand_the_timer_down_for_their_open_alone() {
    let slot = HubSlot::default();
    let (cancel, mut cancel_rx) = tk::oneshot::channel::<()>();
    slot.arm_verify(EPOCH, cancel);
    slot.disarm_verify(ChannelEpoch(9));
    assert!(
        matches!(cancel_rx.try_recv(), Err(tk::oneshot::error::TryRecvError::Empty)),
        "another open's verification leaves this timer armed",
    );
    slot.disarm_verify(EPOCH);
    assert!(matches!(
        cancel_rx.try_recv(),
        Err(tk::oneshot::error::TryRecvError::Closed),
    ));

    let (_packets, _close, _shutdown) = installed(&slot, EPOCH);
    let (cancel, mut cancel_rx) = tk::oneshot::channel::<()>();
    slot.arm_verify(EPOCH, cancel);
    slot.close(EPOCH, 1000, "done".to_string());
    assert!(matches!(
        cancel_rx.try_recv(),
        Err(tk::oneshot::error::TryRecvError::Closed),
    ));
}

/// A Channel's controller that hears nothing, for a policy to announce through.
struct Deaf;

impl subsys::Handler<ChannelSystem> for Deaf {
    fn on_packet(&mut self, _pkt: subsys::Packet<FromChannelSys>) -> impl Future<Output = Option<subsys::Packet<FromChannelSys>>> + 'static + Send {
        std::future::ready(None)
    }
}

/// The log's lines once one is there, or what it holds when `HANG` has passed.
async fn logged(log: &Path) -> String {
    let deadline = Instant::now() + HANG;
    loop {
        let lines = fs::read_to_string(log).unwrap_or_default();
        if !lines.is_empty() || Instant::now() >= deadline {
            return lines;
        }
        tk::sleep(tk::TkDuration::from_millis(10)).await;
    }
}

/// How long a test waits on the policy's task before calling it a hang.
const HANG: tk::TkDuration = tk::TkDuration::from_secs(5);

#[test]
fn an_emergency_reaches_the_policy_once_one_is_installed() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a tokio runtime");
    let home = std::env::temp_dir().join(format!("grammar_mcp_face_policy_{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let log = home.join("emergency.nuonl");
    let warning = Emergency::ChannelSpamWarning(ChannelSpamWarningEmergency {
        origin: "thread/a".to_string(),
        hits: 3,
        window_secs: 10,
        rate: 3,
    });
    let lines = rt.block_on(async {
        let spam = SpamThresholdsData {
            warn_window_secs: 10,
            warn_rate: 1_000,
            error_window_secs: 10,
            error_rate: 10_000,
        };
        let paths = ChannelSysPaths::new(home.clone(), home.clone());
        let channel = ChannelHandle::start(paths, ChannelSysParams { spam }, Deaf)
            .await
            .expect("the Channel's system starts");
        let slot = HubSlot::default();
        slot.raise(warning.clone());
        slot.install_policy(EmergencyPolicy::new(Arc::new(channel), "nomAAA", log.clone()));
        slot.raise(warning);
        logged(&log).await
    });
    let _ = fs::remove_dir_all(&home);
    assert_eq!(lines.lines().count(), 1, "the one raised before the install went nowhere: {lines}");
    assert!(lines.contains("channel_spam_warning"), "the line is the warning's: {lines}");
}

/// How many emergencies the concurrency test raises at once.
const RAISED: u32 = 64;

#[test]
fn emergencies_raised_at_once_log_whole_records() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .expect("a tokio runtime");
    let home = std::env::temp_dir().join(format!("grammar_mcp_face_records_{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let log = home.join("emergency.nuonl");
    rt.block_on(async {
        let spam = SpamThresholdsData {
            warn_window_secs: 10,
            warn_rate: 1_000,
            error_window_secs: 10,
            error_rate: 10_000,
        };
        let paths = ChannelSysPaths::new(home.clone(), home.clone());
        let channel = ChannelHandle::start(paths, ChannelSysParams { spam }, Deaf)
            .await
            .expect("the Channel's system starts");
        let policy = EmergencyPolicy::new(Arc::new(channel), "nomAAA", log.clone());
        let raised: Vec<_> = (0..RAISED)
            .map(|hits| {
                let policy = policy.clone();
                let warning = Emergency::ChannelSpamWarning(ChannelSpamWarningEmergency {
                    origin: "thread/a".to_string(),
                    hits,
                    window_secs: 10,
                    rate: 3,
                });
                tk::spawn(async move { policy.respond(warning).await })
            })
            .collect();
        for task in raised {
            task.await.expect("a policy task ends");
        }
    });
    let text = fs::read_to_string(&log).unwrap_or_default();
    let _ = fs::remove_dir_all(&home);
    let mut hits: Vec<i64> = text
        .lines()
        .map(|line| {
            let record = nu::from_nuon(line, None)
                .unwrap_or_else(|e| panic!("each line is one whole record: {line:?}: {e}"));
            record
                .get_data_by_key("hits")
                .and_then(|hits| hits.as_int().ok())
                .unwrap_or_else(|| panic!("a record carries its hits: {line:?}"))
        })
        .collect();
    hits.sort_unstable();
    let expected: Vec<i64> = (0..i64::from(RAISED)).collect();
    assert_eq!(hits, expected, "every emergency logged once, its record whole");
}
