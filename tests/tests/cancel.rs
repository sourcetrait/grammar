//! Cooperative cancellation: `interface await stop` stops a running await job, its
//! execute's `sleep` bailing on the cancel before the job's last line runs. A SYSTEM
//! test: the stop goes in over the stdio transport while the job runs, and the job
//! leaves `interface await list`.

use std::time::{Duration, Instant};

use serde_json::json;
use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// The PREPARED record of a suite whose prepare answers its marker's path.
const PREPARED: &str = "record<job: record<id: int, started: datetime>, prepared: record<marker: string>>";

/// A suite whose job naps 3 seconds, then writes its marker, unless it is stopped.
fn suite() -> String {
    format!(
        "export def prepare [data: record<marker: string>]: nothing -> record<marker: string> {{ $data }}
export def execute [data: {PREPARED}]: nothing -> nothing {{
    sleep 3sec
    'ran' | save $data.prepared.marker
}}
export def status [data: {PREPARED}]: nothing -> nothing {{ }}
export def stop [data: {PREPARED}]: nothing -> nothing {{ }}
"
    )
}

/// Whether `interface await list` holds the job under `nonce`.
fn listed(host: &mut Host, nonce: &str) -> bool {
    let resp = host.interface("interface await list");
    envelope(&resp)["result"]
        .as_list()
        .unwrap_or_default()
        .iter()
        .any(|row| row.get("await").and_then(NuVal::as_str) == Some(nonce))
}

/// Poll the await list until it holds the job, or no longer does, as `held` asks.
fn poll_listed(host: &mut Host, nonce: &str, held: bool, secs: u64) -> bool {
    let deadline = Instant::now() + Duration::from_secs(secs);
    loop {
        if listed(host, nonce) == held {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[tested]
fn an_await_stop_stops_a_running_job() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn(t.temp_dir());
    let marker = t.temp_dir().join("ran.txt");
    let data = format!("{{marker: {}}}", nu_literal(&marker.display().to_string()));
    let started = host.call("await", json!({"host": "local", "module": suite(), "data": data}));
    assert!(!has_error_path(&started), "the job starts; got {started}");
    let nonce = envelope(&started)["nonce"].as_str().expect("the call's nonce").to_string();
    assert!(poll_listed(&mut host, &nonce, true, 5), "the job lists by its await");
    let stopped = host.interface(&format!("interface await stop {}", nu_literal(&nonce)));
    assert!(!has_error_path(&stopped), "the stop answers; got {stopped}");
    assert!(poll_listed(&mut host, &nonce, false, 5), "the stopped job leaves the list");
    std::thread::sleep(Duration::from_secs(5));
    assert!(!marker.exists(), "the stopped job never reached its last line");
}
