use crate::*;

/// How many times one file must be registered before it can be a cycle.
const CYCLE_REPEATS: usize = 3;

/// How many `..` hops one spelling of a repeated file must carry to be a runaway.
const CYCLE_HOPS: usize = 8;

/// How many files a cycle message names before it stops listing them.
const CYCLE_NAMED: usize = 4;

/// The files a module-resolution CYCLE ran through; None on any other failure.
pub fn detect_import_cycle(
    working_set: &nu::StateWorkingSet,
    baseline: usize,
) -> Option<String> {
    let mut seen: HashMap<PathBuf, (usize, usize)> = HashMap::new();
    for cached in working_set.files().skip(baseline) {
        let spelling = Path::new(&*cached.name);
        let Ok(canonical) = std::fs::canonicalize(spelling) else {
            continue;
        };
        let hops = spelling
            .components()
            .filter(|c| matches!(c, std::path::Component::ParentDir))
            .count();
        let entry = seen.entry(canonical).or_default();
        entry.0 += 1;
        entry.1 = entry.1.max(hops);
    }

    let mut repeated: Vec<(PathBuf, usize)> = seen
        .into_iter()
        .filter(|(_, (count, hops))| *count >= CYCLE_REPEATS && *hops >= CYCLE_HOPS)
        .map(|(path, (count, _))| (path, count))
        .collect();
    if repeated.is_empty() {
        return None;
    }
    repeated.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let named: Vec<String> = repeated
        .iter()
        .take(CYCLE_NAMED)
        .map(|(path, _)| path.display().to_string())
        .collect();
    Some(named.join(" uses "))
}
