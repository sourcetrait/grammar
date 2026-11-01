//! The stateless executor's concurrency bound. Fire N concurrent execute()s (N above
//! the semaphore cap) over the stdio transport: every one completes (the buffered
//! multi-read on `Host` collects them all back by id), and `interface process list`
//! never shows more than the cap (= available_parallelism / 2) in flight - the pool
//! saturates to exactly the cap and never past it. A SYSTEM test: it needs real
//! pipelined concurrency against the spawned binary plus the process-global
//! concurrency cap.

use std::time::{Duration, Instant};

use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// Mirrors `server::run::eval_concurrency_cap`: available_parallelism / 2, min 1.
fn expected_cap() -> usize {
    (std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        / 2)
    .max(1)
}

#[tested]
fn concurrent_calls_complete_and_hold_the_semaphore_bound() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn(t.temp_dir());
    let cap = expected_cap();
    let n = cap + 3;

    // Fire N concurrent calls, each sleeping ~1s so they overlap; do NOT read the
    // responses yet. They come back near-simultaneously ~1s later and must be
    // collected by id, not discarded.
    let ids: Vec<(usize, u64)> = (0..n)
        .map(|i| {
            let id = host.request(
                "execute",
                execute_args(
                    "def execute [data: record<i: int>]: nothing -> record<out: int> {\n    sleep 1sec\n    { out: $data.i }\n}",
                    &format!("{{i: {i}}}"),
                ),
            );
            (i, id)
        })
        .collect();

    // While the first wave runs, poll the process list and record the max concurrent
    // `execute` count. 1s definitions + a sub-1s window means the whole first wave is in
    // flight throughout, so the pool sits saturated at the cap (the extra
    // n - cap calls wait on the semaphore and only start as permits free).
    let mut max_in_flight = 0usize;
    let deadline = Instant::now() + Duration::from_millis(900);
    while Instant::now() < deadline {
        let resp = host.interface("interface process list");
        let in_flight = envelope(&resp)["result"]["processes"]
            .as_list()
            .map(|a| {
                a.iter()
                    .filter(|e| e.get("tool").and_then(|t| t.as_str()) == Some("execute"))
                    .count()
            })
            .unwrap_or(0);
        max_in_flight = max_in_flight.max(in_flight);
        std::thread::sleep(Duration::from_millis(25));
    }

    // Every call completes with its own result - the buffered multi-read collects
    // all N by id, none lost to the interleaved arrival.
    for (i, id) in ids {
        let resp = host.read_id(id);
        assert!(!has_error_path(&resp), "call {i} should complete; got {resp}");
        assert_eq!(
            envelope(&resp)["result"]["out"].as_i64(),
            Some(i as i64),
            "call {i} should return its own data; got {resp}",
        );
    }

    // The bound held: never more than the cap ran at once...
    assert!(
        max_in_flight <= cap,
        "in-flight peak {max_in_flight} exceeded the concurrency cap {cap}",
    );
    // ...and the pool actually saturated to the full cap under N > cap load.
    assert_eq!(
        max_in_flight, cap,
        "the pool should saturate to cap {cap} under {n} concurrent calls; peak was {max_in_flight}",
    );
}
