
use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// A definition answering the work dir a call sees.
const WORK_DIR: &str =
    "def execute [data: nothing]: nothing -> record<wd: string> { { wd: $env.EQUIP_WORK_DIR } }";

/// The home a host resolves with `HOME` set to `home`: `home` on Unix, and on
/// Windows the user's profile, which no variable moves.
fn platform_home(home: &std::path::Path) -> std::path::PathBuf {
    if cfg!(windows) {
        std::path::PathBuf::from(std::env::var("USERPROFILE").expect("Windows names the user's profile"))
    } else {
        home.to_path_buf()
    }
}

#[tested]
fn default_id_is_user_env() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_full(t.temp_dir(), &[], &[("USER", "udefault")]);
    let (resp, _) = host.equip_new("sourcetrait/mylib", &t.temp_dir().join("src"));
    assert!(!has_error_path(&resp), "equip new should succeed; got {resp}");
    let lib_dir = namespace_dir(&t.temp_dir().join("data"), "udefault", "default")
        .join("equip")
        .join("sourcetrait")
        .join("mylib")
        .join("rig");
    assert!(
        lib_dir.exists(),
        "default-id namespace should land under <user>/default/; expected {}",
        lib_dir.display(),
    );
}

/// The namespace's homes come from the UENV variables before the XDG ones.
#[tested]
fn the_uenv_homes_come_before_the_xdg_ones() {
    let t = testing::test!({ .using_temp_dir() });
    let data = t.temp_dir().join("uenv-data");
    let cache = t.temp_dir().join("uenv-cache");
    let mut host = Host::spawn_full(
        t.temp_dir(),
        &["--id", "uenvid"],
        &[
            ("UENV_USR_DATA", data.to_str().expect("a UTF-8 path")),
            ("UENV_USR_CACHE", cache.to_str().expect("a UTF-8 path")),
        ],
    );
    let resp = host.execute(WORK_DIR, "null");
    assert!(!has_error_path(&resp), "the call runs; got {resp}");
    assert!(
        namespace_dir(&data, "uenvid", "default").join("equip").exists(),
        "the namespace's equips land under UENV_USR_DATA",
    );
    assert!(
        namespace_dir(&cache, "uenvid", "default").join("execute").exists(),
        "and its calls' logs under UENV_USR_CACHE",
    );
    assert!(
        !namespace_dir(&t.temp_dir().join("data"), "uenvid", "default").exists(),
        "nothing lands under XDG_DATA_HOME",
    );
}

#[tested]
fn explicit_id_and_namespace_select_their_own_dirs() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_args(t.temp_dir(), &["--id", "aid", "--namespace", "ns1"]);
    let (resp, _) = host.equip_new("sourcetrait/mylib", &t.temp_dir().join("src"));
    assert!(!has_error_path(&resp), "equip new should succeed; got {resp}");
    let lib_dir = namespace_dir(&t.temp_dir().join("data"), "aid", "ns1")
        .join("equip")
        .join("sourcetrait")
        .join("mylib")
        .join("rig");
    assert!(lib_dir.exists(), "explicit --id/--namespace should select the namespace; expected {}", lib_dir.display());

    let info = host.info();
    let env = &envelope(&info)["result"];
    assert_eq!(env["id"].as_str(), Some("aid"), "got {env}");
    assert_eq!(env["namespace"].as_str(), Some("ns1"), "got {env}");
}

#[tested]
fn the_test_flag_defaults_the_namespace() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_args(t.temp_dir(), &["--id", "tflag", "--test"]);
    let info = host.info();
    assert_eq!(
        envelope(&info)["result"]["namespace"].as_str(),
        Some("test"),
        "--test names the ordinary test channel without spelling its namespace \
         out; got {info}",
    );
    assert!(
        namespace_dir(&t.temp_dir().join("data"), "tflag", "test").exists(),
        "and the namespace it selects is a real one on disk",
    );
}

#[tested]
fn an_explicit_namespace_beats_the_test_default() {
    let t = testing::test!({ .using_temp_dir() });
    // --test supplies a DEFAULT, never an override. A run that asks for a
    // particular namespace gets it, which is what keeps the flag from being a mode.
    let mut host = Host::spawn_args(
        t.temp_dir(),
        &["--id", "tflag2", "--test", "--namespace", "elsewhere"],
    );
    let info = host.info();
    assert_eq!(
        envelope(&info)["result"]["namespace"].as_str(),
        Some("elsewhere"),
        "an explicit --namespace wins over the --test default; got {info}",
    );
}

#[tested]
fn without_the_flag_the_namespace_is_still_default() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_args(t.temp_dir(), &["--id", "noflag"]);
    let info = host.info();
    assert_eq!(
        envelope(&info)["result"]["namespace"].as_str(),
        Some("default"),
        "normal operation is unchanged; got {info}",
    );
}

#[tested]
fn namespaces_are_disjoint() {
    let t = testing::test!({ .using_temp_dir() });
    {
        let mut ns1 = Host::spawn_args(t.temp_dir(), &["--id", "aid", "--namespace", "ns1"]);
        let (_, src) = ns1.equip_new("sourcetrait/nslib", &t.temp_dir().join("src"));
        write_source(&src, "mod.nu", "export module m\n");
        write_source(&src, "m/mod.nu", "export use double\n");
        write_source(
            &src,
            "m/double/mod.nu",
            &valid_function_source("x: int", "out: int", "{ out: ($data.x * 2) }"),
        );
        let committed = ns1.commit("sourcetrait/nslib");
        assert!(!has_error_path(&committed), "ns1 commit failed: {committed}");
        let called = ns1.call_np("sourcetrait/nslib:m:double", "{x: 4}");
        assert_eq!(envelope(&called)["result"]["out"].as_i64(), Some(8), "ns1 call: {called}");
    }

    let mut ns2 = Host::spawn_args(t.temp_dir(), &["--id", "aid", "--namespace", "ns2"]);
    let info = envelope(&ns2.info());
    let block = info["result"]["signatures"]
        .as_str()
        .expect("info carries a signatures block");
    assert!(
        block.is_empty(),
        "ns2 must not see ns1's rigs, so its block is empty rather than \
         absent; got {block:?}",
    );
    let called = ns2.call_np("sourcetrait/nslib:m:double", "{x: 4}");
    assert!(has_error_path(&called), "ns2 call into ns1's rig must fail; got {called}");

    assert!(namespace_dir(&t.temp_dir().join("data"), "aid", "ns1").exists());
    assert!(namespace_dir(&t.temp_dir().join("data"), "aid", "ns2").exists());
}

#[tested]
fn env_carries_id_namespace_and_work_dir() {
    let t = testing::test!({ .using_temp_dir() });
    let wd = t.temp_dir().join("wd");
    std::fs::create_dir_all(&wd).unwrap();
    let wd_str = wd.to_str().unwrap();
    let mut host = Host::spawn_args(
        t.temp_dir(),
        &["--id", "envid", "--namespace", "envns", "--workdir", wd_str],
    );
    let resp = host.execute(
        "def execute [data: nothing]: nothing -> record<id: string, ns: string, wd: string, nom: string> {\n    { id: $env.EQUIP_ID, ns: $env.EQUIP_NAMESPACE, wd: $env.EQUIP_WORK_DIR, nom: $env.EQUIP_MCP_NOM }\n}",
        "null",
    );
    let env = envelope(&resp);
    assert_eq!(env["result"]["id"].as_str(), Some("envid"), "EQUIP_ID; got {resp}");
    assert_eq!(env["result"]["ns"].as_str(), Some("envns"), "EQUIP_NAMESPACE; got {resp}");
    assert_eq!(env["result"]["wd"].as_str(), Some(wd_str), "EQUIP_WORK_DIR; got {resp}");
    let info = host.info();
    assert_eq!(envelope(&info)["result"]["work_dir"].as_str(), Some(wd_str), "info work_dir; got {info}");
    let nom = envelope(&info)["result"]["nom"].as_str().map(str::to_string);
    assert!(nom.is_some(), "info carries the host's nom; got {info}");
    assert_eq!(
        env["result"]["nom"].as_str(),
        nom.as_deref(),
        "a definition reads the host's nom as EQUIP_MCP_NOM; got {resp}",
    );
}

#[tested]
fn workdir_tilde_expands_against_home() {
    let t = testing::test!({ .using_temp_dir() });
    let home = t.temp_dir().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let home_str = home.to_str().unwrap();
    let mut host = Host::spawn_full(
        t.temp_dir(),
        &["--id", "tildeid", "--workdir", "~/wd_x"],
        &[("HOME", home_str)],
    );
    let resp = host.execute(WORK_DIR, "null");
    let expected = platform_home(&home).join("wd_x");
    assert_eq!(
        envelope(&resp)["result"]["wd"].as_str(),
        expected.to_str(),
        "a ~/ workdir should expand against the home; got {resp}",
    );
}

#[tested]
fn workdir_defaults_under_home_proj_equip_id() {
    let t = testing::test!({ .using_temp_dir() });
    let home = t.temp_dir().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let home_str = home.to_str().unwrap();
    let mut host = Host::spawn_full(t.temp_dir(), &["--id", "wdid"], &[("HOME", home_str)]);
    let resp = host.execute(WORK_DIR, "null");
    let expected = platform_home(&home).join("proj").join("equip").join("wdid");
    assert_eq!(
        envelope(&resp)["result"]["wd"].as_str(),
        expected.to_str(),
        "an absent --workdir should default to <home>/proj/equip/<id>; got {resp}",
    );
}
