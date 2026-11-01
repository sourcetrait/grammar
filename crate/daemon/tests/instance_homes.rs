//! The worker's data and cache homes against the MCP's, under one environment.

use std::collections::BTreeMap;

use sourcetrait_grammar_daemon::*;

/// Alone in its binary, so nothing reads the environment while this test sets it.
#[test]
fn the_engine_homes_are_the_mcps_own() {
    let root = tempfile::tempdir().expect("a scratch root");
    let home = root.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let mut env = BTreeMap::new();
    let tmp = root.path().join("tmp");
    std::fs::create_dir_all(&tmp).unwrap();
    env.insert("UENV_USR_TMP".to_string(), tmp.to_string_lossy().into_owned());
    env.insert("HOME".to_string(), home.to_string_lossy().into_owned());
    let xdg = [("XDG_DATA_HOME", root.path().join("xdg-data")), ("XDG_CACHE_HOME", root.path().join("xdg-cache"))];
    for with_xdg in [false, true] {
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::remove_var("UENV_USR_DATA");
            std::env::remove_var("UENV_USR_CACHE");
            for (var, path) in &xdg {
                if with_xdg {
                    std::env::set_var(var, path);
                } else {
                    std::env::remove_var(var);
                }
            }
        }
        for (var, path) in &xdg {
            if with_xdg {
                env.insert(var.to_string(), path.to_string_lossy().into_owned());
            } else {
                env.remove(*var);
            }
        }
        let mcp = directories::BaseDirs::new().expect("the MCP's base dirs");
        let (data, cache) = (mcp.data_dir().to_path_buf(), mcp.cache_dir().to_path_buf());
        std::fs::create_dir_all(&data).unwrap();
        std::fs::create_dir_all(&cache).unwrap();
        let resolved = InstanceHomes::resolve(&env).expect("every home resolves");
        assert_eq!((resolved.data, resolved.cache), (data, cache), "XDG's variables set: {with_xdg}");
    }
}
