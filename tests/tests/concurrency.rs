//! The stateless executor's concurrency bound. Fire N concurrent execute()s (N above
//! the semaphore cap) over the stdio transport: every one completes (the buffered
//! multi-read on `Host` collects them all back by id), and the calls' own start and
//! end times never overlap past the cap (= available_parallelism / 2) - the pool
//! saturates to exactly the cap and never past it. A SYSTEM test: it needs real
//! pipelined concurrency against the spawned binary plus the process-global
//! concurrency cap.

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

    // Fire N concurrent calls, each stamping its start, sleeping ~1s so they overlap,
    // and stamping its end; do NOT read the responses yet. They come back
    // near-simultaneously ~1s later and must be collected by id, not discarded.
    let ids: Vec<(usize, u64)> = (0..n)
        .map(|i| {
            let id = host.request(
                "execute",
                execute_args(
                    "def execute [data: record<i: int>]: nothing -> record<out: int, start: int, end: int> {\n    let start = (date now | into int)\n    sleep 1sec\n    { out: $data.i, start: $start, end: (date now | into int) }\n}",
                    &format!("{{i: {i}}}"),
                ),
            );
            (i, id)
        })
        .collect();

    // Every call completes with its own result - the buffered multi-read collects
    // all N by id, none lost to the interleaved arrival - and its span on the pool.
    let mut spans: Vec<(i64, i64)> = Vec::new();
    for (i, id) in ids {
        let resp = host.read_id(id);
        assert!(!has_error_path(&resp), "call {i} should complete; got {resp}");
        let env = envelope(&resp);
        let result = &env["result"];
        assert_eq!(result["out"].as_i64(), Some(i as i64), "call {i} should return its own data; got {resp}");
        let start = result["start"].as_i64().unwrap_or_else(|| panic!("call {i}'s start; got {resp}"));
        let end = result["end"].as_i64().unwrap_or_else(|| panic!("call {i}'s end; got {resp}"));
        spans.push((start, end));
    }

    // The most calls on the pool at once: at each call's start, the spans holding it.
    // Each span is stamped inside its eval, under its permit, so the stamps can't show
    // more evals at once than the semaphore let run.
    let max_in_flight = spans
        .iter()
        .map(|(at, _)| spans.iter().filter(|(start, end)| start <= at && at < end).count())
        .max()
        .unwrap_or(0);

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
