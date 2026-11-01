use crate::*;

fn error_count(diagnostics: &[Diagnostic]) -> usize {
    diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count()
}

fn cap_diagnostics(diagnostics: Vec<Diagnostic>, cap: usize) -> Vec<Diagnostic> {
    let mut errors = 0usize;
    let mut warnings = 0usize;
    let mut out = Vec::new();
    for d in diagnostics {
        match d.severity {
            Severity::Error => {
                if errors < cap {
                    out.push(d);
                    errors += 1;
                }
            }
            Severity::Warning => {
                if warnings < cap {
                    out.push(d);
                    warnings += 1;
                }
            }
        }
    }
    out
}

fn prefix_diagnostic_paths(result: &mut ValidationResult, rig: &str) {
    for d in &mut result.diagnostics {
        if let Some(src) = &mut d.source
            && let Some(path) = &src.path
        {
            src.path = Some(format!("{rig}/{path}"));
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ValidationResult {
    pub diagnostics: Vec<Diagnostic>,
    pub functions: Vec<IndexFunction>,
    pub modules: Vec<IndexModule>,
    pub docs: Vec<DocEntry>,
}

#[derive(Debug, Clone)]
pub struct DocEntry {
    pub docs_path: String,
    pub summary: String,
    pub details: String,
}

impl ValidationResult {
    pub fn is_empty(&self) -> bool {
        error_count(&self.diagnostics) == 0
    }
}

const SUMMARY_MAX_CHARS: usize = 80;

fn extract_doc(source: &str, marker: Option<&str>) -> (String, String, usize) {
    let lines: Vec<&str> = source.lines().collect();
    let doc_idxs: Vec<usize> = match marker {
        None => {
            let mut idxs = Vec::new();
            for (i, l) in lines.iter().enumerate() {
                if l.trim_start().starts_with('#') {
                    idxs.push(i);
                } else {
                    break;
                }
            }
            idxs
        }
        Some(m) => match lines.iter().position(|l| l.trim_start().starts_with(m)) {
            None => Vec::new(),
            Some(decl) => {
                let mut idxs = Vec::new();
                let mut i = decl;
                while i > 0 {
                    i -= 1;
                    if lines[i].trim_start().starts_with('#') {
                        idxs.push(i);
                    } else {
                        break;
                    }
                }
                idxs.reverse();
                idxs
            }
        },
    };
    if doc_idxs.is_empty() {
        return (String::new(), String::new(), 0);
    }
    let stripped: Vec<String> = doc_idxs
        .iter()
        .map(|&i| {
            let t = lines[i].trim_start();
            let t = t.strip_prefix('#').unwrap_or(t);
            t.strip_prefix(' ').unwrap_or(t).to_string()
        })
        .collect();
    let joined = stripped.join("\n");
    let (summary, details) = match joined.split_once("\n\n") {
        Some((s, d)) => (s.to_string(), d.to_string()),
        None => (joined, String::new()),
    };
    (summary, details, doc_idxs[0] + 1)
}

fn check_summary_length(
    rel: &str,
    source: &str,
    marker: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (summary, _details, line) = extract_doc(source, marker);
    if summary.chars().count() > SUMMARY_MAX_CHARS {
        diagnostics.push(Diagnostic::warning(
            "lint::summary_length",
            Some(Source {
                path: Some(rel.to_string()),
                position: [wide(line), 1],
            }),
            "doc summary line exceeds 80 characters",
        ));
    }
}


pub(crate) const NAME_DENYLIST: &[&str] = &["docs", "tools", "bin", "target"];

/// The words `interface help` answers itself, held from an equip's author and name.
pub(crate) const HELP_WORDS: &[&str] = &["interface", GRIMM];

const ASSET_EXT_DENYLIST: &[&str] = &[
    "nu", "sh", "bash", "zsh", "fish", "ksh", "py", "rb", "pl", "js", "ts", "lua", "ps1", "bat",
    "cmd", "com", "exe",
];

#[derive(Copy, Clone)]
enum Zone {
    Source,
    Doc,
    Asset,
    Lib,
}

impl Zone {
    fn ext_kind(self) -> &'static str {
        match self {
            Zone::Source => "rig::source_extension_denied",
            Zone::Doc => "rig::doc_extension_denied",
            Zone::Asset => "rig::asset_extension_denied",
            Zone::Lib => "lib::extension_denied",
        }
    }

    fn exec_kind(self) -> &'static str {
        match self {
            Zone::Source => "rig::source_executable_denied",
            Zone::Doc => "rig::doc_executable_denied",
            Zone::Asset => "rig::asset_executable_denied",
            Zone::Lib => "lib::executable_denied",
        }
    }
}

/// Whether a file runs as a program: any execute bit set.
#[cfg(unix)]
fn is_executable(path: &std::path::Path) -> bool {
    fs::metadata(path)
        .map(|m| std::os::unix::fs::PermissionsExt::mode(&m.permissions()) & 0o111 != 0)
        .unwrap_or(false)
}

/// Whether a file runs as a program: its extension is one `PATHEXT` names.
#[cfg(not(unix))]
fn is_executable(path: &std::path::Path) -> bool {
    /// `PATHEXT`'s value where the environment holds none.
    const DEFAULT_PATHEXT: &str = ".COM;.EXE;.BAT;.CMD";
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| DEFAULT_PATHEXT.to_string());
    pathext
        .split(';')
        .any(|known| known.trim_start_matches('.').eq_ignore_ascii_case(ext))
}

fn ext_lower(path: &std::path::Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
}

fn is_sanctioned_root_file(name: &str) -> bool {
    matches!(
        name,
        "rig.toml" | "README.md" | "LEGAL.md" | "LICENSE.txt"
    ) || (name.starts_with("LICENSE-") && name.ends_with(".txt"))
}

fn push_ext_denied(
    root: &std::path::Path,
    path: &std::path::Path,
    zone: Zone,
    message: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned();
    diagnostics.push(Diagnostic::error(
        zone.ext_kind(),
        Some(Source {
            path: Some(rel),
            position: [0, 0],
        }),
        message.to_string(),
    ));
}

fn check_not_executable(
    root: &std::path::Path,
    path: &std::path::Path,
    zone: Zone,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !is_executable(path) {
        return;
    }
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned();
    diagnostics.push(Diagnostic::error(
        zone.exec_kind(),
        Some(Source {
            path: Some(rel),
            position: [0, 0],
        }),
        "equip files must not be executable (+x); clear the executable bit",
    ));
}

fn validate_assets_tree(
    root: &std::path::Path,
    dir: &std::path::Path,
    result: &mut ValidationResult,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        if error_count(&result.diagnostics) > LINT_VIOLATION_CAP {
            return Ok(());
        }
        let entry = entry?;
        let path = entry.path();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            validate_assets_tree(root, &path, result)?;
        } else if ft.is_file() {
            check_not_executable(root, &path, Zone::Asset, &mut result.diagnostics);
            if let Some(ext) = ext_lower(&path)
                && ASSET_EXT_DENYLIST.contains(&ext.as_str())
            {
                push_ext_denied(
                    root,
                    &path,
                    Zone::Asset,
                    ".assets/ must not contain executable/script file types (.sh, .py, .nu, ...)",
                    &mut result.diagnostics,
                );
            }
        }
    }
    Ok(())
}

fn validate_docs_tree(
    root: &std::path::Path,
    dir: &std::path::Path,
    result: &mut ValidationResult,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        if error_count(&result.diagnostics) > LINT_VIOLATION_CAP {
            return Ok(());
        }
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            validate_docs_tree(root, &path, result)?;
        } else if ft.is_file() {
            check_not_executable(root, &path, Zone::Doc, &mut result.diagnostics);
            let allowed = name == ".gitignore"
                || matches!(ext_lower(&path).as_deref(), Some("md") | Some("txt"));
            if !allowed {
                push_ext_denied(
                    root,
                    &path,
                    Zone::Doc,
                    ".docs/ accepts only .md and .txt files (and .gitignore)",
                    &mut result.diagnostics,
                );
            }
        }
    }
    Ok(())
}

struct SelfView {
    base: PathBuf,
}

static SELF_VIEW_SEQ: AtomicU64 = AtomicU64::new(0);

impl SelfView {
    fn new(equip: &str, source_dir: &Path, kinds: &[&str]) -> Option<Self> {
        if !is_valid_equip(equip) {
            return None;
        }
        let base = std::env::temp_dir().join(format!(
            "grammar_selfview_{}_{}",
            process::id(),
            SELF_VIEW_SEQ.fetch_add(1, Ordering::Relaxed),
        ));
        let view = Self { base };
        for kind in kinds {
            let link = view.base.join(EQUIP_DIR).join(equip).join(kind);
            fs::create_dir_all(link.parent()?).ok()?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(source_dir.join(kind), &link).ok()?;
            #[cfg(windows)]
            std::os::windows::fs::symlink_dir(source_dir.join(kind), &link).ok()?;
        }
        Some(view)
    }
}

impl Drop for SelfView {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

/// Validate an equip's source kinds against one self-view, building its index.
pub(crate) fn validate_equip_source(
    equip: &str,
    source_dir: &Path,
    kinds: &[&str],
    engine: &ParseEngine,
) -> io::Result<ValidationResult> {
    let mut result = ValidationResult::default();
    let self_view = SelfView::new(equip, source_dir, kinds);
    let scoped;
    let walk_engine = match &self_view {
        Some(view) => {
            scoped = engine.with_extra_lib_dir(view.base.clone());
            &scoped
        }
        None => engine,
    };
    for kind in kinds {
        let root = source_dir.join(kind);
        let mut found = ValidationResult::default();
        if *kind == KIND_LIB {
            validate_lib_tree(&root, &root, walk_engine, &mut found)?;
        } else {
            let (modules, functions) = validate_walk(&root, &root, walk_engine, "", &mut found)?;
            found.modules = modules;
            found.functions = functions;
        }
        prefix_diagnostic_paths(&mut found, &format!("{equip}/{kind}"));
        result.diagnostics.append(&mut found.diagnostics);
        if *kind == KIND_RIG {
            result.functions = found.functions;
            result.modules = found.modules;
            result.docs = found.docs;
        }
    }
    result.diagnostics = cap_diagnostics(result.diagnostics, LINT_VIOLATION_CAP);
    Ok(result)
}

/// Walk a lib: `.nu` files alone, none executable, each parsed by the engine.
fn validate_lib_tree(
    root: &Path,
    dir: &Path,
    engine: &ParseEngine,
    result: &mut ValidationResult,
) -> io::Result<()> {
    let mut entries: Vec<fs::DirEntry> = fs::read_dir(dir)?.collect::<io::Result<_>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        if error_count(&result.diagnostics) > LINT_VIOLATION_CAP {
            return Ok(());
        }
        let path = entry.path();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            validate_lib_tree(root, &path, engine, result)?;
        } else if ft.is_file() {
            check_not_executable(root, &path, Zone::Lib, &mut result.diagnostics);
            if ext_lower(&path).as_deref() == Some("nu") {
                validate_lib_file(root, &path, engine, result)?;
            } else {
                push_ext_denied(
                    root,
                    &path,
                    Zone::Lib,
                    "a lib holds .nu files alone",
                    &mut result.diagnostics,
                );
            }
        }
    }
    Ok(())
}

/// Parse one lib file as a module; its import cycle or parse errors are findings.
fn validate_lib_file(
    root: &Path,
    path: &Path,
    engine: &ParseEngine,
    result: &mut ValidationResult,
) -> io::Result<()> {
    let source = fs::read_to_string(path)?;
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned();
    let fname = path.to_string_lossy().into_owned();
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("lib");
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let wrapper_name = format!("__lib_{stem}");
    let (wrapped, prefix_len) = wrap_as_module(&source, &wrapper_name);
    let engine_state = engine.engine_state_for_file(parent);
    let mut ws = nu::StateWorkingSet::new(&engine_state);
    let _ = ws.files.push(PathBuf::from(&fname), nu::Span::unknown());
    let files_before = ws.num_files();
    let _ = nu::parse(&mut ws, Some(&fname), wrapped.as_bytes(), false);
    if !ws.parse_errors.is_empty()
        && let Some(cycle) = detect_import_cycle(&ws, files_before)
    {
        result.diagnostics.push(Diagnostic::error(
            "module::circular_import",
            Some(Source {
                path: Some(rel),
                position: [0, 0],
            }),
            cycle,
        ));
        return Ok(());
    }
    let source_start = file_start(&ws, &fname) + prefix_len;
    for err in &ws.parse_errors {
        let span_start = err.span().start.saturating_sub(source_start);
        let (line, _col) = span_to_line_col(&source, span_start);
        result.diagnostics.push(Diagnostic::error(
            "lib::parse_error",
            Some(Source {
                path: Some(rel.clone()),
                position: [wide(line), 0],
            }),
            format!("parse error: {}", nuval::NuValError::from(err).short()),
        ));
    }
    Ok(())
}

fn validate_walk(
    root: &std::path::Path,
    dir: &std::path::Path,
    engine: &ParseEngine,
    module_path: &str,
    result: &mut ValidationResult,
) -> io::Result<(Vec<IndexModule>, Vec<IndexFunction>)> {
    let modnu = dir.join("mod.nu");
    let modnu_src = if modnu.exists() {
        fs::read_to_string(&modnu).unwrap_or_default()
    } else {
        String::new()
    };
    if modnu.exists() {
        let (summary, details, _) = extract_doc(&modnu_src, None);
        if !summary.is_empty() || !details.is_empty() {
            result.docs.push(DocEntry {
                docs_path: module_path.to_string(),
                summary,
                details,
            });
        }
        let rel = modnu
            .strip_prefix(root)
            .unwrap_or(&modnu)
            .to_string_lossy()
            .into_owned();
        let fname = modnu.to_string_lossy().into_owned();
        if module_path.is_empty()
            && parse_module_has_main(&modnu, "root", &modnu_src, dir, engine)
        {
            result.diagnostics.push(Diagnostic::error(
                "rig::root_function",
                Some(Source {
                    path: Some(rel.clone()),
                    position: [0, 0],
                }),
                "a call-target cannot live at the rig root; move it into a module",
            ));
        }
        validate_mod_nu_ast(&rel, &fname, "mod", &modnu_src, dir, engine, &mut result.diagnostics);
        scan_reserved_terms(&rel, &fname, "mod", &modnu_src, dir, engine, &mut result.diagnostics);
    }
    let edges = extract_module_edges(&modnu_src, &modnu, dir, engine);

    let is_root = module_path.is_empty();
    let mut dirs: Vec<(String, PathBuf)> = Vec::new();
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            if is_root && name == ".assets" {
                validate_assets_tree(root, &path, result)?;
            } else if is_root && name == ".docs" {
                validate_docs_tree(root, &path, result)?;
            } else if name.starts_with('.') {
            } else {
                dirs.push((name, path));
            }
        } else if ft.is_file() {
            let is_nu = path.extension().map(|e| e == "nu").unwrap_or(false);
            if is_nu {
                check_not_executable(root, &path, Zone::Source, &mut result.diagnostics);
                files.push((name, path));
            } else if name == ".gitignore" {
                check_not_executable(root, &path, Zone::Source, &mut result.diagnostics);
            } else if name.starts_with('.') {
            } else {
                check_not_executable(root, &path, Zone::Source, &mut result.diagnostics);
                if !(is_root && is_sanctioned_root_file(&name)) {
                    push_ext_denied(
                        root,
                        &path,
                        Zone::Source,
                        "only .nu files are allowed in the rig tree (plus root README.md / LEGAL.md / LICENSE.txt / LICENSE-*.txt / rig.toml, .gitignore, and the .assets/ + .docs/ dirs)",
                        &mut result.diagnostics,
                    );
                }
            }
        }
    }
    dirs.sort();
    files.sort();

    let mut functions: Vec<IndexFunction> = Vec::new();
    let mut modules: Vec<IndexModule> = Vec::new();

    for (name, path) in &files {
        if name == "mod.nu" {
            continue;
        }
        if error_count(&result.diagnostics) > LINT_VIOLATION_CAP {
            return Ok((modules, functions));
        }
        validate_flat_file(root, path, engine, result)?;
        let stem = name.trim_end_matches(".nu");
        if !edges.iter().any(|(_, n)| n == stem) {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .into_owned();
            result.diagnostics.push(Diagnostic::error(
                "rig::orphan",
                Some(Source {
                    path: Some(rel),
                    position: [0, 0],
                }),
                format!(
                    "`{name}` is not reached through the module graph; `export use`/`use` it from mod.nu or remove it",
                ),
            ));
        }
    }

    for (name, path) in &dirs {
        if error_count(&result.diagnostics) > LINT_VIOLATION_CAP {
            return Ok((modules, functions));
        }
        let child_modnu = path.join("mod.nu");
        if !child_modnu.exists() {
            continue;
        }
        let child_src = fs::read_to_string(&child_modnu).unwrap_or_default();
        let rel = child_modnu
            .strip_prefix(root)
            .unwrap_or(&child_modnu)
            .to_string_lossy()
            .into_owned();
        let fname = child_modnu.to_string_lossy().into_owned();
        let edge_kinds: Vec<EdgeKind> = edges
            .iter()
            .filter(|(_, n)| n == name)
            .map(|(k, _)| *k)
            .collect();
        let child_has_main = parse_module_has_main(&child_modnu, name, &child_src, path, engine);

        if child_has_main {
            if module_path.is_empty() {
                result.diagnostics.push(Diagnostic::error(
                    "rig::root_function",
                    Some(Source {
                        path: Some(rel),
                        position: [0, 0],
                    }),
                    "a call-target cannot live at the rig root; move it into a module",
                ));
                continue;
            }
            check_not_executable(root, &child_modnu, Zone::Source, &mut result.diagnostics);
            if edge_kinds.is_empty() {
                result.diagnostics.push(Diagnostic::error(
                    "rig::orphan",
                    Some(Source {
                        path: Some(rel.clone()),
                        position: [0, 0],
                    }),
                    format!("call `{name}` is not wired into mod.nu; add `export use {name}`"),
                ));
            } else if !edge_kinds.contains(&EdgeKind::ExportUse) {
                result.diagnostics.push(Diagnostic::error(
                    "rig::call_wiring",
                    Some(Source {
                        path: Some(rel.clone()),
                        position: [0, 0],
                    }),
                    format!(
                        "a call must be wired into its parent via `export use {name}`, not `export module`",
                    ),
                ));
            }
            if has_child_module_dir(path) {
                result.diagnostics.push(Diagnostic::error(
                    "rig::call_leaf",
                    Some(Source {
                        path: Some(rel.clone()),
                        position: [0, 0],
                    }),
                    format!(
                        "a call target is an edge module and cannot contain submodules; `{name}` has one",
                    ),
                ));
            }
            let extracted = validate_function_file_ast(
                &rel,
                &fname,
                name,
                &child_src,
                path,
                engine,
                &mut result.diagnostics,
            );
            scan_reserved_terms(&rel, &fname, name, &child_src, path, engine, &mut result.diagnostics);
            if let Some((idx_fn, summary, details)) = extracted {
                let docs_path = format!("{module_path}/{name}");
                if !summary.is_empty() || !details.is_empty() {
                    result.docs.push(DocEntry {
                        docs_path,
                        summary,
                        details,
                    });
                }
                functions.push(idx_fn);
            }
        } else {
            if edge_kinds.is_empty() {
                result.diagnostics.push(Diagnostic::error(
                    "rig::orphan",
                    Some(Source {
                        path: Some(rel),
                        position: [0, 0],
                    }),
                    format!("`{name}` is not wired into mod.nu; add `export module {name}` (or remove it)"),
                ));
            }
            let child_path = if module_path.is_empty() {
                name.clone()
            } else {
                format!("{module_path}/{name}")
            };
            let (sub_mods, sub_fns) = validate_walk(root, path, engine, &child_path, result)?;
            if !sub_fns.is_empty() || !sub_mods.is_empty() {
                modules.push(IndexModule {
                    name: name.clone(),
                    functions: sub_fns,
                    modules: sub_mods,
                });
            }
        }
    }

    Ok((modules, functions))
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum EdgeKind {
    ExportModule,
    ExportUse,
    Use,
}

fn parse_module_has_main(
    modnu: &std::path::Path,
    name: &str,
    source: &str,
    parent: &std::path::Path,
    engine: &ParseEngine,
) -> bool {
    let fname = modnu.to_string_lossy().into_owned();
    let wrapper_name = format!("__cls_{name}");
    let (wrapped, _) = wrap_as_module(source, &wrapper_name);
    let engine_state = engine.engine_state_for_file(parent);
    let mut ws = nu::StateWorkingSet::new(&engine_state);
    let _ = ws.files.push(PathBuf::from(&fname), nu::Span::unknown());
    let _ = nu::parse(&mut ws, Some(&fname), wrapped.as_bytes(), false);
    if !ws.parse_errors.is_empty() {
        return false;
    }
    match ws.find_module(wrapper_name.as_bytes()) {
        Some(id) => ws.get_module(id).main.is_some(),
        None => false,
    }
}

fn extract_module_edges(
    source: &str,
    modnu: &std::path::Path,
    parent: &std::path::Path,
    engine: &ParseEngine,
) -> Vec<(EdgeKind, String)> {
    let mut edges = Vec::new();
    if source.is_empty() {
        return edges;
    }
    let fname = modnu.to_string_lossy().into_owned();
    let wrapper_name = "__edges";
    let (wrapped, _) = wrap_as_module(source, wrapper_name);
    let engine_state = engine.engine_state_for_file(parent);
    let mut ws = nu::StateWorkingSet::new(&engine_state);
    let _ = ws.files.push(PathBuf::from(&fname), nu::Span::unknown());
    let block = nu::parse(&mut ws, Some(&fname), wrapped.as_bytes(), false);
    if !ws.parse_errors.is_empty() {
        return edges;
    }
    let body_id = block
        .pipelines
        .first()
        .and_then(|p| p.elements.first())
        .and_then(|elem| match &elem.expr.expr {
            nu::Expr::Call(call) => call.arguments.iter().find_map(|arg| {
                if let nu::Argument::Positional(e) = arg
                    && let nu::Expr::Block(id) = &e.expr
                {
                    return Some(*id);
                }
                None
            }),
            _ => None,
        });
    let Some(body_id) = body_id else {
        return edges;
    };
    let body = ws.get_block(body_id);
    for pipeline in &body.pipelines {
        for elem in &pipeline.elements {
            if let nu::Expr::Call(call) = &elem.expr.expr {
                let decl = ws.get_decl(call.decl_id);
                let kind = match decl.name() {
                    "export module" => EdgeKind::ExportModule,
                    "export use" => EdgeKind::ExportUse,
                    "use" | "overlay use" => EdgeKind::Use,
                    _ => continue,
                };
                if let Some(nu::Argument::Positional(e)) = call.arguments.first()
                    && let Some(n) = edge_name_from_expr(&e.expr)
                {
                    edges.push((kind, n));
                }
            }
        }
    }
    edges
}

fn edge_name_from_expr(expr: &nu::Expr) -> Option<String> {
    let raw = match expr {
        nu::Expr::String(s)
        | nu::Expr::RawString(s)
        | nu::Expr::GlobPattern(s, _)
        | nu::Expr::Filepath(s, _)
        | nu::Expr::Directory(s, _) => s.clone(),
        _ => return None,
    };
    let stem = std::path::Path::new(&raw)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&raw)
        .to_string();
    if stem.is_empty() { None } else { Some(stem) }
}

fn has_child_module_dir(dir: &std::path::Path) -> bool {
    let Ok(read) = fs::read_dir(dir) else {
        return false;
    };
    for entry in read.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        if entry.path().join("mod.nu").exists() {
            return true;
        }
    }
    false
}

fn validate_flat_file(
    root: &std::path::Path,
    path: &std::path::Path,
    engine: &ParseEngine,
    result: &mut ValidationResult,
) -> io::Result<()> {
    let source = fs::read_to_string(path)?;
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned();
    let fname = path.to_string_lossy().into_owned();
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new(""));
    let wrapper_name = format!("__of_{stem}");
    let (wrapped, prefix_len) = wrap_as_module(&source, &wrapper_name);
    let engine_state = engine.engine_state_for_file(parent);
    let mut ws = nu::StateWorkingSet::new(&engine_state);
    let _ = ws.files.push(PathBuf::from(&fname), nu::Span::unknown());
    let files_before = ws.num_files();
    let _ = nu::parse(&mut ws, Some(&fname), wrapped.as_bytes(), false);
    if !ws.parse_errors.is_empty()
        && let Some(cycle) = detect_import_cycle(&ws, files_before)
    {
        result.diagnostics.push(Diagnostic::error(
            "module::circular_import",
            Some(Source {
                path: Some(rel.clone()),
                position: [0, 0],
            }),
            cycle,
        ));
        return Ok(());
    }
    let source_start = file_start(&ws, &fname) + prefix_len;
    for err in &ws.parse_errors {
        let span_start = err.span().start.saturating_sub(source_start);
        let (line, _col) = span_to_line_col(&source, span_start);
        result.diagnostics.push(Diagnostic::error(
            "rig::parse_error",
            Some(Source {
                path: Some(rel.clone()),
                position: [wide(line), 0],
            }),
            format!("parse error: {}", nuval::NuValError::from(err).short()),
        ));
    }
    if ws.parse_errors.is_empty()
        && let Some(id) = ws.find_module(wrapper_name.as_bytes())
        && ws.get_module(id).main.is_some()
    {
        result.diagnostics.push(Diagnostic::error(
            "rig::main_in_flat_file",
            Some(Source {
                path: Some(rel.clone()),
                position: [0, 0],
            }),
            "`export def main` is reserved for a call target and must live in `<call>/mod.nu`, not a flat file",
        ));
    }
    scan_reserved_terms(&rel, &fname, stem, &source, parent, engine, &mut result.diagnostics);
    Ok(())
}

fn scan_reserved_terms(
    rel: &str,
    fname: &str,
    stem: &str,
    source: &str,
    parent: &std::path::Path,
    engine: &ParseEngine,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for comp in rel.split('/') {
        let bare = comp.strip_suffix(".nu").unwrap_or(comp);
        if is_reserved_term(bare) {
            diagnostics.push(Diagnostic::error(
                "rig::reserved",
                Some(Source {
                    path: Some(rel.to_string()),
                    position: [0, 0],
                }),
                format!(
                    "`{bare}` is reserved (the call-target sentinel) and cannot name a module, directory, or file",
                ),
            ));
        }
    }

    let wrapper_name = format!("__rt_{stem}");
    let (wrapped, prefix_len) = wrap_as_module(source, &wrapper_name);
    let engine_state = engine.engine_state_for_file(parent);
    let mut working_set = nu::StateWorkingSet::new(&engine_state);
    let _ = working_set.files.push(PathBuf::from(fname), nu::Span::unknown());
    let block = nu::parse(&mut working_set, Some(fname), wrapped.as_bytes(), false);
    if !working_set.parse_errors.is_empty() {
        return;
    }
    let wrapped_start = file_start(&working_set, fname);
    let source_start = wrapped_start + prefix_len;

    let mut prev_export_def = false;
    for (span, shape) in nu::flatten_block(&working_set, &block) {
        let content = span_text(&wrapped, wrapped_start, span).unwrap_or("");
        let flag = is_reserved_term(content)
            && match &shape {
                nu::FlatShape::VarDecl(_) => true,
                nu::FlatShape::String => !prev_export_def,
                _ => false,
            };
        if flag {
            let src_off = span.start.saturating_sub(source_start);
            let (line, _col) = span_to_line_col(source, src_off);
            diagnostics.push(Diagnostic::error(
                "rig::reserved",
                Some(Source {
                    path: Some(rel.to_string()),
                    position: [wide(line), 0],
                }),
                format!(
                    "`{content}` is reserved -- it may appear only as a call-target's exported `main`; rename this identifier",
                ),
            ));
        }
        if matches!(&shape, nu::FlatShape::Signature) {
            for pname in signature_param_names(content) {
                if is_reserved_term(&pname) {
                    let src_off = span.start.saturating_sub(source_start);
                    let (line, _col) = span_to_line_col(source, src_off);
                    diagnostics.push(Diagnostic::error(
                        "rig::reserved",
                        Some(Source {
                            path: Some(rel.to_string()),
                            position: [wide(line), 0],
                        }),
                        format!(
                            "`{pname}` is reserved and cannot be a parameter name; rename it",
                        ),
                    ));
                }
            }
        }
        prev_export_def =
            matches!(&shape, nu::FlatShape::InternalCall(_)) && content == "export def";
    }
}

pub(crate) fn is_reserved_term(s: &str) -> bool {
    s == "main"
}

fn signature_param_names(sig: &str) -> Vec<String> {
    let chars: Vec<char> = sig.chars().collect();
    let n = chars.len();
    let mut names = Vec::new();
    let mut depth: i32 = 0;
    let mut at_slot_start = true;
    let mut i = 0;
    while i < n {
        let c = chars[i];
        match c {
            '[' | '<' | '(' | '{' => {
                if depth > 0 {
                    at_slot_start = false;
                }
                depth += 1;
                i += 1;
            }
            ']' | '>' | ')' | '}' => {
                depth -= 1;
                i += 1;
            }
            ',' if depth <= 1 => {
                at_slot_start = true;
                i += 1;
            }
            ':' => {
                at_slot_start = false;
                i += 1;
            }
            c if c.is_whitespace() => {
                i += 1;
            }
            _ if at_slot_start && depth <= 1 => {
                let start = i;
                while i < n {
                    let cc = chars[i];
                    if cc.is_alphanumeric() || cc == '_' || cc == '-' || cc == '.' {
                        i += 1;
                    } else {
                        break;
                    }
                }
                let raw: String = chars[start..i].iter().collect();
                let name = raw
                    .trim_start_matches("...")
                    .trim_start_matches("--")
                    .trim_start_matches('-');
                if !name.is_empty() {
                    names.push(name.to_string());
                }
                at_slot_start = false;
            }
            _ => {
                i += 1;
            }
        }
    }
    names
}

fn validate_mod_nu_ast(
    rel: &str,
    fname: &str,
    stem: &str,
    source: &str,
    parent: &std::path::Path,
    engine: &ParseEngine,
    diagnostics: &mut Vec<Diagnostic>,
) {
    check_summary_length(rel, source, None, diagnostics);
    let wrapper_name = format!("__v_{stem}");
    let (wrapped, prefix_len) = wrap_as_module(source, &wrapper_name);
    let engine_state = engine.engine_state_for_file(parent);
    let mut working_set = nu::StateWorkingSet::new(&engine_state);
    let _ = working_set.files.push(PathBuf::from(fname), nu::Span::unknown());
    let files_before = working_set.num_files();
    let outer_block = nu::parse(&mut working_set, Some(fname), wrapped.as_bytes(), false);
    let source_start = file_start(&working_set, fname) + prefix_len;

    if !working_set.parse_errors.is_empty()
        && let Some(cycle) = detect_import_cycle(&working_set, files_before)
    {
        diagnostics.push(Diagnostic::error(
            "module::circular_import",
            Some(Source {
                path: Some(rel.to_string()),
                position: [0, 0],
            }),
            cycle,
        ));
        return;
    }

    for err in &working_set.parse_errors {
        let span_start = err.span().start.saturating_sub(source_start);
        let (line, _col) = span_to_line_col(source, span_start);
        diagnostics.push(Diagnostic::error(
            "rig::parse_error",
            Some(Source {
                path: Some(rel.to_string()),
                position: [wide(line), 0],
            }),
            format!("parse error: {}", nuval::NuValError::from(err).short()),
        ));
    }

    let body_block_id = outer_block
        .pipelines
        .first()
        .and_then(|p| p.elements.first())
        .and_then(|elem| match &elem.expr.expr {
            nu::Expr::Call(call) => call.arguments.iter().find_map(|arg| {
                if let nu::Argument::Positional(e) = arg {
                    if let nu::Expr::Block(id) = &e.expr {
                        Some(*id)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }),
            _ => None,
        });

    let body_block_id: nu::BlockId = match body_block_id {
        Some(id) => id,
        None => {
            return;
        }
    };

    let body = working_set.get_block(body_block_id);
    for pipeline in &body.pipelines {
        for elem in &pipeline.elements {
            check_mod_nu_pipeline_element(
                rel,
                &elem.expr,
                &working_set,
                source,
                source_start,
                diagnostics,
            );
        }
    }
}

fn check_mod_nu_pipeline_element(
    rel: &str,
    expr: &nu::Expression,
    working_set: &nu::StateWorkingSet,
    source: &str,
    source_start: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let span_start = expr.span.start.saturating_sub(source_start);
    let (line, _col) = span_to_line_col(source, span_start);
    match &expr.expr {
        nu::Expr::Call(call) => {
            let decl = working_set.get_decl(call.decl_id);
            let name = decl.name();
            if matches!(
                name,
                "export use"
                    | "export module"
                    | "export const"
                    | "export def"
                    | "use"
                    | "overlay use"
                    | "def"
                    | "const"
                    | "alias"
            ) {
                return;
            }
            diagnostics.push(Diagnostic::error(
                "rig::mod_nu",
                Some(Source {
                    path: Some(rel.to_string()),
                    position: [wide(line), 0],
                }),
                format!(
                    "mod.nu may only contain the module cascade (`export module`/`export use`), exports (`export def`/`export const`), or private `def`/`const`/`use`/`alias` declarations; got call to `{name}`",
                ),
            ));
        }
        nu::Expr::Garbage => {
        }
        nu::Expr::ImportPattern(_) | nu::Expr::Overlay(_) => {}
        other => {
            diagnostics.push(Diagnostic::error(
                "rig::mod_nu",
                Some(Source {
                    path: Some(rel.to_string()),
                    position: [wide(line), 0],
                }),
                format!(
                    "mod.nu may only contain the module cascade, exports, or private declarations; got `{}`",
                    short_expr_label(other),
                ),
            ));
        }
    }
}

fn short_expr_label(expr: &nu::Expr) -> &'static str {
    match expr {
        nu::Expr::FullCellPath(_) => "cell-path expression",
        nu::Expr::Var(_) => "variable reference",
        nu::Expr::String(_) => "string literal",
        nu::Expr::Int(_) => "integer literal",
        nu::Expr::Float(_) => "float literal",
        nu::Expr::Bool(_) => "bool literal",
        nu::Expr::Block(_) => "block",
        nu::Expr::Closure(_) => "closure",
        nu::Expr::BinaryOp(_, _, _) => "binary operation",
        nu::Expr::Subexpression(_) => "subexpression",
        nu::Expr::Keyword(_) => "keyword",
        _ => "other expression",
    }
}

fn validate_function_file_ast(
    rel: &str,
    fname: &str,
    stem: &str,
    source: &str,
    parent: &std::path::Path,
    engine: &ParseEngine,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<(IndexFunction, String, String)> {
    check_summary_length(rel, source, Some("export def main"), diagnostics);
    let wrapper_name = format!("__v_{stem}");
    let (wrapped, prefix_len) = wrap_as_module(source, &wrapper_name);
    let engine_state = engine.engine_state_for_file(parent);
    let mut working_set = nu::StateWorkingSet::new(&engine_state);
    let _ = working_set.files.push(PathBuf::from(fname), nu::Span::unknown());
    let files_before = working_set.num_files();
    let _ = nu::parse(&mut working_set, Some(fname), wrapped.as_bytes(), false);
    let wrapped_start = file_start(&working_set, fname);
    let source_start = wrapped_start + prefix_len;

    if !working_set.parse_errors.is_empty()
        && let Some(cycle) = detect_import_cycle(&working_set, files_before)
    {
        diagnostics.push(Diagnostic::error(
            "module::circular_import",
            Some(Source {
                path: Some(rel.to_string()),
                position: [0, 0],
            }),
            cycle,
        ));
        return None;
    }

    for err in &working_set.parse_errors {
        let span_start = err.span().start.saturating_sub(source_start);
        let (line, _col) = span_to_line_col(source, span_start);
        diagnostics.push(Diagnostic::error(
            "rig::parse_error",
            Some(Source {
                path: Some(rel.to_string()),
                position: [wide(line), 0],
            }),
            format!("parse error: {}", nuval::NuValError::from(err).short()),
        ));
    }
    if !working_set.parse_errors.is_empty() {
        return None;
    }

    let wrapper_name_bytes = wrapper_name.as_bytes();
    let module_id = match working_set.find_module(wrapper_name_bytes) {
        Some(id) => id,
        None => {
            diagnostics.push(Diagnostic::error(
                "rig::internal",
                Some(Source {
                    path: Some(rel.to_string()),
                    position: [0, 0],
                }),
                "internal: wrapper module not found after parse",
            ));
            return None;
        }
    };
    let module: &nu::Module = working_set.get_module(module_id);

    let main_id = module.main?;

    let main_line = decl_line(&working_set, main_id, source, source_start);
    let main_sig = working_set.get_decl(main_id).signature();
    let contract = match contract_signature(&main_sig, "main") {
        Ok(contract) => contract,
        Err(reasons) => {
            for reason in reasons {
                diagnostics.push(Diagnostic::error(
                    "rig::contract",
                    Some(Source {
                        path: Some(rel.to_string()),
                        position: [wide(main_line), 0],
                    }),
                    reason,
                ));
            }
            return None;
        }
    };
    let summary = main_sig.description.clone();
    let details = main_sig.extra_description.clone();
    Some((
        IndexFunction {
            name: stem.to_string(),
            data_type: contract.data_type,
            result_type: contract.result_type,
            takes_timeout: contract.takes_timeout,
        },
        summary,
        details,
    ))
}

fn decl_line(
    working_set: &nu::StateWorkingSet,
    decl_id: nu::DeclId,
    source: &str,
    source_start: usize,
) -> usize {
    let decl = working_set.get_decl(decl_id);
    let span = decl.signature().name.is_empty();
    let _ = span;
    if let Some(block_id) = decl.block_id() {
        let block = working_set.get_block(block_id);
        if let Some(span) = block.span {
            let src_offset = span.start.saturating_sub(source_start);
            let (line, _col) = span_to_line_col(source, src_offset);
            return line;
        }
    }
    0
}
