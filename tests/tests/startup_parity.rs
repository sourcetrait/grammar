//! The host's startup held to nushell's own on a pinned env: `$nu`'s paths, the
//! startup env and constants, std, and `print`, through both of `execute`'s lanes and call.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use nu_config::{CliOverrides, NushellConfigDirs, TestEnv, resolve_paths};
use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

const ID: &str = "parity";
const NAMESPACE: &str = "default";

/// The parent shell's `NU_VERSION`, which the host must not inherit.
const PARENT_NU_VERSION: &str = "0.0.0-parent";

/// Every `$nu` path field, read by a definition.
const NU_PATHS: &str = "{
    home_dir: $nu.home-dir,
    default_config_dir: $nu.default-config-dir,
    config_path: $nu.config-path,
    env_path: $nu.env-path,
    history_path: $nu.history-path,
    loginshell_path: $nu.loginshell-path,
    plugin_path: $nu.plugin-path,
    data_dir: $nu.data-dir,
    cache_dir: $nu.cache-dir,
    vendor_autoload_dirs: $nu.vendor-autoload-dirs,
    user_autoload_dirs: $nu.user-autoload-dirs,
}";

/// The startup env and constants nushell sets, read by a definition.
const STARTUP_ENV: &str = "{
    config: ($env.config | describe | str starts-with 'record<'),
    env_conversions: ($env.ENV_CONVERSIONS | describe),
    path: ($env.PATH | describe),
    nu_lib_dirs_env: $env.NU_LIB_DIRS,
    nu_lib_dirs_const: $NU_LIB_DIRS,
    nu_plugin_dirs_env: $env.NU_PLUGIN_DIRS,
    nu_plugin_dirs_const: $NU_PLUGIN_DIRS,
    nu_version: $env.NU_VERSION,
}";

/// A host on a pinned env, and what nushell resolves for that env.
struct Parity {
    host: Host,
    dirs: NushellConfigDirs,
    namespace: PathBuf,
    cache_namespace: PathBuf,
    host_stderr: PathBuf,
}

impl Parity {
    fn spawn(root: &Path) -> Self {
        let home = root.join("home");
        let config_home = root.join("config");
        let plugins = root.join("plugins");
        for dir in [&home, &config_home, &plugins] {
            std::fs::create_dir_all(dir).expect("mkdir a pinned dir");
        }
        let data_dirs = format!(
            "{}:{}",
            path_str(&root.join("share_a")),
            path_str(&root.join("share_b")),
        );
        let vendor = root.join("vendor");
        let host_stderr = root.join("host_stderr.log");
        let pinned = [
            ("HOME", path_str(&home)),
            ("XDG_CONFIG_HOME", path_str(&config_home)),
            ("XDG_DATA_DIRS", data_dirs.clone()),
            ("NU_VENDOR_AUTOLOAD_DIR", path_str(&vendor)),
            ("NU_VERSION", PARENT_NU_VERSION.to_string()),
            ("NU_LIB_DIRS", path_str(&root.join("parent_lib"))),
            ("GRAMMAR_MCP_PLUGIN_DIR", path_str(&plugins)),
        ];
        let envs: Vec<(&str, &str)> = pinned.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let host = Host::spawn_logged(
            &root.join("scratch"),
            &["--id", ID, "--namespace", NAMESPACE],
            &envs,
            &host_stderr,
        );
        let namespace = namespace_dir(host.data_home(), ID, NAMESPACE);
        let cache_namespace = namespace_dir(host.cache_home(), ID, NAMESPACE);
        let vars = HashMap::from([
            ("XDG_CONFIG_HOME".to_string(), path_str(&config_home)),
            ("XDG_DATA_HOME".to_string(), path_str(host.data_home())),
            ("XDG_CACHE_HOME".to_string(), path_str(host.cache_home())),
            ("XDG_DATA_DIRS".to_string(), data_dirs),
            ("NU_VENDOR_AUTOLOAD_DIR".to_string(), path_str(&vendor)),
        ]);
        let overrides = CliOverrides {
            plugin_file: Some(namespace.join("plugin.msgpackz")),
            ..Default::default()
        };
        // Windows' home is the user's profile, which no variable moves, so there
        // nushell resolves it whatever HOME holds.
        let home = if cfg!(windows) {
            PathBuf::from(std::env::var("USERPROFILE").expect("Windows names the user's profile"))
        } else {
            home
        };
        let (dirs, _) = resolve_paths(&TestEnv::new(vars).with_home_dir(home), &overrides)
            .expect("nushell resolves the pinned env");
        Self {
            host,
            dirs,
            namespace,
            cache_namespace,
            host_stderr,
        }
    }

    /// The clean envelope of a `def` (or `def --env`) answering `result_type` from
    /// `body`, panicking on an error.
    fn eval(&mut self, def: &str, result_type: &str, body: &str) -> NuVal {
        let definition =
            format!("{def} execute [data: nothing]: nothing -> {result_type} {{\n{body}\n}}");
        let resp = self.host.execute(&definition, "null");
        extract_envelope(&resp).unwrap_or_else(|| panic!("`{def}` should succeed; got {resp}"))
    }

    /// The stderr log of the call whose envelope is `env`, under `kind`.
    fn call_stderr(&self, kind: &str, env: &NuVal) -> String {
        let nonce = env["nonce"].as_str().expect("envelope nonce");
        let log = self.cache_namespace.join(kind).join(nonce).join("stderr");
        std::fs::read_to_string(&log).unwrap_or_else(|e| panic!("read {}: {e}", log.display()))
    }

    /// The host's own stderr so far.
    fn host_stderr(&self) -> String {
        std::fs::read_to_string(&self.host_stderr).unwrap_or_default()
    }

    /// The engine version the host was built against, from `interface info`.
    fn engine_version(&mut self) -> String {
        let info = self.host.info();
        envelope(&info)["result"]["nu_version"]
            .as_str()
            .expect("info's nu_version")
            .to_string()
    }

    /// `$NU_PLUGIN_DIRS` as nushell sets it: config home's `plugins`, the exe's dir.
    fn plugin_dirs(&self) -> NuVal {
        let exe = dunce::canonicalize(grammar_mcp_bin()).expect("canonicalize the host");
        let exe_dir = exe.parent().expect("the host's dir");
        NuVal::List(vec![
            NuVal::from(path_str(&self.dirs.config_home.join("plugins"))),
            NuVal::from(path_str(exe_dir)),
        ])
    }
}

/// A record of `fields`, in the order given, as the envelope carries one.
fn record(fields: Vec<(&str, NuVal)>) -> NuVal {
    NuVal::Record(fields.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

/// `$nu`'s path fields as nushell renders them from `dirs`.
fn expected_nu(dirs: &NushellConfigDirs) -> NuVal {
    record(vec![
        ("home_dir", NuVal::from(shown(&dirs.home_dir))),
        ("default_config_dir", NuVal::from(shown(&dirs.config_home))),
        ("config_path", NuVal::from(shown(dirs.config_file.as_path()))),
        ("env_path", NuVal::from(shown(dirs.env_file.as_path()))),
        ("history_path", NuVal::from(shown(&dirs.config_home.join("history.txt")))),
        ("loginshell_path", NuVal::from(shown(&dirs.config_home.join("login.nu")))),
        ("plugin_path", NuVal::from(shown(dirs.plugin_file.as_path()))),
        ("data_dir", NuVal::from(shown(&dirs.data_home))),
        ("cache_dir", NuVal::from(shown(&dirs.cache_home))),
        ("vendor_autoload_dirs", listed(&dirs.vendor_autoload_dirs)),
        ("user_autoload_dirs", listed(&dirs.user_autoload_dirs)),
    ])
}

/// The type of `NU_PATHS`'s record.
const NU_PATHS_TYPE: &str = "record<home_dir: string, default_config_dir: string, \
    config_path: string, env_path: string, history_path: string, loginshell_path: string, \
    plugin_path: string, data_dir: string, cache_dir: string, \
    vendor_autoload_dirs: list<string>, user_autoload_dirs: list<string>>";

/// The type of `STARTUP_ENV`'s record.
const STARTUP_ENV_TYPE: &str = "record<config: bool, env_conversions: string, path: string, \
    nu_lib_dirs_env: list<string>, nu_lib_dirs_const: list<string>, \
    nu_plugin_dirs_env: list<string>, nu_plugin_dirs_const: list<string>, nu_version: string>";

/// A path as `$nu` shows it: canonical where it exists, as given otherwise.
fn shown(path: &Path) -> String {
    if path.exists() {
        path_str(&dunce::canonicalize(path).expect("canonicalize"))
    } else {
        path_str(path)
    }
}

/// A list of paths as `$nu` shows it, each as given.
fn listed(paths: &[PathBuf]) -> NuVal {
    NuVal::List(paths.iter().map(|p| NuVal::String(path_str(p))).collect())
}

fn path_str(path: &Path) -> String {
    path.to_str().expect("a UTF-8 path").to_string()
}

/// A definition's `$nu` paths are nushell's for the same env.
#[tested]
fn the_pool_sees_nushells_paths() {
    let t = testing::test!({ .using_temp_dir() });
    let mut p = Parity::spawn(t.temp_dir());
    let env = p.eval("def", NU_PATHS_TYPE, NU_PATHS);
    assert_eq!(env["result"], expected_nu(&p.dirs), "the definition's $nu paths; got {env}");
}

/// A definition sees the env and constants nushell's startup sets, the host's lib
/// dirs in both `NU_LIB_DIRS`, and the engine's `NU_VERSION`, not the parent's.
#[tested]
fn the_pool_sees_nushells_startup_env() {
    let t = testing::test!({ .using_temp_dir() });
    let mut p = Parity::spawn(t.temp_dir());
    let env = p.eval("def", STARTUP_ENV_TYPE, STARTUP_ENV);
    let lib_dirs = NuVal::List(vec![NuVal::from(path_str(&p.namespace))]);
    let expected = record(vec![
        ("config", NuVal::from(true)),
        ("env_conversions", NuVal::from("record")),
        ("path", NuVal::from("list<string>")),
        ("nu_lib_dirs_env", lib_dirs.clone()),
        ("nu_lib_dirs_const", lib_dirs),
        ("nu_plugin_dirs_env", nuon("[]")),
        ("nu_plugin_dirs_const", p.plugin_dirs()),
        ("nu_version", NuVal::from(p.engine_version())),
    ]);
    assert_eq!(env["result"], expected, "the definition's startup env; got {env}");
    assert_ne!(env["result"]["nu_version"], PARENT_NU_VERSION, "NU_VERSION is the engine's");
}

/// A definition imports and runs nushell's standard library.
#[tested]
fn the_pool_loads_the_standard_library() {
    let t = testing::test!({ .using_temp_dir() });
    let mut p = Parity::spawn(t.temp_dir());
    let body = "use std/assert\nuse std/iter\n\
                let found = ([1 2 3] | iter find {|x| $x > 1 })\n\
                assert equal $found 2\n{ found: $found }";
    let env = p.eval("def", "record<found: int>", body);
    assert_eq!(env["result"]["found"], 2, "std ran; got {env}");
}

/// A parse-time constant reads `$nu.home-dir` through the contract's parse and the eval.
#[tested]
fn a_const_reads_the_home_at_parse_time() {
    let t = testing::test!({ .using_temp_dir() });
    let mut p = Parity::spawn(t.temp_dir());
    let env = p.eval("def", "record<home: string>", "const home = $nu.home-dir\n{ home: $home }");
    assert_eq!(env["result"]["home"], shown(&p.dirs.home_dir), "the const's home; got {env}");
}

/// `print` writes to the call's stderr log, honouring `--no-newline`, and
/// nothing of it reaches the host's own stderr.
#[tested]
fn print_lands_in_the_calls_stderr_log() {
    let t = testing::test!({ .using_temp_dir() });
    let mut p = Parity::spawn(t.temp_dir());
    let body = "print 'PRINTMARK pool'\n42 | print\nprint --no-newline 'PRINTMARK tail'\n{ ok: true }";
    let env = p.eval("def", "record<ok: bool>", body);
    assert_eq!(p.call_stderr("execute", &env), "PRINTMARK pool\n42\nPRINTMARK tail", "the call's log");
    assert!(!p.host_stderr().contains("PRINTMARK"), "the host's stderr: {}", p.host_stderr());
}

/// A stateful definition sees the same startup and its `print` lands in its log.
#[tested]
fn the_stateful_lane_sees_the_same_startup() {
    let t = testing::test!({ .using_temp_dir() });
    let mut p = Parity::spawn(t.temp_dir());
    let body = "print 'PRINTMARK stateful'\n\
                { home: $nu.home-dir, path: ($env.PATH | describe), \
                config: ($env.config | describe | str starts-with 'record<') }";
    let env = p.eval("def --env", "record<home: string, path: string, config: bool>", body);
    let expected = record(vec![
        ("home", NuVal::from(shown(&p.dirs.home_dir))),
        ("path", NuVal::from("list<string>")),
        ("config", NuVal::from(true)),
    ]);
    assert_eq!(env["result"], expected, "the stateful definition's startup; got {env}");
    assert_eq!(p.call_stderr("execute", &env), "PRINTMARK stateful\n", "the call's log");
}

/// With the standard library's files ahead of a rig's in the span space, the
/// validator still reads each token: a `main` variable is refused at its line.
#[tested]
fn a_reserved_main_is_still_refused() {
    let t = testing::test!({ .using_temp_dir() });
    let mut p = Parity::spawn(t.temp_dir());
    let (_, src) = p.host.equip_new("sourcetrait/reserved", &t.temp_dir().join("src"));
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export module probe\nexport use probe\n");
    write_source(
        &src,
        "m/probe/mod.nu",
        "export def main [data: record<noop: int>]: nothing -> record<n: int> {\n\
         let main = 1\n\
         { n: $main }\n\
         }\n",
    );
    let committed = p.host.commit("sourcetrait/reserved");
    let reserved: Vec<NuVal> = envelope_error(&committed)
        .and_then(|error| error.as_list().map(<[NuVal]>::to_vec))
        .map(|rows| rows.into_iter().filter(|row| row["kind"] == "rig::reserved").collect())
        .unwrap_or_default();
    assert_eq!(reserved.len(), 1, "one reserved `main`; got {committed}");
    assert_eq!(reserved[0]["source"]["position"], nuon("[2, 0]"), "at its line; got {committed}");
}

/// A rig reading `$nu.home-dir` in a constant and using std passes the
/// validator, and its call sees the same startup and logs its `print`.
#[tested]
fn a_rigs_call_sees_the_same_startup() {
    let t = testing::test!({ .using_temp_dir() });
    let mut p = Parity::spawn(t.temp_dir());
    let (_, src) = p.host.equip_new("sourcetrait/parity", &t.temp_dir().join("src"));
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export module probe\nexport use probe\n");
    write_source(
        &src,
        "m/probe/mod.nu",
        "use std/assert\n\
         const HOME = $nu.home-dir\n\
         export def main [data: record<noop: int>]: nothing -> \
         record<home: string, path: string, plugin_dirs: list<string>> {\n\
         assert equal 1 1\n\
         print 'PRINTMARK call'\n\
         { home: $HOME, path: ($env.PATH | describe), plugin_dirs: $NU_PLUGIN_DIRS }\n\
         }\n",
    );
    let committed = p.host.commit("sourcetrait/parity");
    assert!(!has_error_path(&committed), "the validator takes the rig; got {committed}");
    let resp = p.host.call_np("sourcetrait/parity:m:probe", "{noop: 0}");
    let env = extract_envelope(&resp).unwrap_or_else(|| panic!("the call should succeed; got {resp}"));
    let expected = record(vec![
        ("home", NuVal::from(shown(&p.dirs.home_dir))),
        ("path", NuVal::from("list<string>")),
        ("plugin_dirs", p.plugin_dirs()),
    ]);
    assert_eq!(env["result"], expected, "the call's startup; got {env}");
    assert_eq!(p.call_stderr("calls", &env), "PRINTMARK call\n", "the call's log");
}
