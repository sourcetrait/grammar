use crate::*;

/// How much one origin may emit before it is warned, then stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpamThresholds {
    pub warn_window: Duration,
    pub warn_rate: u32,
    pub error_window: Duration,
    pub error_rate: u32,
}

impl SpamThresholds {
    /// The longer of the two windows.
    pub(crate) fn retention(&self) -> Duration {
        if self.warn_window > self.error_window {
            self.warn_window
        } else {
            self.error_window
        }
    }

    /// The thresholds as the wire carries them, windows in whole seconds.
    pub fn data(self) -> SpamThresholdsData {
        SpamThresholdsData {
            warn_window_secs: self.warn_window.as_secs(),
            warn_rate: self.warn_rate,
            error_window_secs: self.error_window.as_secs(),
            error_rate: self.error_rate,
        }
    }

    /// The wire's thresholds, a zero window or rate refused.
    pub fn from_data(data: SpamThresholdsData) -> Result<Self, String> {
        Ok(Self {
            warn_window: positive_window("spam_warn_window_secs", data.warn_window_secs)?,
            warn_rate: positive_rate("spam_warn_rate", data.warn_rate)?,
            error_window: positive_window("spam_error_window_secs", data.error_window_secs)?,
            error_rate: positive_rate("spam_error_rate", data.error_rate)?,
        })
    }

    /// A partial update, applied whole or not at all.
    pub(crate) fn updated(
        self,
        update: ThresholdsRequest,
    ) -> Result<Self, String> {
        let mut spam = self;
        if let Some(secs) = update.warn_window_secs {
            spam.warn_window = positive_window("spam_warn_window_secs", secs)?;
        }
        if let Some(rate) = update.warn_rate {
            spam.warn_rate = positive_rate("spam_warn_rate", rate)?;
        }
        if let Some(secs) = update.error_window_secs {
            spam.error_window = positive_window("spam_error_window_secs", secs)?;
        }
        if let Some(rate) = update.error_rate {
            spam.error_rate = positive_rate("spam_error_rate", rate)?;
        }
        Ok(spam)
    }
}

fn positive_window(
    name: &str,
    secs: u64,
) -> Result<Duration, String> {
    if secs == 0 {
        Err(format!("{name} must be a positive number of seconds"))
    } else {
        Ok(Duration::from_secs(secs))
    }
}

fn positive_rate(
    name: &str,
    rate: u32,
) -> Result<u32, String> {
    if rate == 0 {
        Err(format!("{name} must be greater than zero"))
    } else {
        Ok(rate)
    }
}

/// What the spam counter decided about one send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpamVerdict {
    /// Under both thresholds.
    Clear,
    /// Crossed the soft threshold for the first time.
    Warn { hits: u32, window_secs: u64 },
    /// Crossed the hard threshold; refuse the send and stop the offender.
    Stop {
        hits: u32,
        window_secs: u64,
        notify: bool,
    },
}

/// One origin's recent sends, plus its warned and stopped flags.
#[derive(Default)]
pub(crate) struct OriginCounter {
    hits: Vec<Instant>,
    warned: bool,
    stopped: bool,
}

/// The live policy and each origin's recent sends.
pub(crate) struct Spam {
    pub(crate) thresholds: SpamThresholds,
    /// Recent sends per origin, bounded by eviction rather than by a cap.
    counters: HashMap<String, OriginCounter>,
}

impl Spam {
    pub(crate) fn new(thresholds: SpamThresholds) -> Self {
        Self {
            thresholds,
            counters: HashMap::new(),
        }
    }

    /// Record one send by `from` at `now` and say what it costs.
    pub(crate) fn record_at(
        &mut self,
        from: &str,
        now: Instant,
    ) -> SpamVerdict {
        let spam = self.thresholds;
        let retention = spam.retention();
        self.counters.retain(|_, counter| {
            counter
                .hits
                .retain(|at| now.saturating_duration_since(*at) < retention);
            !counter.hits.is_empty()
        });
        let counter = self.counters.entry(from.to_string()).or_default();
        counter.hits.push(now);
        let within = |window: Duration| -> u32 {
            counter
                .hits
                .iter()
                .filter(|at| now.saturating_duration_since(**at) < window)
                .count() as u32
        };
        let errors = within(spam.error_window);
        if errors >= spam.error_rate {
            let notify = !counter.stopped;
            counter.stopped = true;
            return SpamVerdict::Stop {
                hits: errors,
                window_secs: spam.error_window.as_secs(),
                notify,
            };
        }
        let warns = within(spam.warn_window);
        if warns >= spam.warn_rate && !counter.warned {
            counter.warned = true;
            return SpamVerdict::Warn {
                hits: warns,
                window_secs: spam.warn_window.as_secs(),
            };
        }
        SpamVerdict::Clear
    }
}
