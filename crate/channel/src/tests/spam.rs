use crate::*;

/// A tight, explicit policy, so the tests never lean on the shipped defaults.
fn policy(
    warn_rate: u32,
    error_rate: u32,
) -> Spam {
    Spam::new(SpamThresholds {
        warn_window: Duration::from_secs(10),
        warn_rate,
        error_window: Duration::from_secs(10),
        error_rate,
    })
}

#[test]
fn the_soft_threshold_warns_exactly_once_per_origin() {
    let mut spam = policy(3, 100);
    let now = Instant::now();
    assert_eq!(spam.record_at("thread/a", now), SpamVerdict::Clear);
    assert_eq!(spam.record_at("thread/a", now), SpamVerdict::Clear);
    assert!(
        matches!(spam.record_at("thread/a", now), SpamVerdict::Warn { hits: 3, .. }),
        "the third send reaches the rate",
    );
    assert_eq!(
        spam.record_at("thread/a", now),
        SpamVerdict::Clear,
        "one warning per abuser - a report about spam must not itself become spam",
    );
}

#[test]
fn the_hard_threshold_stops_and_outranks_the_warning() {
    let mut spam = policy(2, 3);
    let now = Instant::now();
    spam.record_at("thread/a", now);
    assert!(matches!(spam.record_at("thread/a", now), SpamVerdict::Warn { .. }));
    assert!(
        matches!(spam.record_at("thread/a", now), SpamVerdict::Stop { hits: 3, notify: true, .. }),
        "the hard threshold is checked first, so crossing both reports the stop",
    );
    assert!(
        matches!(spam.record_at("thread/a", now), SpamVerdict::Stop { notify: false, .. }),
        "the refusal persists, but only an episode's first crossing is announced",
    );
}

#[test]
fn origins_are_counted_separately() {
    let mut spam = policy(2, 100);
    let now = Instant::now();
    spam.record_at("thread/a", now);
    assert_eq!(
        spam.record_at("thread/b", now),
        SpamVerdict::Clear,
        "one origin's traffic must not flag another's",
    );
    assert!(matches!(spam.record_at("thread/a", now), SpamVerdict::Warn { .. }));
}

#[test]
fn hits_outside_the_window_stop_counting_and_the_origin_is_evicted() {
    let mut spam = policy(3, 100);
    let start = Instant::now();
    spam.record_at("thread/a", start);
    spam.record_at("thread/a", start);
    let later = start + Duration::from_secs(11);
    assert_eq!(
        spam.record_at("thread/a", later),
        SpamVerdict::Clear,
        "past the 10s window the two earlier hits no longer count, so this is a first send",
    );
    assert_eq!(
        spam.record_at("thread/a", later),
        SpamVerdict::Clear,
        "and the window really did reset rather than merely skipping one",
    );
}

#[test]
fn the_wire_thresholds_refuse_a_zero_window_or_rate() {
    let data = SpamThresholdsData {
        warn_window_secs: 10,
        warn_rate: 5,
        error_window_secs: 10,
        error_rate: 9,
    };
    let thresholds = SpamThresholds::from_data(data).expect("a positive policy");
    assert_eq!(thresholds.data(), data, "the wire's form round-trips");
    assert!(SpamThresholds::from_data(SpamThresholdsData { warn_rate: 0, ..data }).is_err());
    assert!(
        SpamThresholds::from_data(SpamThresholdsData { error_window_secs: 0, ..data }).is_err()
    );
}
