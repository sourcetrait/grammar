//! mcp's controller supervising: a request reaches the engine through it, the
//! controller holds the namespace's lock while it runs, and the ordered shutdown
//! stops the engine and the operator, then releases the lock. Its own binary,
//! since the lock is the namespace's.

use sourcetrait_grammar_mcp::guts::SupervisedSystem;

#[test]
fn mcp_system_starts_both_components_and_stops_them() {
    let system = SupervisedSystem::start();
    assert!(system.engine_answers(), "a request reaches the engine through mcp's controller");
    assert!(system.host_lock_held(), "the controller holds the namespace's lock while it runs");
    system.shutdown();
    assert!(!system.host_lock_held(), "the shutdown releases the lock once its systems stop");
    assert!(
        !system.engine_answers(),
        "the engine stopped, so a request answers its failure",
    );
}
