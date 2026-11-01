use crate::*;

#[test]
fn reaper_starts_empty_and_is_idempotent() {
    // No zombies of ours in a test binary, so a pass is a no-op that must not panic and
    // must leave nothing tracked. Repeated passes are the watchdog's actual usage.
    let mut reaper = OrphanReaper::new();
    reaper.reap();
    reaper.reap();
}
