//! The namespace's purviews: their stored rows, seeding, pruning, and resolution.
use crate::*;

/// The namespace-level meta directory.
pub(crate) const META_DIR: &str = ".meta";

/// The stored rows' file under the meta directory.
const PURVIEWS_FILE: &str = "purviews.nuon";

/// What is in view when nothing says otherwise; configurable.
pub const PURVIEW_DEFAULT: &str = "default";

/// What is in view right now; derived, never a row.
pub const PURVIEW_CURRENT: &str = ".";

/// Everything: both a built-in purview id and a namepath pattern.
pub const PURVIEW_ALL: &str = "*";

/// The builtins' own purviews together, each by reference.
pub const PURVIEW_CORE: &str = "core";

/// The sigil marking a value as a reference to another purview.
pub const PURVIEW_REF: char = '@';

/// The stored rows' path in the namespace's data home.
pub fn purviews_path() -> PathBuf {
    data_home().join(META_DIR).join(PURVIEWS_FILE)
}

/// Render the purview table as NUON through nuval, in nushell's default style.
pub(crate) fn purviews_to_nuon(rows: &[PurviewRow]) -> Result<String, String> {
    nuval::to_nuon(rows, NuonStyle::Default).map_err(|e| e.to_string())
}

/// Read the purview table back from its NUON through nuval.
pub(crate) fn purviews_from_nuon(text: &str) -> Result<Vec<PurviewRow>, String> {
    nuval::from_nuon(text).map_err(|e| e.to_string())
}

/// A store failure as the internal error it answers.
fn store_failed(
    phase: &str,
    reason: impl ToString,
) -> RuntimeError {
    RuntimeError::Internal {
        phase: phase.to_string(),
        reason: reason.to_string(),
    }
}

/// Every configured purview; none only before the substrate materialized it.
pub(crate) fn load_purviews() -> Result<Option<Vec<PurviewRow>>, RuntimeError> {
    let text = match fs::read_to_string(purviews_path()) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(store_failed("io", e)),
    };
    purviews_from_nuon(&text)
        .map(Some)
        .map_err(|reason| store_failed("purview::decode", reason))
}

/// Each staged write's number, so concurrent writers never share a staging file.
static STAGED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Write the rows whole: staged beside the table, then renamed over it.
pub(crate) fn save_purviews(rows: &[PurviewRow]) -> Result<(), RuntimeError> {
    let path = purviews_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| store_failed("io", e))?;
    }
    let nuon = purviews_to_nuon(rows).map_err(|reason| store_failed("purview::encode", reason))?;
    let number = STAGED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let staged = path.with_file_name(format!(".purviews.{}.{number}.tmp", std::process::id()));
    fs::write(&staged, nuon.as_bytes()).map_err(|e| store_failed("io", e))?;
    fs::rename(&staged, &path).map_err(|e| {
        let _ = fs::remove_file(&staged);
        store_failed("io", e)
    })
}

/// Is `id` a legal purview label? Path-like, but never a path.
pub(crate) fn is_valid_purview_id(id: &str) -> bool {
    if id.is_empty() || id.starts_with('/') || id.starts_with("./") || id.ends_with('/') {
        return false;
    }
    if id.contains("//") {
        return false;
    }
    id.split('/').all(|seg| {
        let mut chars = seg.chars();
        match chars.next() {
            Some(first) if first.is_ascii_lowercase() || first == '_' => {
                chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            }
            _ => false,
        }
    })
}

/// A built-in that is derived rather than stored.
pub(crate) fn is_derived_purview(id: &str) -> bool {
    id == PURVIEW_CURRENT || id == PURVIEW_ALL
}

/// The purview id a value references, or none for a plain pattern.
pub fn purview_ref(value: &str) -> Option<&str> {
    value.strip_prefix(PURVIEW_REF)
}

/// May `id` be referenced as `@id`?
pub(crate) fn is_valid_purview_ref(id: &str) -> bool {
    !is_derived_purview(id) && is_valid_purview_id(id)
}

/// Expand purview references into the patterns they stand for.
pub(crate) fn expand_values(
    values: &[String],
    rows: Option<&Vec<PurviewRow>>,
) -> Vec<String> {
    fn walk(
        values: &[String],
        rows: Option<&Vec<PurviewRow>>,
        seen: &mut Vec<String>,
        out: &mut Vec<String>,
    ) {
        for value in values {
            let Some(id) = purview_ref(value) else {
                if !out.contains(value) {
                    out.push(value.clone());
                }
                continue;
            };
            if seen.iter().any(|s| s == id) {
                continue;
            }
            seen.push(id.to_string());
            if let Some(row) = rows.and_then(|r| r.iter().find(|row| row.id == id)) {
                let nested = row.namepath_patterns.clone();
                walk(&nested, rows, seen, out);
            }
        }
    }
    let mut out: Vec<String> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    walk(values, rows, &mut seen, &mut out);
    out
}

/// Write `default` as `['*']` when it has no row, under the equip store's lock.
pub fn ensure_default_purview() -> Result<(), RuntimeError> {
    let _repo = equip_repo_lock();
    let mut rows = load_purviews()?.unwrap_or_default();
    if rows.iter().any(|row| row.id == PURVIEW_DEFAULT) {
        return Ok(());
    }
    rows.push(PurviewRow {
        id: PURVIEW_DEFAULT.to_string(),
        namepath_patterns: vec![PURVIEW_ALL.to_string()],
    });
    save_purviews(&rows)
}

/// Seed a builtin's purviews where absent: its own row, and its `@` in `core`.
pub(crate) fn seed_builtin(
    rows: &mut Vec<PurviewRow>,
    name: &str,
) {
    if !rows.iter().any(|row| row.id == name) {
        rows.push(PurviewRow {
            id: name.to_string(),
            namepath_patterns: vec![format!("{name}:")],
        });
    }
    let reference = format!("{PURVIEW_REF}{name}");
    match rows.iter_mut().find(|row| row.id == PURVIEW_CORE) {
        Some(row) => {
            if !row.namepath_patterns.contains(&reference) {
                row.namepath_patterns.push(reference);
            }
        }
        None => rows.push(PurviewRow {
            id: PURVIEW_CORE.to_string(),
            namepath_patterns: vec![reference],
        }),
    }
}

/// Seed the purviews of each builtin a new namespace installed.
pub fn seed_builtin_purviews(installed: &[&str]) {
    if installed.is_empty() {
        return;
    }
    let _repo = equip_repo_lock();
    let mut rows = match load_purviews() {
        Ok(rows) => rows.unwrap_or_default(),
        Err(e) => {
            eprintln!("grammar: builtin purviews not seeded: {e:?}");
            return;
        }
    };
    for name in installed {
        seed_builtin(&mut rows, name);
    }
    if let Err(e) = save_purviews(&rows) {
        eprintln!("grammar: builtin purviews not seeded: {e:?}");
    }
}

/// The namepath patterns a set of ids puts in view, in order and deduplicated.
pub(crate) fn resolve_patterns(
    ids: &[String],
    rows: Option<&Vec<PurviewRow>>,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for id in ids {
        let patterns: Vec<String> = if id == PURVIEW_ALL {
            vec![PURVIEW_ALL.to_string()]
        } else {
            match rows.and_then(|r| r.iter().find(|row| &row.id == id)) {
                Some(row) => row.namepath_patterns.clone(),
                None => Vec::new(),
            }
        };
        for pattern in patterns {
            if !out.contains(&pattern) {
                out.push(pattern);
            }
        }
    }
    out
}

/// Is `pattern` the whole-rig pattern of `rig`, in either of a builtin's spellings?
pub(crate) fn is_rig_pattern(
    pattern: &str,
    rig: &str,
) -> bool {
    matches!(NamepathPattern::parse(pattern), Ok(NamepathPattern::Rig { rig: named }) if named == rig)
}

/// What a stored value needs in order to mean anything.
fn pattern_requires(pattern: &str) -> Option<PatternNeed> {
    if let Some(id) = purview_ref(pattern) {
        return Some(PatternNeed::Purview(id.to_string()));
    }
    match NamepathStr::parse(pattern).ok()? {
        NamepathStr::Pattern(NamepathPattern::All) => None,
        NamepathStr::Pattern(NamepathPattern::Current) => Some(PatternNeed::Nothing),
        NamepathStr::Pattern(NamepathPattern::Author { author }) => {
            Some(PatternNeed::Author(author))
        }
        NamepathStr::Pattern(
            NamepathPattern::Rig { rig }
            | NamepathPattern::ModuleTree { rig, .. }
            | NamepathPattern::ModuleCalls { rig, .. },
        ) => Some(PatternNeed::Rig(rig)),
        NamepathStr::Namepath(n) => n
            .validate()
            .ok()
            .map(|r| PatternNeed::Rig(r.rig().to_string())),
    }
}

/// What a stored pattern needs to stand.
enum PatternNeed {
    Author(String),
    Rig(String),
    /// A `@id` reference, satisfied by a purview rather than by a rig.
    Purview(String),
    /// Names nothing at all: a stored `.` is meaningless in a row.
    Nothing,
}

/// Drop patterns no registered rig or purview satisfies, until none dangles.
pub(crate) fn prune_dangling(rows: &mut Vec<PurviewRow>) -> Vec<String> {
    let names = registered_rig_names();
    let mut pruned: Vec<String> = Vec::new();
    loop {
        let ids: Vec<String> = rows.iter().map(|row| row.id.clone()).collect();
        let mut dropped = false;
        for row in rows.iter_mut() {
            row.namepath_patterns.retain(|pattern| {
                let live = match pattern_requires(pattern) {
                    None => true,
                    Some(PatternNeed::Nothing) => false,
                    Some(PatternNeed::Author(author)) => names
                        .iter()
                        .any(|n| n.split_once('/').is_some_and(|(a, _)| a == author)),
                    Some(PatternNeed::Rig(rig)) => names.iter().any(|n| n == &rig),
                    Some(PatternNeed::Purview(id)) => ids.iter().any(|k| k == &id),
                };
                if !live {
                    dropped = true;
                    if !pruned.contains(pattern) {
                        pruned.push(pattern.clone());
                    }
                }
                live
            });
        }
        let before = rows.len();
        rows.retain(|row| !row.namepath_patterns.is_empty());
        if !dropped && rows.len() == before {
            return pruned;
        }
    }
}

/// May `id` be named as something to bring into view?
pub(crate) fn is_nameable_purview(
    id: &str,
    rows: Option<&Vec<PurviewRow>>,
) -> bool {
    id == PURVIEW_ALL || rows.is_some_and(|r| r.iter().any(|row| row.id == id))
}

/// Each id beside the namepath patterns it resolves to.
pub(crate) fn purview_views(
    ids: &[String],
    rows: Option<&Vec<PurviewRow>>,
) -> Vec<PurviewView> {
    ids.iter()
        .map(|id| PurviewView(id.clone(), resolve_patterns(std::slice::from_ref(id), rows)))
        .collect()
}

/// Each configured purview, its patterns as stored.
pub(crate) fn purview_list() -> Result<Vec<PurviewView>, RuntimeError> {
    let rows = load_purviews()?;
    let ids: Vec<String> = rows
        .as_ref()
        .map(|r| r.iter().map(|row| row.id.clone()).collect())
        .unwrap_or_default();
    Ok(purview_views(&ids, rows.as_ref()))
}

/// The refusal of a purview id or reference nothing may configure.
fn invalid_id(
    id: &str,
    reason: &str,
) -> RuntimeError {
    RuntimeError::PurviewInvalidId {
        id: id.to_string(),
        reason: reason.to_string(),
    }
}

/// Set an id's patterns, an empty list deleting it; the rows after, pruned.
pub(crate) fn purview_configure(
    id: &str,
    patterns: &[String],
) -> Result<Vec<PurviewRow>, RuntimeError> {
    if is_derived_purview(id) {
        return Err(invalid_id(
            id,
            "`.` and `*` are derived, not stored, so they cannot be configured",
        ));
    }
    if !is_valid_purview_id(id) {
        return Err(invalid_id(
            id,
            "must be slash-separated snake components, bare relative (no leading `/` or `./`)",
        ));
    }
    for value in patterns {
        if let Some(reference) = purview_ref(value)
            && !is_valid_purview_ref(reference)
        {
            return Err(invalid_id(
                value,
                "`@` must reference a configurable purview; `@.` and `@*` are derived and are \
                 never rows",
            ));
        }
    }
    let _repo = equip_repo_lock();
    let mut rows = load_purviews()?.unwrap_or_default();
    rows.retain(|row| row.id != id);
    let patterns = if patterns.is_empty() && id == PURVIEW_DEFAULT {
        vec![PURVIEW_ALL.to_string()]
    } else {
        patterns.to_vec()
    };
    if !patterns.is_empty() {
        rows.push(PurviewRow {
            id: id.to_string(),
            namepath_patterns: patterns,
        });
    }
    prune_dangling(&mut rows);
    save_purviews(&rows)?;
    Ok(rows)
}

/// What a set of ids puts in view: the patterns, each id's own, the unknown ids.
pub(crate) fn purview_resolve(ids: &[String]) -> Result<PurviewResolution, RuntimeError> {
    let rows = load_purviews()?;
    let rows = rows.as_ref();
    Ok(PurviewResolution {
        patterns: expand_values(&resolve_patterns(ids, rows), rows),
        views: purview_views(ids, rows),
        unknown: ids
            .iter()
            .filter(|id| !is_nameable_purview(id, rows))
            .cloned()
            .collect(),
    })
}

/// A freshly installed rig brought into `default` and the session's view.
pub(crate) fn purview_add_rig(
    rig: &str,
    current: &[String],
) -> Result<Vec<PurviewRow>, RuntimeError> {
    let _repo = equip_repo_lock();
    let mut rows = load_purviews()?.unwrap_or_default();
    let builtin = builtin_name(rig);
    let pattern = match builtin {
        Some(name) => {
            seed_builtin(&mut rows, name);
            format!("{PURVIEW_REF}{PURVIEW_CORE}")
        }
        None => format!("{rig}:"),
    };
    let mut targets: Vec<String> = vec![PURVIEW_DEFAULT.to_string()];
    for id in current {
        if id != PURVIEW_ALL && !targets.contains(id) {
            targets.push(id.clone());
        }
    }
    for id in targets {
        if builtin.is_some() && (id == PURVIEW_CORE || builtin_equip(&id).is_some()) {
            continue;
        }
        if let Some(row) = rows.iter_mut().find(|row| row.id == id) {
            let present = row.namepath_patterns.contains(&pattern)
                || row.namepath_patterns.iter().any(|p| is_rig_pattern(p, rig));
            if !present {
                row.namepath_patterns.push(pattern.clone());
            }
        }
    }
    prune_dangling(&mut rows);
    save_purviews(&rows)?;
    Ok(rows)
}

/// An uninstalled rig dropped from every purview that named it.
pub(crate) fn purview_remove_rig(rig: &str) -> Result<Vec<PurviewRow>, RuntimeError> {
    let _repo = equip_repo_lock();
    let mut rows = load_purviews()?.unwrap_or_default();
    for row in rows.iter_mut() {
        row.namepath_patterns.retain(|p| !is_rig_pattern(p, rig));
    }
    prune_dangling(&mut rows);
    save_purviews(&rows)?;
    Ok(rows)
}
