use crate::*;

fn skeleton_function_source() -> String {
    "# one-line summary (<= 80 chars); becomes this function's doc\nexport def main [data: record<>]: nothing -> record<> {\n    # logic here; declare each record<>'s fields, data `nothing` if it takes none\n    {}\n}\n".to_string()
}

fn additively_wire_modnu(modnu: &std::path::Path, entry: &str) -> io::Result<()> {
    let existing = if modnu.exists() {
        fs::read_to_string(modnu)?
    } else {
        String::new()
    };
    if existing.lines().any(|l| l.trim() == entry) {
        return Ok(());
    }
    let mut out = existing;
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(entry);
    out.push('\n');
    fs::write(modnu, out)
}

fn validate_new_names(
    rig: &str,
    module_path: &str,
    name: Option<&str>,
) -> Result<(), EngineError> {
    check_equip_name(rig)?;
    if !is_valid_module_path(module_path) {
        return Err(EngineError::RigInvalidName {
            name: module_path.to_string(),
            reason: "invalid module path".to_string(),
        });
    }
    for seg in module_path.split('/').filter(|s| !s.is_empty()) {
        if is_reserved_term(seg) {
            return Err(EngineError::RigInvalidName {
                name: seg.to_string(),
                reason: "reserved `main` cannot name a module".to_string(),
            });
        }
    }
    if let Some(n) = name
        && (!is_valid_ident(n) || is_reserved_term(n))
    {
        return Err(EngineError::RigInvalidName {
            name: n.to_string(),
            reason: "invalid or reserved (`main`) function name".to_string(),
        });
    }
    Ok(())
}

/// Register an empty equip at its source: a rig with an empty root `mod.nu`.
pub fn establish_equip(
    equip: &str,
    source_dir: &Path,
) -> Result<Vec<String>, EngineError> {
    check_equip_name(equip)?;
    let _repo = equip_repo_lock();
    if equip_dir(equip).exists() {
        return Err(EngineError::EquipAlreadyRegistered {
            equip: equip.to_string(),
        });
    }
    let rig_source = source_dir.join(KIND_RIG);
    let canonical = rig_dir(equip);
    fs::create_dir_all(canonical.join(".meta"))?;
    fs::write(canonical.join("mod.nu"), b"")?;
    let index = RigIndex {
        source_path: rig_source.clone(),
        functions: Vec::new(),
        modules: Vec::new(),
    };
    let index_nuon = index_to_nuon(&index).map_err(|reason| EngineError::Internal {
        phase: "establish::serialize_index".to_string(),
        reason,
    })?;
    fs::write(canonical.join(META_FILE), index_nuon.as_bytes())?;
    run_git(&equip_root(), &["add", "--", equip])?;
    run_git(
        &equip_root(),
        &["commit", "-m", &format!("new equip {equip}")],
    )?;
    fs::create_dir_all(&rig_source)?;
    let root_modnu = rig_source.join("mod.nu");
    if !root_modnu.exists() {
        fs::write(&root_modnu, b"")?;
    }
    Ok(vec![root_modnu.to_string_lossy().into_owned()])
}

pub fn scaffold_leaf_exists(
    rig: &str,
    module_path: &str,
    name: Option<&str>,
) -> Result<bool, EngineError> {
    require_rig(rig)?;
    let mut dir = load_rig_source(rig)?;
    for seg in module_path.split('/').filter(|s| !s.is_empty()) {
        dir = dir.join(seg);
    }
    let leaf = match name {
        Some(fn_name) => dir.join(fn_name),
        None => dir,
    };
    Ok(leaf.exists())
}

pub fn scaffold_leaf(
    rig: &str,
    module_path: &str,
    name: Option<&str>,
) -> Result<Vec<String>, EngineError> {
    validate_new_names(rig, module_path, name)?;
    require_rig(rig)?;
    let sp = load_rig_source(rig)?;
    let mut created: Vec<String> = Vec::new();

    let mut dir = sp.clone();
    if !module_path.is_empty() {
        let segs: Vec<&str> = module_path.split('/').collect();
        for (i, seg) in segs.iter().enumerate() {
            let parent_modnu = dir.join("mod.nu");
            let child = dir.join(seg);
            let terminal_module = name.is_none() && i == segs.len() - 1;
            if terminal_module && child.exists() {
                return Err(EngineError::RigInvalidName {
                    name: module_path.to_string(),
                    reason: "module already exists; edit it instead of scaffolding over it"
                        .to_string(),
                });
            }
            fs::create_dir_all(&child)?;
            let child_modnu = child.join("mod.nu");
            if !child_modnu.exists() {
                fs::write(&child_modnu, b"")?;
                created.push(child_modnu.to_string_lossy().into_owned());
            }
            additively_wire_modnu(&parent_modnu, &format!("export module {seg}"))?;
            dir = child;
        }
    }

    if let Some(fn_name) = name {
        let fn_dir = dir.join(fn_name);
        if fn_dir.exists() {
            return Err(EngineError::RigInvalidName {
                name: fn_name.to_string(),
                reason: "function already exists; edit it instead of scaffolding over it"
                    .to_string(),
            });
        }
        fs::create_dir_all(&fn_dir)?;
        let fn_modnu = fn_dir.join("mod.nu");
        fs::write(&fn_modnu, skeleton_function_source())?;
        let parent_modnu = dir.join("mod.nu");
        additively_wire_modnu(&parent_modnu, &format!("export module {fn_name}"))?;
        additively_wire_modnu(&parent_modnu, &format!("export use {fn_name}"))?;
        created.push(fn_modnu.to_string_lossy().into_owned());
    }

    Ok(created)
}

fn run_git_output(dir: &std::path::Path, args: &[&str]) -> io::Result<String> {
    let out = process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| io::Error::other(format!("git: {e}")))?;
    if !out.status.success() {
        return Err(io::Error::other(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr),
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn parse_git_changes(porcelain: &str) -> CommitResult {
    let mut result = CommitResult {
        added: Vec::new(),
        modified: Vec::new(),
        removed: Vec::new(),
        warnings: Vec::new(),
    };
    for line in porcelain.lines() {
        if line.len() < 4 {
            continue;
        }
        let raw = line[3..].trim();
        let path = raw.rsplit(" -> ").next().unwrap_or(raw).trim().to_string();
        match line.as_bytes()[0] as char {
            'A' | 'C' => result.added.push(path),
            'M' | 'R' => result.modified.push(path),
            'D' => result.removed.push(path),
            _ => {}
        }
    }
    result
}

fn write_rig_meta(
    equip: &str,
    source_path: &Path,
    result: &ValidationResult,
) -> Result<(), EngineError> {
    let canonical = rig_dir(equip);
    let meta_dir = canonical.join(".meta");
    fs::create_dir_all(&meta_dir)?;
    let index = RigIndex {
        source_path: source_path.to_path_buf(),
        functions: result.functions.clone(),
        modules: result.modules.clone(),
    };
    let index_nuon = index_to_nuon(&index).map_err(|reason| EngineError::Internal {
        phase: "commit::serialize_index".to_string(),
        reason,
    })?;
    fs::write(canonical.join(META_FILE), index_nuon.as_bytes())?;
    let docs_dir = meta_dir.join("docs");
    for doc in &result.docs {
        let dir = if doc.docs_path.is_empty() {
            docs_dir.clone()
        } else {
            docs_dir.join(&doc.docs_path)
        };
        fs::create_dir_all(&dir)?;
        if !doc.summary.is_empty() {
            fs::write(dir.join("summary.md"), doc.summary.as_bytes())?;
        }
        if !doc.details.is_empty() {
            fs::write(dir.join("details.md"), doc.details.as_bytes())?;
        }
    }
    Ok(())
}

fn write_lib_meta(
    equip: &str,
    source_path: &Path,
) -> Result<(), EngineError> {
    fs::create_dir_all(lib_dir(equip).join(".meta"))?;
    let index = LibIndex {
        source_path: source_path.to_path_buf(),
    };
    let index_nuon = meta_to_nuon(&index).map_err(|reason| EngineError::Internal {
        phase: "commit::serialize_lib_index".to_string(),
        reason,
    })?;
    fs::write(lib_meta_path(equip), index_nuon.as_bytes())?;
    Ok(())
}

/// The source an installed equip was committed from: its kinds' parent.
fn registered_source(equip: &str) -> Result<PathBuf, EngineError> {
    let kind_source = if rig_meta_path(equip).exists() {
        load_rig_source(equip)?
    } else if lib_meta_path(equip).exists() {
        load_lib_index(equip)?.source_path
    } else {
        return Err(EngineError::EquipNotRegistered {
            equip: equip.to_string(),
        });
    };
    kind_source
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| EngineError::Internal {
            phase: "equip::registered_source".to_string(),
            reason: format!("`{}` has no parent", kind_source.display()),
        })
}

/// The kinds an equip's source holds, in install order.
fn source_kinds(source_dir: &Path) -> Vec<&'static str> {
    EQUIP_KINDS
        .iter()
        .copied()
        .filter(|kind| source_dir.join(kind).is_dir())
        .collect()
}

/// The kinds an equip's source holds, refusing a missing source or one with none.
fn require_source_kinds(source_dir: &Path) -> Result<Vec<&'static str>, EngineError> {
    if !source_dir.is_dir() {
        return Err(EngineError::EquipSourceMissing {
            path: source_dir.display().to_string(),
        });
    }
    let kinds = source_kinds(source_dir);
    if kinds.is_empty() {
        return Err(EngineError::EquipNoKinds {
            source_dir: source_dir.display().to_string(),
        });
    }
    Ok(kinds)
}

/// Validate an equip's source, then commit its kinds in place of the last ones.
fn promote_equip(
    equip: &str,
    source_dir: &Path,
    engine: &ParseEngine,
    verb: &str,
) -> Result<CommitResult, EngineError> {
    let kinds = require_source_kinds(source_dir)?;
    let result = validate_equip_source(equip, source_dir, &kinds, engine)?;
    if !result.is_empty() {
        return Err(EngineError::EquipViolations {
            diagnostics: result.diagnostics,
        });
    }
    let _repo = equip_repo_lock();
    let canonical = equip_dir(equip);
    if canonical.exists() {
        fs::remove_dir_all(&canonical)?;
    }
    for kind in &kinds {
        let kind_source = source_dir.join(kind);
        if *kind == KIND_LIB {
            copy_tree_all(&kind_source, &lib_dir(equip))?;
            write_lib_meta(equip, &kind_source)?;
        } else {
            copy_dir_recursive(&kind_source, &rig_dir(equip))?;
            write_rig_meta(equip, &kind_source, &result)?;
        }
    }
    run_git(&equip_root(), &["add", "--", equip])?;
    let porcelain = run_git_output(&equip_root(), &["status", "--porcelain", "--", equip])?;
    let mut changed = parse_git_changes(&porcelain);
    changed.warnings = Diagnostic::bucket(result.diagnostics).1;
    if changed.is_empty() {
        return Ok(changed);
    }
    run_git(
        &equip_root(),
        &["commit", "-m", &format!("{verb} equip {equip}")],
    )?;
    Ok(changed)
}

/// Re-commit an installed equip from its registered source.
pub fn commit_equip(equip: &str, engine: &ParseEngine) -> Result<CommitResult, EngineError> {
    check_equip_name(equip)?;
    if !equip_dir(equip).exists() {
        return Err(EngineError::EquipNotRegistered {
            equip: equip.to_string(),
        });
    }
    let source_dir = registered_source(equip)?;
    promote_equip(equip, &source_dir, engine, "commit")
}

pub fn check_source_dir(equip: &str, source_dir: &Path) -> Result<(), EngineError> {
    let registered = registered_source(equip)?;
    if registered != source_dir {
        return Err(EngineError::EquipSourcePathMismatch {
            equip: equip.to_string(),
            passed: source_dir.display().to_string(),
            registered: registered.display().to_string(),
        });
    }
    Ok(())
}

/// Install an equip from its source, validated whole before anything is written.
pub fn install_equip(
    equip: &str,
    source_dir: &Path,
    engine: &ParseEngine,
) -> Result<CommitResult, EngineError> {
    check_equip_name(equip)?;
    if equip_dir(equip).exists() {
        return Err(EngineError::EquipAlreadyRegistered {
            equip: equip.to_string(),
        });
    }
    promote_equip(equip, source_dir, engine, "install")
}

pub fn uninstall_equip(equip: &str) -> Result<(), EngineError> {
    let _repo = equip_repo_lock();
    let canonical = equip_dir(equip);
    if !canonical.exists() {
        return Ok(());
    }
    fs::remove_dir_all(&canonical)?;
    run_git(&equip_root(), &["add", "--", equip])?;
    run_git(
        &equip_root(),
        &["commit", "-m", &format!("uninstall equip {equip}")],
    )?;
    Ok(())
}

pub fn check_equip(
    equip: &str,
    engine: &ParseEngine,
) -> Result<ValidationResult, EngineError> {
    if !equip_dir(equip).exists() {
        return Err(EngineError::EquipNotRegistered {
            equip: equip.to_string(),
        });
    }
    let source_dir = registered_source(equip)?;
    let kinds = require_source_kinds(&source_dir)?;
    Ok(validate_equip_source(equip, &source_dir, &kinds, engine)?)
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> io::Result<()> {
    copy_rig_tree(src, dst, true)
}

fn copy_rig_tree(src: &std::path::Path, dst: &std::path::Path, is_root: bool) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let src_path = entry.path();
        let dst_path = dst.join(&name);
        let ft = entry.file_type()?;
        if ft.is_dir() {
            if is_root && (name == ".assets" || name == ".docs") {
                copy_tree_all(&src_path, &dst_path)?;
            } else if name.starts_with('.') {
                continue;
            } else {
                copy_rig_tree(&src_path, &dst_path, false)?;
            }
        } else if ft.is_file() && (name == ".gitignore" || !name.starts_with('.')) {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn copy_tree_all(src: &std::path::Path, dst: &std::path::Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let ft = entry.file_type()?;
        if ft.is_dir() {
            copy_tree_all(&src_path, &dst_path)?;
        } else if ft.is_file() {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
