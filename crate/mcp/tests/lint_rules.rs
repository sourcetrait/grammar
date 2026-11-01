//! The lint's rules, held to definitions through the `guts` surface.
//!
//! Integration rather than unit: each call builds a full-shell `ParseEngine`,
//! its plugin registry read off disk, so it rides the resource caveat.
//! `lint_definition` lints a definition whatever its parse, since a `use` or
//! `source` path that resolves nowhere fails the parse; `refused` and
//! `accepted` hold a definition to the contract first, so a case that fails to
//! parse can neither pass nor trip. `tests/body_lint.rs` covers the same lint
//! at the `execute` tool's boundary.

use sourcetrait_grammar_mcp::guts::{NuVal, analyze_definition, lint_definition, nuon};

/// A definition around `body`, its data a record of `fields`.
fn definition(fields: &str, body: &str) -> String {
    format!("def execute [data: record<{fields}>]: nothing -> record<out: int> {{\n{body}\n}}")
}

/// The rows the contract refuses a definition around `body` with.
fn refused(fields: &str, body: &str) -> Vec<NuVal> {
    match analyze_definition(&definition(fields, body)) {
        Ok(summary) => panic!("accepted `{body}` as {summary}"),
        Err(rows) => rows,
    }
}

/// Hold a definition around `body` to the contract, which accepts it.
fn accepted(fields: &str, body: &str) {
    if let Err(rows) = analyze_definition(&definition(fields, body)) {
        panic!("refused `{body}`: {rows:?}");
    }
}

fn lint(body: &str) -> Vec<NuVal> {
    lint_definition(&definition("noop: int", body))
}

fn lint_data(fields: &str, body: &str) -> Vec<NuVal> {
    lint_definition(&definition(fields, body))
}

fn kinds(v: &[NuVal]) -> Vec<String> {
    v.iter().map(|d| d["kind"].as_str().expect("kind").to_string()).collect()
}

fn is_hardcoded(d: &NuVal) -> bool {
    d["kind"] == "lint::hardcoded_variable"
}

fn is_denied(d: &NuVal) -> bool {
    d["kind"] == "lint::denied_command"
}

#[test]
fn passes_clean_record_return() {
    let v = lint("{ out: ($data.noop + 1) }");
    assert!(v.is_empty(), "expected no violations, got {v:?}");
}

#[test]
fn flags_quoted_abs_path() {
    let v = lint("cd \"/srv/proj/x\"; { out: 0 }");
    assert_eq!(v.len(), 1, "got {v:?}");
    assert_eq!(kinds(&v), vec!["lint::hardcoded_variable"]);
}

#[test]
fn flags_bare_abs_path() {
    let v = lint("cd /srv/proj/x; { out: 0 }");
    assert_eq!(v.len(), 1, "got {v:?}");
    assert_eq!(kinds(&v), vec!["lint::hardcoded_variable"]);
}

#[test]
fn passes_allowlist_etc() {
    assert!(lint("cd /etc/foo; { out: 0 }").is_empty());
    assert!(lint("cd \"/etc/foo\"; { out: 0 }").is_empty());
}

#[test]
fn passes_allowlist_dev_proc() {
    assert!(lint("cd /dev/null; { out: 0 }").is_empty());
    assert!(lint("cd \"/proc/self\"; { out: 0 }").is_empty());
}

#[test]
fn flags_tilde_path() {
    let v = lint("cd ~/proj/x; { out: 0 }");
    assert_eq!(v.len(), 1, "got {v:?}");
    assert_eq!(kinds(&v), vec!["lint::hardcoded_variable"]);
}

#[test]
fn flags_relative_path() {
    let v = lint("cd ./relative/x; { out: 0 }");
    assert_eq!(v.len(), 1, "got {v:?}");
    assert_eq!(kinds(&v), vec!["lint::hardcoded_variable"]);
}

#[test]
fn flags_parent_relative_path() {
    let v = lint("cd ../up/x; { out: 0 }");
    assert_eq!(v.len(), 1, "got {v:?}");
    assert_eq!(kinds(&v), vec!["lint::hardcoded_variable"]);
}

#[test]
fn passes_var_path() {
    let v = lint_data("path: string", "cd $data.path; { out: 0 }");
    assert!(v.is_empty(), "got {v:?}");
}

#[test]
fn passes_url() {
    let v = lint("http get \"https://api.example.com\"; { out: 0 }");
    assert!(v.is_empty(), "got {v:?}");
}

#[test]
fn flags_denied_external() {
    let v = lint("^awk '{print $1}'; { out: 0 }");
    assert_eq!(v.len(), 1, "got {v:?}");
    assert_eq!(kinds(&v), vec!["lint::denied_command"]);
}

#[test]
fn passes_allowed_external() {
    let v = lint("^git status; { out: 0 }");
    assert!(v.is_empty(), "got {v:?}");
}

#[test]
fn passes_var_headed_external() {
    let v = lint_data("cmd: string", "^$data.cmd status; { out: 0 }");
    assert!(v.is_empty(), "got {v:?}");
}

#[test]
fn passes_regex_op_match() {
    let v = lint("if (\"abc\" =~ '/foo/bar/') { { out: 1 } } else { { out: 0 } }");
    assert!(v.is_empty(), "got {v:?}");
}

#[test]
fn passes_regex_op_nomatch() {
    let v = lint("if (\"abc\" !~ '/foo/bar/') { { out: 1 } } else { { out: 0 } }");
    assert!(v.is_empty(), "got {v:?}");
}

#[test]
fn passes_regex_named_flag_str_replace() {
    let v = lint("let x = (\"abc\" | str replace --regex '/x/y/' ''); { out: 0 }");
    assert!(v.is_empty(), "got {v:?}");
}

#[test]
fn passes_regex_named_flag_parse() {
    let v = lint("let x = (\"a/b/c\" | parse --regex '(?<g>.+/.+)'); { out: 0 }");
    assert!(v.is_empty(), "got {v:?}");
}

#[test]
fn passes_source_path() {
    let v = lint("source \"/srv/lib/util.nu\"; { out: 0 }");
    assert!(v.is_empty(), "got {v:?}");
}

#[test]
fn passes_source_env_path() {
    let v = lint("source-env \"/srv/lib/env.nu\"; { out: 0 }");
    assert!(v.is_empty(), "got {v:?}");
}

#[test]
fn passes_use_and_overlay_paths() {
    let u = lint("use ./helpers/util.nu; { out: 0 }");
    assert!(u.is_empty(), "got {u:?}");
    let o = lint("overlay use ./helpers/util.nu; { out: 0 }");
    assert!(o.is_empty(), "got {o:?}");
}

#[test]
fn passes_bare_glob() {
    let v = lint("ls *.md; { out: 0 }");
    assert!(v.is_empty(), "got {v:?}");
}

#[test]
fn passes_division() {
    let v = lint("let x = (1 / 2); { out: 0 }");
    assert!(v.is_empty(), "got {v:?}");
}

#[test]
fn flags_nested_def_body() {
    let v = lint("def helper [a] { cd \"/x/y\"; 1 }; { out: 0 }");
    assert_eq!(v.len(), 1, "got {v:?}");
    assert_eq!(kinds(&v), vec!["lint::hardcoded_variable"]);
}

#[test]
fn flags_interpolation_behind_hardcoded_root() {
    let v = refused("noop: int", "cd $\"/home/($env.USER)/proj/x\"; { out: 0 }");
    assert_eq!(kinds(&v), vec!["lint::hardcoded_variable"], "the leading literal alone; got {v:?}");
    assert_eq!(v[0]["source"]["position"], nuon("[2, 6]"), "at `/home/`; got {v:?}");
}

#[test]
fn flags_leading_literal_path() {
    let v = refused("name: string", "let m = $\"/srv/proj missing: ($data.name)\"; { out: 0 }");
    assert_eq!(kinds(&v), vec!["lint::hardcoded_variable"], "got {v:?}");
}

#[test]
fn flags_bare_prefix_before_interpolation() {
    for body in ["let p = $'~/($data.rest)'; { out: 0 }", "let p = $'/($data.rest)'; { out: 0 }"] {
        let v = refused("rest: string", body);
        assert_eq!(kinds(&v), vec!["lint::hardcoded_variable"], "`{body}` got {v:?}");
    }
}

#[test]
fn flags_bare_prefix_path() {
    for body in ["cd /; { out: 0 }", "cd ~/; { out: 0 }"] {
        let v = refused("noop: int", body);
        assert_eq!(kinds(&v), vec!["lint::hardcoded_variable"], "`{body}` got {v:?}");
    }
}

#[test]
fn passes_literal_after_interpolation() {
    accepted("root: string", "let p = $'($data.root)/x'; { out: 0 }");
    accepted("pid: int", "let p = $'/proc/($data.pid)/status'; { out: 0 }");
}

#[test]
fn passes_untrimmed_separator() {
    accepted("noop: int", "let s = ([a b] | str join ' / '); { out: 0 }");
}

#[test]
fn passes_bare_prefix_text() {
    for prefix in ["/", "./", "../", "~/"] {
        accepted("noop: int", &format!("let s = ([a b] | str join '{prefix}'); {{ out: 0 }}"));
    }
}

#[test]
fn passes_document_text() {
    accepted(
        "name: string",
        "let d = $\"Detail: `plan/agenda/($data.name)/<Task>.md`\"; { out: 0 }",
    );
}

#[test]
fn passes_comment_marker() {
    accepted("noop: int", "let m = ['///' '//!' '//']; { out: 0 }");
}

#[test]
fn cap_at_three() {
    let body = "\
^awk 'x'
cd \"/a/b\"
^grep foo
cd ~/y
{ out: 0 }";
    let v = lint(body);
    assert_eq!(v.len(), 3, "got {v:?}");
    assert_eq!(
        kinds(&v),
        vec![
            "lint::denied_command",
            "lint::hardcoded_variable",
            "lint::denied_command",
        ]
    );
}

#[test]
fn under_cap_all_surface() {
    let body = "\
^awk 'x'
cd \"/a/b\"
^grep foo
{ out: 0 }";
    let v = lint(body);
    assert_eq!(v.len(), 3, "got {v:?}");
    assert_eq!(
        kinds(&v),
        vec![
            "lint::denied_command",
            "lint::hardcoded_variable",
            "lint::denied_command",
        ]
    );
}

#[test]
fn empty_body_passes() {
    let v = lint("");
    assert!(v.is_empty(), "got {v:?}");
}

#[test]
fn record_field_paths_flagged() {
    let v = lint("{ p: \"/srv/proj\", out: 0 }");
    assert_eq!(v.len(), 1, "got {v:?}");
    assert_eq!(kinds(&v), vec!["lint::hardcoded_variable"]);
}

#[test]
fn list_items_walked() {
    let v = lint("let xs = [\"/a/b\" \"/c/d\"]; { out: 0 }");
    assert_eq!(v.len(), 2, "got {v:?}");
}

#[test]
fn line_col_within_definition() {
    let v = lint("cd \"/a/b\"; { out: 0 }");
    assert_eq!(v.len(), 1, "got {v:?}");
    assert_eq!(
        v[0]["source"]["position"],
        nuon("[2, 4]"),
        "the position is the definition's own line and column; got {:?}",
        v[0],
    );
    assert!(
        v[0]["source"]["path"].is_nothing(),
        "a definition's diagnostic has no file; got {:?}",
        v[0]["source"]["path"],
    );
    assert_eq!(v[0]["kind"], "lint::hardcoded_variable");
}

#[test]
fn flags_single_segment_abs_path() {
    let v = lint("cd \"/tmp\"; { out: 0 }");
    assert_eq!(v.len(), 1, "got {v:?}");
    assert_eq!(kinds(&v), vec!["lint::hardcoded_variable"]);
}

#[test]
fn flags_single_segment_abs_path_bare() {
    let v = lint("cd /tmp; { out: 0 }");
    assert_eq!(v.len(), 1, "got {v:?}");
    assert_eq!(kinds(&v), vec!["lint::hardcoded_variable"]);
}

#[test]
fn allowlist_still_passes_under_strict_rule() {
    let with_slash = lint("cd /etc/; { out: 0 }");
    assert!(with_slash.is_empty(), "got {with_slash:?}");
    let without_slash = lint("cd /etc; { out: 0 }");
    assert_eq!(without_slash.len(), 1, "got {without_slash:?}");
}

#[test]
fn flags_abs_path_external_head_denylist() {
    let v = lint("^/usr/bin/awk 'x'; { out: 0 }");
    let denylist_count = v.iter().filter(|d| is_denied(d)).count();
    let path_count = v.iter().filter(|d| is_hardcoded(d)).count();
    assert!(denylist_count >= 1, "no denylist fired; got {v:?}");
    assert!(path_count >= 1, "no path fired; got {v:?}");
}

#[test]
fn flags_replacement_in_str_replace_regex() {
    let v = lint("let x = (\"abc\" | str replace --regex '/x/' '/replacement/path'); { out: 0 }");
    assert_eq!(v.len(), 1, "got {v:?}");
    assert_eq!(kinds(&v), vec!["lint::hardcoded_variable"]);
}

#[test]
fn walks_table_cell_paths() {
    let v = lint("[[c1 c2]; [\"/a/b\" 1] [2 \"/c/d\"]]; { out: 0 }");
    assert_eq!(v.len(), 2, "got {v:?}");
    assert!(v.iter().all(is_hardcoded));
}

#[test]
fn walks_match_arm_body_paths() {
    let v = lint("match $data.noop { 0 => { cd \"/a/b\" } _ => { 0 } }; { out: 0 }");
    assert_eq!(v.len(), 1, "got {v:?}");
    assert_eq!(kinds(&v), vec!["lint::hardcoded_variable"]);
}

#[test]
fn walks_where_row_condition_paths() {
    let v = lint("[{p: \"x\"}] | where p == \"/a/b\"; { out: 0 }");
    assert!(v.iter().any(is_hardcoded), "got {v:?}");
}

#[test]
fn walks_range_bounds() {
    let v = lint("let r = (cd \"/a/b\"; 1)..5; { out: 0 }");
    assert!(v.iter().any(is_hardcoded), "got {v:?}");
}

#[test]
fn passes_allowlist_sys() {
    assert!(lint("cd /sys/class/net; { out: 0 }").is_empty());
    assert!(lint("cd \"/sys/class/net\"; { out: 0 }").is_empty());
}

#[test]
fn flags_run_path() {
    let v = lint("cd /run/foo; { out: 0 }");
    assert_eq!(v.len(), 1, "got {v:?}");
    assert_eq!(kinds(&v), vec!["lint::hardcoded_variable"]);
}

#[test]
fn passes_regex_named_flag_split_list() {
    let v = lint("let xs = ([\"a/b\" \"c\"] | split list --regex '/'); { out: 0 }");
    assert!(v.is_empty(), "got {v:?}");
}

#[test]
fn flags_split_list_without_regex() {
    let v = lint("let xs = ([\"x\"] | split list \"/a/b\"); { out: 0 }");
    assert!(v.iter().any(is_hardcoded), "got {v:?}");
}
