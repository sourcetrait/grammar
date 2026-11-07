use crate::job::catch_up;
use crate::*;

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
