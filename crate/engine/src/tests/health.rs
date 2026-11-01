use crate::*;

fn watch(nonce: &str, lane: Lane, finished: bool) -> HungWatch {
    HungWatch {
        nonce: nonce.to_string(),
        tool: "execute",
        lane,
        started_at: 0,
        cancelled_at: 1000,
        finished: Arc::new(AtomicBool::new(finished)),
    }
}

/// A finished thread leaves the registry when the snapshot is read; a running
/// one is reported with its lane and stays watched.
#[test]
fn a_snapshot_prunes_finished_threads_and_reports_the_rest() {
    let registry: HungRegistry = Arc::new(std::sync::Mutex::new(HashMap::new()));
    register_hung(&registry, watch("done", Lane::Stateless, true));
    register_hung(&registry, watch("hung", Lane::Stateful, false));
    let snapshot = hung_snapshot(&registry);
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].nonce, "hung");
    assert_eq!(snapshot[0].lane, Lane::Stateful);
    let map = registry.lock().unwrap();
    assert!(!map.contains_key("done"), "a finished entry must be pruned");
    assert!(map.contains_key("hung"), "a running entry stays watched");
}

/// The first stamp of a cancelled call stands: a kill and its later timeout
/// both register it, and the earlier cancel time is the honest one.
#[test]
fn a_second_registration_keeps_the_first_stamp() {
    let registry: HungRegistry = Arc::new(std::sync::Mutex::new(HashMap::new()));
    register_hung(&registry, watch("w", Lane::Stateless, false));
    let mut later = watch("w", Lane::Stateless, false);
    later.cancelled_at = 9000;
    register_hung(&registry, later);
    let snapshot = hung_snapshot(&registry);
    assert_eq!(snapshot[0].cancelled_at, 1000);
}
