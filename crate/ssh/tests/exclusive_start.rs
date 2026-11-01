//! A namespace's one worker: the lock held for its life, a killed one's passed on.

mod support;

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
#[cfg(unix)]
use std::time::Duration;

#[cfg(unix)]
use support::exit_within;
use support::{TestInstance, open, runtime};

use sourcetrait_grammar_protocol::LIMITS_DEFAULT;

#[test]
fn two_namespaces_start_apart() {
    let instance = TestInstance::new("fixture");
    runtime().block_on(async {
        let (_default, default) = open(&instance, "default", LIMITS_DEFAULT).await;
        let (_other, other) = open(&instance, "other", LIMITS_DEFAULT).await;
        let (default, other) = (default.expect("default's worker").0, other.expect("other's worker, its lock its own").0);
        assert_ne!(instance.load("default").paths.lock, instance.load("other").paths.lock);
        assert_ne!(default.worker().nom, other.worker().nom, "a nom each");
    });
}

/// The lock's inode and record are read on Unix alone: Windows' lock covers the
/// whole file, so no other process reads it while it is held.
#[test]
fn a_killed_workers_lock_passes_to_its_restart() {
    let instance = TestInstance::new("fixture");
    #[cfg_attr(not(unix), allow(unused_variables))]
    let lock = instance.load("default").paths.lock;
    runtime().block_on(async {
        let (mut killed, opened) = open(&instance, "default", LIMITS_DEFAULT).await;
        let (session, _events) = opened.expect("the first worker");
        #[cfg(unix)]
        let inode = std::fs::metadata(&lock).unwrap().ino();
        let killed_nom = session.worker().nom.clone();
        killed.kill().await.expect("killed");
        #[cfg_attr(not(unix), allow(unused_variables))]
        let (restarted, opened) = open(&instance, "default", LIMITS_DEFAULT).await;
        let (session, _events) = opened.expect("the restart takes the released lock");
        #[cfg(unix)]
        {
            assert_eq!(std::fs::metadata(&lock).unwrap().ino(), inode, "the lock's inode kept");
            let record = std::fs::read_to_string(&lock).unwrap();
            assert!(record.contains(&format!("pid: {}", restarted.id().unwrap())), "{record}");
            assert!(record.contains("holder: grammar-ssh"), "{record}");
        }
        assert_ne!(session.worker().nom, killed_nom, "a fresh nom");
        assert_eq!(TestInstance::worker_dirs(&instance.home("tmp")).len(), 1, "the killed worker's scratch swept");
    });
}

/// Unix alone: a directory's mode is what makes it refuse new entries there,
/// and Windows' read-only attribute doesn't stop a directory taking them.
#[cfg(unix)]
#[test]
fn a_failed_start_refuses_its_hello_and_releases_its_lock() {
    let instance = TestInstance::new("fixture");
    let tmp = instance.home("tmp");
    runtime().block_on(async {
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o500)).unwrap();
        let (mut failed, refused) = open(&instance, "default", LIMITS_DEFAULT).await;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o700)).unwrap();
        let refusal = refused.expect_err("no scratch can be made");
        assert!(refusal.message().contains("scratch::unavailable"), "{}", refusal.message());
        let status = exit_within(&mut failed, Duration::from_secs(30)).await;
        assert_eq!(status.code(), Some(73), "{status:?}");
        let (_worker, opened) = open(&instance, "default", LIMITS_DEFAULT).await;
        opened.expect("the lock was released");
    });
}
