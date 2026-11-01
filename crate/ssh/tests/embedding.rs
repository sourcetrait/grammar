//! An embedding host's use of the runtime library, without MCP or the service.

mod support;

use sourcetrait_grammar_ssh::{GrammarSshError, Runtime};

#[test]
fn a_process_starts_one_runtime() {
    let instance = support::TestInstance::new("embedder");
    let config = instance.load("test");
    let runtime = Runtime::start(config.clone()).expect("the first start");
    assert_eq!(runtime.identity(), &config.identity);
    let tmp = runtime.scratch().tmp().to_path_buf();
    assert!(tmp.is_dir(), "the worker's scratch is made");
    assert!(config.paths.lock.is_file(), "and its lock taken");
    let info = runtime.info();
    assert_eq!((info.id.as_str(), info.namespace.as_str(), info.nom.as_str()), ("embedder", "test", runtime.nom()));

    assert_eq!(Runtime::start(config.clone()).err(), Some(GrammarSshError::AlreadyStarted));

    runtime.shutdown().expect("the shutdown");
    assert!(!tmp.exists(), "the shutdown removes the scratch");
    assert_eq!(
        Runtime::start(config).err(),
        Some(GrammarSshError::AlreadyStarted),
        "a process starts no second runtime, after a shutdown too",
    );
}
