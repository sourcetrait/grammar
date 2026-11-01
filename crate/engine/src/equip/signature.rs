use crate::*;

/// A one-line summary from `.meta/docs/<docs_path>/summary.md`, FLATTENED.
fn read_summary(
    docs_dir: &std::path::Path,
    docs_path: &str,
) -> String {
    read_doc(docs_dir, docs_path, "summary.md")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn read_doc(docs_dir: &std::path::Path, docs_path: &str, file: &str) -> String {
    let dir = if docs_path.is_empty() {
        docs_dir.to_path_buf()
    } else {
        docs_dir.join(docs_path)
    };
    fs::read_to_string(dir.join(file)).unwrap_or_default()
}

/// One space per depth level - the indentation IS the structure.
const SIGNATURE_INDENT: &str = " ";

/// `<token> # <summary>`, with the ` # ...` omitted entirely when undocumented.
pub(crate) fn with_summary(
    token: &str,
    summary: &str,
) -> String {
    let summary = summary.trim();
    if summary.is_empty() {
        token.to_string()
    } else {
        format!("{token} # {summary}")
    }
}

/// Append one indented signature line to the block.
fn push_signature_line(
    out: &mut String,
    depth: usize,
    token: &str,
    summary: &str,
) {
    out.push_str(&SIGNATURE_INDENT.repeat(depth));
    out.push_str(&with_summary(token, summary));
    out.push('\n');
}

/// A call's `<name> <data> <result>`; `name` is a leaf or a full namepath.
pub(crate) fn signature_of(
    name: &str,
    f: &IndexFunction,
) -> String {
    format!(
        "{name} {} {}",
        signature_group(&f.data_type),
        signature_group(&f.result_type),
    )
}

/// Join a parent module path with a leaf name, tolerating an empty parent.
fn join_path(
    parent: &str,
    name: &str,
) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}/{name}")
    }
}

/// What every module of one rig's signature walk shares.
struct SignatureScope<'a> {
    docs_dir: &'a std::path::Path,
    rig: &'a str,
    patterns: &'a [NamepathStr],
}

/// Render one module's calls and submodules that the scope's patterns put in view.
fn push_within_signatures(
    out: &mut String,
    depth: usize,
    functions: &[IndexFunction],
    modules: &[IndexModule],
    parent: &str,
    scope: &SignatureScope<'_>,
) -> bool {
    let mut showed = false;
    let mut calls: Vec<&IndexFunction> = functions.iter().collect();
    calls.sort_by(|a, b| a.name.cmp(&b.name));
    for f in calls {
        let namepath = NamepathRef::Function {
            rig: scope.rig.to_string(),
            module_path: parent.to_string(),
            name: f.name.clone(),
        };
        if !scope.patterns.iter().any(|p| p.covers(&namepath)) {
            continue;
        }
        let docs_path = join_path(parent, &f.name);
        push_signature_line(
            out,
            depth,
            &signature_of(&f.name, f),
            &read_summary(scope.docs_dir, &docs_path),
        );
        showed = true;
    }
    let mut subs: Vec<&IndexModule> = modules.iter().collect();
    subs.sort_by(|a, b| a.name.cmp(&b.name));
    for m in subs {
        let module_path = join_path(parent, &m.name);
        let mut body = String::new();
        let below = push_within_signatures(
            &mut body,
            depth + 1,
            &m.functions,
            &m.modules,
            &module_path,
            scope,
        );
        let namepath = NamepathRef::Module {
            rig: scope.rig.to_string(),
            module_path: module_path.clone(),
        };
        let covered = scope.patterns.iter().any(|p| p.covers(&namepath));
        if !below && !covered {
            continue;
        }
        push_signature_line(out, depth, &m.name, &read_summary(scope.docs_dir, &module_path));
        out.push_str(&body);
        showed = true;
    }
    showed
}

/// The namespace as ONE indented signature block, filtered to a set of patterns.
pub async fn render_signatures_within(
    locks: &RigLocks,
    patterns: &[NamepathStr],
) -> String {
    let mut out = String::new();
    let names = registered_rig_names();
    for name in names.iter().filter(|name| builtin_name(name).is_some()) {
        if let Some((summary, body)) = rig_signatures(locks, name, 1, patterns).await {
            push_signature_line(&mut out, 0, display_equip(name), &summary);
            out.push_str(&body);
        }
    }
    let mut current_author: Option<String> = None;
    for name in names.iter().filter(|name| builtin_name(name).is_none()) {
        let Some((author, leaf)) = name.split_once('/') else {
            continue;
        };
        let Some((summary, body)) = rig_signatures(locks, name, 2, patterns).await else {
            continue;
        };
        if current_author.as_deref() != Some(author) {
            push_signature_line(&mut out, 0, author, "");
            current_author = Some(author.to_string());
        }
        push_signature_line(&mut out, 1, leaf, &summary);
        out.push_str(&body);
    }
    out
}

/// A rig's summary and its signatures in view from `depth`; None when none are.
async fn rig_signatures(
    locks: &RigLocks,
    name: &str,
    depth: usize,
    patterns: &[NamepathStr],
) -> Option<(String, String)> {
    let lock = locks.lookup(name).await;
    let _guard = match &lock {
        Some(l) => Some(l.read().await),
        None => None,
    };
    let index = match load_index(name) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("grammar: signature block skipped {name}: {e}");
            return None;
        }
    };
    let docs_dir = rig_docs_dir(name);
    let scope = SignatureScope {
        docs_dir: &docs_dir,
        rig: name,
        patterns,
    };
    let mut body = String::new();
    let showed = push_within_signatures(
        &mut body,
        depth,
        &index.functions,
        &index.modules,
        "",
        &scope,
    );
    let rig_namepath = NamepathRef::Rig {
        rig: name.to_string(),
    };
    let covered = patterns.iter().any(|p| p.covers(&rig_namepath));
    if !showed && !covered {
        return None;
    }
    Some((read_summary(&docs_dir, ""), body))
}

pub fn index_node<'a>(
    index: &'a RigIndex,
    module_path: &str,
) -> Option<(&'a Vec<IndexFunction>, &'a Vec<IndexModule>)> {
    if module_path.is_empty() {
        return Some((&index.functions, &index.modules));
    }
    let mut fns = &index.functions;
    let mut mods = &index.modules;
    for seg in module_path.split('/') {
        let m = mods.iter().find(|m| m.name == seg)?;
        fns = &m.functions;
        mods = &m.modules;
    }
    Some((fns, mods))
}

pub fn inspect_impl(
    rig: &str,
    module_path: &str,
    name: Option<&str>,
) -> Result<InspectDoc, EngineError> {
    check_equip_name(rig)?;
    require_rig(rig)?;
    if !is_valid_module_path(module_path) {
        return Err(EngineError::RigInvalidModulePath {
            module_path: module_path.to_string(),
            reason: "invalid module path".to_string(),
        });
    }
    let index = load_index(rig)?;
    let docs_dir = rig_docs_dir(rig);
    let root = rig_dir(rig);
    match name {
        Some(fn_name) => {
            let (fns, _) = index_node(&index, module_path).ok_or_else(|| {
                EngineError::RigInvalidModulePath {
                    module_path: module_path.to_string(),
                    reason: "module not found".to_string(),
                }
            })?;
            let f = fns.iter().find(|f| f.name == fn_name).ok_or_else(|| {
                EngineError::FunctionNotDefined {
                    rig: rig.to_string(),
                    module_path: module_path.to_string(),
                    name: fn_name.to_string(),
                }
            })?;
            let docs_path = format!("{module_path}/{fn_name}");
            let namepath = format!("{}:{module_path}:{fn_name}", display_equip(rig));
            Ok(InspectDoc::Call(CallDoc {
                src: root
                    .join(module_path)
                    .join(fn_name)
                    .join("mod.nu")
                    .display()
                    .to_string(),
                signature: with_summary(
                    &signature_of(&namepath, f),
                    &read_summary(&docs_dir, &docs_path),
                ),
                details: read_doc(&docs_dir, &docs_path, "details.md"),
            }))
        }
        None if module_path.is_empty() => Ok(InspectDoc::Rig(RigDoc {
            srcdir: root.display().to_string(),
            summary: read_summary(&docs_dir, ""),
            details: read_doc(&docs_dir, "", "details.md"),
        })),
        None => {
            if index_node(&index, module_path).is_none() {
                return Err(EngineError::RigInvalidModulePath {
                    module_path: module_path.to_string(),
                    reason: "module not found".to_string(),
                });
            }
            Ok(InspectDoc::Module(ModuleDoc {
                src: root.join(module_path).join("mod.nu").display().to_string(),
                summary: read_summary(&docs_dir, module_path),
                details: read_doc(&docs_dir, module_path, "details.md"),
            }))
        }
    }
}
