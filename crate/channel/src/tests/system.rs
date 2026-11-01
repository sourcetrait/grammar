use crate::*;

/// How long a test waits on the system before calling it a hang.
const HANG: Duration = Duration::from_secs(5);

/// What the Channel sent its controller unasked, as the controller heard it.
struct Heard {
    ups: tokio::sync::mpsc::UnboundedSender<FromChannelSys>,
}

impl subsys::Handler<ChannelSystem> for Heard {
    fn on_packet(&mut self, pkt: subsys::Packet<FromChannelSys>) -> impl Future<Output = Option<subsys::Packet<FromChannelSys>>> + 'static + Send {
        let _ = self.ups.send(pkt.take_msg());
        std::future::ready(None)
    }
}

type HeardRx = tokio::sync::mpsc::UnboundedReceiver<FromChannelSys>;

/// A Channel on a permissive policy, and what its controller hears.
async fn start() -> (ChannelHandle, HeardRx) {
    let (ups, heard) = tokio::sync::mpsc::unbounded_channel();
    let home = std::env::temp_dir();
    let spam = SpamThresholdsData {
        warn_window_secs: 10,
        warn_rate: 1_000,
        error_window_secs: 10,
        error_rate: 10_000,
    };
    let handle = ChannelHandle::start(
        ChannelSysPaths::new(home.clone(), home),
        ChannelSysParams { spam },
        Heard { ups },
    )
    .await
    .expect("the Channel's system starts");
    (handle, heard)
}

/// A test's own inbox directory, empty.
fn inbox(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("grammar_channel_{test}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

async fn open(
    handle: &ChannelHandle,
    epoch: u64,
    mode: ChannelMode,
    inbox: &std::path::Path,
) {
    let greeting = matches!(mode, ChannelMode::Sync).then(|| "greeting".to_string());
    handle
        .request(OpenRequest {
            epoch: ChannelEpoch(epoch),
            mode,
            inbox: inbox.to_string_lossy().into_owned(),
            greeting,
        })
        .await
        .expect("open");
}

/// An async Channel open under epoch 1, claimed and verified.
async fn verified(handle: &ChannelHandle, inbox: &std::path::Path) {
    open(handle, 1, ChannelMode::Async, inbox).await;
    handle.notify(HubNotice::Claimed(ChannelEpoch(1))).await.expect("claim");
    let ready = handle.request(ReadyRequest).await.expect("ready");
    assert_eq!(ready, Ok(ChannelEpoch(1)));
}

fn send(event: NuVal) -> SendRequest {
    SendRequest(ChannelSendData {
        from: "thread/a".to_string(),
        model: "foo/Bar".to_string(),
        event,
        attached: None,
    })
}

fn small_event() -> NuVal {
    NuVal::Record(vec![("n".to_string(), NuVal::Int(1))])
}

#[tokio::test]
async fn a_bodys_line_goes_up_before_its_answer() {
    let (handle, mut heard) = start().await;
    verified(&handle, &inbox("line")).await;
    let SendVerdict::Sent(id) = handle.request(send(small_event())).await.expect("send") else {
        panic!("a verified channel takes the send");
    };
    match heard.try_recv() {
        Ok(FromChannelSys::Line(Line { epoch, line })) => {
            assert_eq!(epoch, ChannelEpoch(1));
            assert!(line.contains(&id), "the line carries the answered id");
        }
        other => panic!("the line is heard before the answer returns: {other:?}"),
    }
    handle.stop().await.expect("stop");
}

#[tokio::test]
async fn an_early_send_closes_the_channel_and_its_hub_hears_the_frame() {
    let (handle, mut heard) = start().await;
    open(&handle, 1, ChannelMode::Async, &inbox("early")).await;
    handle.notify(HubNotice::Claimed(ChannelEpoch(1))).await.expect("claim");
    let verdict = handle.request(send(small_event())).await.expect("send");
    assert_eq!(verdict, SendVerdict::Refused(ChannelSendError::NotVerified.message().to_string()));
    match heard.try_recv() {
        Ok(FromChannelSys::CloseHub(close)) => {
            assert_eq!((close.epoch, close.code), (ChannelEpoch(1), 1008));
            assert_eq!(close.reason, "emit before verification");
        }
        other => panic!("the hub hears its close frame: {other:?}"),
    }
    let status = handle.request(StatusRequest).await.expect("status");
    assert_eq!(status.state.phase, ChannelPhase::Closed);
    handle.stop().await.expect("stop");
}

#[tokio::test]
async fn a_sync_channel_holds_its_lines_for_the_sync() {
    let (handle, mut heard) = start().await;
    open(&handle, 1, ChannelMode::Sync, &inbox("sync")).await;
    let synced = handle.request(SyncRequest).await.expect("sync").expect("a sync channel");
    assert_eq!(synced.lines, ["greeting".to_string()], "the greeting comes first");
    assert_eq!(handle.request(ReadyRequest).await.expect("ready"), Ok(ChannelEpoch(1)));
    let SendVerdict::Sent(id) = handle.request(send(small_event())).await.expect("send") else {
        panic!("a verified channel takes the send");
    };
    let synced = handle.request(SyncRequest).await.expect("sync").expect("a sync channel");
    assert_eq!(synced.lines.len(), 1);
    assert!(synced.lines[0].contains(&id));
    assert!(heard.try_recv().is_err(), "a sync channel's lines never go up");
    handle.stop().await.expect("stop");
}

#[tokio::test]
async fn a_gone_peer_makes_the_channel_report_its_hub_gone() {
    let (handle, _heard) = start().await;
    verified(&handle, &inbox("gone")).await;
    handle.notify(HubNotice::PeerGone(ChannelEpoch(1))).await.expect("peer gone");
    let verdict = handle.request(send(small_event())).await.expect("send");
    assert_eq!(verdict, SendVerdict::Refused(ChannelSendError::HubGone.message().to_string()));
    let hosted = handle
        .request(HostRequest {
            line: "greeting".to_string(),
        })
        .await
        .expect("host");
    assert_eq!(hosted, Err(ChannelSendError::HubGone), "a re-greeting learns the same");
    handle.stop().await.expect("stop");
}

#[tokio::test]
async fn an_expiry_closes_only_an_unverified_open_of_its_epoch() {
    let (handle, mut heard) = start().await;
    open(&handle, 2, ChannelMode::Async, &inbox("expire")).await;
    let expire = |epoch| ExpireRequest {
        epoch: ChannelEpoch(epoch),
        code: 1008,
        reason: "verification window expired".to_string(),
    };
    let stale = handle.request(expire(1)).await.expect("expire");
    assert!(!stale.closed, "an earlier open's timer closes nothing");
    let due = handle.request(expire(2)).await.expect("expire");
    assert!(due.closed);
    assert_eq!(due.inbox, None, "an expiry leaves the inbox readable");
    match heard.try_recv() {
        Ok(FromChannelSys::CloseHub(close)) => assert_eq!(close.epoch, ChannelEpoch(2)),
        other => panic!("the hub hears its close frame: {other:?}"),
    }
    handle.stop().await.expect("stop");
}

#[tokio::test]
async fn the_spam_emergencies_go_up_to_the_controller() {
    let (handle, mut heard) = start().await;
    let set = handle
        .request(ThresholdsRequest {
            warn_rate: Some(2),
            error_rate: Some(3),
            ..Default::default()
        })
        .await
        .expect("thresholds")
        .expect("a valid update");
    assert_eq!((set.warn_rate, set.error_rate), (2, 3), "the update answers what is in force");
    verified(&handle, &inbox("spam")).await;
    for _ in 0..2 {
        assert!(matches!(handle.request(send(small_event())).await, Ok(SendVerdict::Sent(_))));
    }
    let mut warned = false;
    while let Ok(up) = heard.try_recv() {
        warned |= matches!(up, FromChannelSys::Emergency(Emergency::ChannelSpamWarning(_)));
    }
    assert!(warned, "the soft threshold goes up as a warning");
    let SendVerdict::Stop(stop) = handle.request(send(small_event())).await.expect("send") else {
        panic!("the hard threshold stops the sender");
    };
    assert!(stop.notify);
    handle
        .notify(StoppedNotice(ChannelSpamErrorEmergency {
            origin: "thread/a".to_string(),
            hits: stop.hits,
            window_secs: stop.window_secs,
            rate: stop.rate,
            action: "signals triggered".to_string(),
        }))
        .await
        .expect("stopped");
    let status = handle.request(StatusRequest).await.expect("status");
    assert_eq!(status.state.phase, ChannelPhase::Verified, "a stop refuses one sender only");
    match heard.try_recv() {
        Ok(FromChannelSys::Emergency(Emergency::ChannelSpamError(e))) => {
            assert_eq!(e.action, "signals triggered");
        }
        other => panic!("the stop goes up as the error: {other:?}"),
    }
    handle.stop().await.expect("stop");
}

#[tokio::test]
async fn a_spilled_send_writes_its_files_before_its_line_goes_up() {
    let (handle, mut heard) = start().await;
    let dir = inbox("spill");
    verified(&handle, &dir).await;
    let big = NuVal::Record(vec![("text".to_string(), NuVal::String("x".repeat(1_000)))]);
    let req = SendRequest(ChannelSendData {
        from: "thread/a".to_string(),
        model: "foo/Bar".to_string(),
        event: big,
        attached: Some(small_event()),
    });
    let SendVerdict::Sent(id) = handle.request(req).await.expect("send") else {
        panic!("a verified channel takes the send");
    };
    let attached = fs::read_to_string(dir.join(format!("{id}.nuon"))).expect("the attachment");
    assert_eq!(attached, "{n: 1}");
    let spilled = fs::read_to_string(dir.join(format!("{id}.event.nuon"))).expect("the spill");
    assert!(spilled.contains("xxxx"), "the full event sits in its spill");
    assert!(matches!(heard.try_recv(), Ok(FromChannelSys::Line(_))));
    let _ = fs::remove_dir_all(&dir);
    handle.stop().await.expect("stop");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_thread_outside_the_runtime_sends_and_notifies() {
    let (handle, mut heard) = start().await;
    verified(&handle, &inbox("thread")).await;
    let handle = std::sync::Arc::new(handle);
    let (answer, answered) = tokio::sync::oneshot::channel();
    let asker = handle.clone();
    std::thread::spawn(move || {
        let verdict = asker.request_blocking(send(small_event()));
        let notice = asker.notify_blocking(HubNotice::PeerGone(ChannelEpoch(1)));
        drop(asker);
        let _ = answer.send((verdict, notice));
    });
    let (verdict, notice) = tokio::time::timeout(HANG, answered)
        .await
        .expect("the thread's asks end rather than hanging")
        .expect("answered");
    assert!(matches!(verdict, Ok(SendVerdict::Sent(_))), "{verdict:?}");
    assert_eq!(notice, Ok(()));
    assert!(matches!(heard.recv().await, Some(FromChannelSys::Line(_))));
    let hosted = handle
        .request(HostRequest {
            line: "x".to_string(),
        })
        .await
        .expect("host");
    assert_eq!(hosted, Err(ChannelSendError::HubGone), "the thread's notice landed");
    let handle = std::sync::Arc::into_inner(handle).expect("the thread let go of the handle");
    handle.stop().await.expect("stop");
}
