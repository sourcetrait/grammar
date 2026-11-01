# spam.rs

The send-rate policy: one origin's sends counted against two windows, warned
once, then stopped.

## struct SpamThresholds
Two independent (window, rate) pairs so warn and error can measure different
things - a short window catches a burst, a longer one catches sustained
misbehaviour.

The policy in force has its windows as durations. The wire carries them in whole
seconds (`SpamThresholdsData`), as the config file and `interface channel
config` do; `from_data` refuses a zero window or rate, which would forbid the
first send rather than police a rate.

### fn updated
A partial update applied whole or not at all, so a rejected field never leaves
the others half-moved.

## enum SpamVerdict
`notify` on `Stop` is true only on the first crossing of an episode: the
refusal has to persist for every later send, but announcing it every time would
make the report about spam into spam itself.

## struct OriginCounter
Both flags reset when the window empties and the origin is evicted, so a
producer that goes quiet and later misbehaves again is a fresh episode rather
than one that never ended.

## struct Spam
### field counters
Bounded by eviction rather than by a cap: an origin whose window empties is
dropped, so this holds only currently-emitting work rather than one entry per
origin for the host's whole life.

### fn record_at
Counting keys on the origin, which is host-stamped, so a definition cannot
spread its traffic across identities to stay under the rate. `now` is injected,
so the windows are testable.
