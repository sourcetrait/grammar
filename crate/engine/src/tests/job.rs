use crate::job::{EventQueue, catch_up};
use crate::*;

/// An event of a model, its data a number to tell it by.
fn event(model: &str, n: i64) -> AwaitEvent {
    AwaitEvent {
        model: model.to_string(),
        data: NuVal::Int(n),
    }
}

/// What a queue gives its sender once closed, each event's model and number.
async fn drained(queue: &EventQueue) -> Vec<(String, NuVal)> {
    queue.close();
    let mut sent = Vec::new();
    while let Some(event) = queue.next().await {
        sent.push((event.model, event.data));
    }
    sent
}

/// A run of Statuses no sender took leaves the newest alone waiting.
#[tokio::test]
async fn a_run_of_statuses_leaves_the_newest_waiting() {
    let queue = EventQueue::default();
    for n in 1..=5 {
        queue.push(event(AWAIT_STATUS_MODEL, n));
    }
    assert_eq!(drained(&queue).await, vec![(AWAIT_STATUS_MODEL.to_string(), NuVal::Int(5))]);
}

/// A Status queued after the Result replaces the one waiting before it, and the
/// Result is sent first; a Result and a Stop are never replaced.
#[tokio::test]
async fn statuses_around_a_result_send_the_result_then_the_late_status() {
    let queue = EventQueue::default();
    queue.push(event(AWAIT_STATUS_MODEL, 1));
    queue.push(event(AWAIT_RESULT_MODEL, 2));
    queue.push(event(AWAIT_STOP_MODEL, 3));
    queue.push(event(AWAIT_STATUS_MODEL, 4));
    let sent = vec![
        (AWAIT_RESULT_MODEL.to_string(), NuVal::Int(2)),
        (AWAIT_STOP_MODEL.to_string(), NuVal::Int(3)),
        (AWAIT_STATUS_MODEL.to_string(), NuVal::Int(4)),
    ];
    assert_eq!(drained(&queue).await, sent);
}

/// A sender waiting on an empty queue wakes for an event, then for the close.
#[tokio::test]
async fn a_waiting_sender_wakes_for_an_event_then_for_the_close() {
    let queue = Arc::new(EventQueue::default());
    let waiter = {
        let queue = queue.clone();
        tokio::spawn(async move { queue.next().await })
    };
    tokio::time::sleep(Duration::from_millis(50)).await;
    queue.push(event(AWAIT_RESULT_MODEL, 1));
    let got = waiter.await.expect("the waiter").map(|event| event.data);
    assert_eq!(got, Some(NuVal::Int(1)), "the event queued wakes the waiting sender");
    let waiter = {
        let queue = queue.clone();
        tokio::spawn(async move { queue.next().await })
    };
    tokio::time::sleep(Duration::from_millis(50)).await;
    queue.close();
    assert!(waiter.await.expect("the waiter").is_none(), "the close ends the sender");
}

/// Nanoseconds as the beats' duration.
fn ns(nanos: u64) -> tk::TkDuration {
    tk::TkDuration::from_nanos(nanos)
}

/// Seconds as the beats' duration.
fn secs(secs: u64) -> tk::TkDuration {
    tk::TkDuration::from_secs(secs)
}

/// The catch-up as it stepped before, one gap a pass, to hold the jump against.
fn stepped(
    gaps: &[tk::TkDuration],
    mut beat: usize,
    mut at: tk::TkInstant,
    now: tk::TkInstant,
) -> (usize, tk::TkInstant) {
    let gap = |beat: usize| gaps[beat.min(gaps.len() - 1)];
    while at <= now && at + gap(beat + 1) <= now {
        beat += 1;
        at += gap(beat);
    }
    (beat, at)
}

/// 1ns gaps a second late come to now in one step, the billion beats between
/// skipped; a day late too, which one gap a pass would never finish.
#[test]
fn one_nanosecond_gaps_late_catch_up_in_one_step() {
    let gaps = [ns(1), ns(1)];
    let at = tk::TkInstant::now();
    let now = at + secs(1);
    assert_eq!(catch_up(&gaps, 1, at, now), (1 + 1_000_000_000, now), "the latest time passed, now itself");
    let now = at + secs(86_400);
    assert_eq!(catch_up(&gaps, 1, at, now), (1 + 86_400_000_000_000, now));
}

/// Each of a list's entries is taken in turn as its time passes, the last
/// repeating: gaps of 1, 2, 3, and 4 seconds put beats at 1, 3, 6, 10, then 14,
/// 18, 22 seconds.
#[test]
fn a_lists_entries_are_passed_in_turn() {
    let gaps = [secs(1), secs(2), secs(3), secs(4)];
    let start = tk::TkInstant::now();
    let after = |ms: u64| start + tk::TkDuration::from_millis(ms);
    let beat_one = after(3000);
    assert_eq!(catch_up(&gaps, 1, beat_one, after(2000)), (1, beat_one), "beat 1 not yet due");
    assert_eq!(catch_up(&gaps, 1, beat_one, after(3500)), (1, beat_one), "beat 1 due, run at once");
    assert_eq!(catch_up(&gaps, 1, beat_one, after(6500)), (2, after(6000)), "beat 1 skipped for 2");
    assert_eq!(catch_up(&gaps, 1, beat_one, after(10_000)), (3, after(10_000)), "a time reached is passed");
    assert_eq!(catch_up(&gaps, 1, beat_one, after(20_000)), (5, after(18_000)), "the last gap repeats");
}

/// The jump lands where stepping one gap a pass lands, on every schedule tried.
#[test]
fn the_jump_lands_where_stepping_lands() {
    let lists: [&[tk::TkDuration]; 3] = [&[ns(3), ns(5)], &[ns(3), ns(5), ns(7)], &[ns(7), ns(2), ns(1), ns(4)]];
    let origin = tk::TkInstant::now();
    for gaps in lists {
        for beat in 0..6 {
            for at in 0..20 {
                for now in 0..120 {
                    let (at, now) = (origin + ns(at), origin + ns(now));
                    assert_eq!(
                        catch_up(gaps, beat, at, now),
                        stepped(gaps, beat, at, now),
                        "gaps {gaps:?}, beat {beat}, at {at:?}, now {now:?}",
                    );
                }
            }
        }
    }
}
