//! A call's `print` and `grimm dbg` land in that call's own logs, whichever call
//! parsed the code that runs them.

use std::path::Path;

use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

const ID: &str = "routing";
const NAMESPACE: &str = "default";

/// The clean envelope of a stateful `execute` call running `body`, panicking on an error.
fn stateful(host: &mut Host, body: &str) -> NuVal {
    let definition = format!("def --env execute [data: nothing]: nothing -> record<ok: bool> {{\n{body}\n}}");
    let resp = host.execute(&definition, "null");
    extract_envelope(&resp).unwrap_or_else(|| panic!("the stateful call should succeed; got {resp}"))
}

/// The file `name` in the log dir of the call whose envelope is `env`.
fn call_log(host: &Host, env: &NuVal, name: &str) -> String {
    let nonce = env["nonce"].as_str().expect("envelope nonce");
    let log = namespace_dir(host.cache_home(), ID, NAMESPACE)
        .join("execute")
        .join(nonce)
        .join(name);
    std::fs::read_to_string(&log).unwrap_or_default()
}

/// A module file at `path` holding one exported def.
fn write_module(path: &Path, def: &str) {
    std::fs::write(path, def).expect("write the module");
}

/// The body that imports `module` and runs `call` with the mark.
fn body(module: &Path, call: &str, mark: &str) -> String {
    let module = module.to_str().expect("a UTF-8 path");
    format!("use '{module}'\n{call} '{mark}'\n{{ ok: true }}")
}

/// A module imported by two stateful calls prints into each call's own log,
/// never the call that first imported it.
#[tested]
fn stateful_print_follows_each_call() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_args(&t.temp_dir().join("scratch"), &["--id", ID, "--namespace", NAMESPACE]);
    let module = t.temp_dir().join("printer.nu");
    write_module(&module, "export def say [s: string] { print $s }\n");
    let first = stateful(&mut host, &body(&module, "printer say", "PRINTMARK first"));
    let second = stateful(&mut host, &body(&module, "printer say", "PRINTMARK second"));
    assert_eq!(call_log(&host, &first, "stderr"), "PRINTMARK first\n", "the first call's log");
    assert_eq!(call_log(&host, &second, "stderr"), "PRINTMARK second\n", "the second call's log");
}

/// A module imported by two stateful calls appends its `grimm dbg` to each
/// call's own trace, never the call that first imported it.
#[tested]
fn stateful_grimm_dbg_follows_each_call() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_args(&t.temp_dir().join("scratch"), &["--id", ID, "--namespace", NAMESPACE]);
    let module = t.temp_dir().join("tracer.nu");
    write_module(&module, "export def mark [s: string] { grimm dbg {mark: $s} }\n");
    let first = stateful(&mut host, &body(&module, "tracer mark", "first"));
    let second = stateful(&mut host, &body(&module, "tracer mark", "second"));
    assert_eq!(call_log(&host, &first, "debug.nuonl"), "{mark: first}\n", "the first call's trace");
    assert_eq!(call_log(&host, &second, "debug.nuonl"), "{mark: second}\n", "the second call's trace");
}
