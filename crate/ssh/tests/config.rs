//! A config's names, homes, paths, workdir, and supervisor lines, each held or refused.

use std::collections::BTreeMap;
use std::path::PathBuf;

use sourcetrait_grammar_ssh::guts::{Config, Homes, Name};
use sourcetrait_grammar_ssh::{CONFIG_FILE, GrammarSshResult};

/// The standard variable behind `UENV_USR_TMP` on this platform.
const TMP_STANDARD: &str = if cfg!(windows) { "TMP" } else { "TMPDIR" };

/// grammar-ssh's own effective UID.
fn uid() -> u32 {
    sourcetrait_grammar_scratch::current_uid()
}

/// A scratch tree of homes and a workdir: each made, and the environment naming them.
struct Scratch {
    root: tempfile::TempDir,
    env: BTreeMap<String, String>,
}

impl Scratch {
    /// Homes named by their UENV variables, every one made.
    fn new() -> Self {
        let root = tempfile::tempdir().expect("a scratch root");
        let mut env = BTreeMap::new();
        for (var, dir) in [
            ("UENV_USR_DATA", "data"),
            ("UENV_USR_CACHE", "cache"),
            ("UENV_USR_TMP", "tmp"),
            ("UENV_USR_TRANSIENT", "run"),
        ] {
            let path = root.path().join(dir);
            std::fs::create_dir_all(&path).expect("a home");
            env.insert(var.to_string(), path.to_string_lossy().into_owned());
        }
        std::fs::create_dir_all(root.path().join("work")).expect("the workdir");
        env.insert("HOME".to_string(), root.path().to_string_lossy().into_owned());
        Self { root, env }
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.root.path().join(rel)
    }

    /// `text` as the workdir's config file.
    fn config(&self, text: &str) {
        let file = self.path("work").join(CONFIG_FILE);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }

    fn load(&self) -> GrammarSshResult<Config> {
        Config::load(uid(), "fixture", "default", &self.path("work"), &self.env)
    }
}

#[test]
fn a_name_is_refused_never_sanitized() {
    for good in ["fixture", "default", "a", "A9", "x.y_z-1", &"n".repeat(64)] {
        assert!(Name::parse("id", good).is_ok(), "{good:?}");
    }
    let bad = ["", ".", "..", ".hidden", "-x", "_x", "a/b", "a\\b", "a\0b", "a b", "é", &"n".repeat(65), "../up"];
    for name in bad {
        let refused = Name::parse("namespace", name).expect_err(name);
        assert_eq!(refused.kind().name(), "config::name_invalid", "{name:?}");
    }
}

#[test]
fn a_uenv_home_comes_before_its_standard_variable_and_its_default() {
    let homes = Scratch::new();
    let mut env = homes.env.clone();
    let standard = homes.path("tmpdir");
    std::fs::create_dir_all(&standard).unwrap();
    env.insert(TMP_STANDARD.to_string(), standard.to_string_lossy().into_owned());
    assert_eq!(Homes::resolve(&env).unwrap().tmp, homes.path("tmp"), "UENV first");
    env.insert("UENV_USR_TMP".to_string(), String::new());
    let resolved = Homes::resolve(&env).unwrap();
    assert_eq!(resolved.tmp, standard, "an empty UENV reads as unset");
    assert!(resolved.diagnostics.iter().any(|d| d == "UENV_USR_TMP is ignored, empty"), "and says so");
    env.insert("UENV_USR_TMP".to_string(), "relative/tmp".to_string());
    let resolved = Homes::resolve(&env).unwrap();
    assert_eq!(resolved.tmp, standard, "a relative UENV is ignored");
    assert!(resolved.diagnostics.iter().any(|d| d.contains("UENV_USR_TMP")), "and says so");
}

#[test]
fn a_missing_home_is_refused_and_never_made() {
    let homes = Scratch::new();
    let mut env = homes.env.clone();
    let missing = homes.path("not-there");
    env.insert("UENV_USR_CACHE".to_string(), missing.to_string_lossy().into_owned());
    let refused = Homes::resolve(&env).expect_err("a missing cache home");
    assert_eq!(refused.kind().name(), "config::home_unavailable");
    assert!(!missing.exists(), "the home was not made");
    let mut no_runtime = homes.env.clone();
    no_runtime.remove("UENV_USR_TRANSIENT");
    let resolved = Homes::resolve(&no_runtime).expect("every other home resolves");
    assert_eq!(resolved.runtime, None, "the runtime home has no default");
}

#[test]
fn the_config_loads_its_paths_and_the_workdirs_supervisor_lines() {
    let homes = Scratch::new();
    let loaded = homes.load().expect("the config loads");
    assert_eq!((loaded.uid, loaded.id.as_str(), loaded.namespace.as_str()), (uid(), "fixture", "default"));
    assert_eq!(loaded.workdir, homes.path("work"));
    let namespace = homes.path("data").join("grammar").join("mcp").join("fixture").join("default");
    assert_eq!(loaded.data_home(), namespace);
    assert_eq!(loaded.lock_file(), namespace.join("host.lock"));
    assert_eq!(loaded.cache_home(), homes.path("cache").join("grammar").join("mcp").join("fixture").join("default"));
    assert_eq!(loaded.supervisor.cpu_warn_fraction, 0.8, "the MCP's base without a config file");
    homes.config("[channel]\nspam_warn_rate = 3\n\n[supervisor]\ncpu_warn_fraction = 0.5\n");
    let loaded = homes.load().expect("the config loads");
    assert_eq!(loaded.supervisor.cpu_warn_fraction, 0.5, "the file's line over the base's");
    assert_eq!(loaded.supervisor.ram_warn_fraction, 0.8, "the base's where the file sets none");
}

#[test]
fn a_bad_workdir_or_supervisor_table_is_refused() {
    let homes = Scratch::new();
    let kind = |workdir: PathBuf| {
        Config::load(uid(), "fixture", "default", &workdir, &homes.env).expect_err("refused").kind().name()
    };
    assert_eq!(kind(PathBuf::from("work")), "config::workdir_invalid", "a relative workdir");
    assert_eq!(kind(homes.path("gone")), "config::workdir_invalid", "a missing workdir");
    for text in ["[supervisor]\ncpu_warn_fraction = 2.0\n", "[supervisor]\ncpu_warn = 0.5\n", "[supervisor\n"] {
        homes.config(text);
        assert_eq!(homes.load().expect_err(text).kind().name(), "config::file_invalid", "{text:?}");
    }
}
