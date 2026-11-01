use crate::*;

/// `raw` as an absolute path, `~` expanded, relative to the caller's directory.
fn expand(engine: &nu::EngineInterface, raw: &Path) -> Result<PathBuf, nu::LabeledError> {
    let expanded = nu::expand_path(raw, true);
    if expanded.is_absolute() {
        return Ok(expanded);
    }
    let cwd = engine.get_current_dir()?;
    Ok(nu::expand_path_with(raw, cwd, true))
}

/// `raw` expanded, then canonicalized: a path that must exist.
pub(crate) fn canonical(
    engine: &nu::EngineInterface,
    raw: &Path,
    head: nu::Span,
) -> Result<PathBuf, nu::LabeledError> {
    let expanded = expand(engine, raw)?;
    fs::canonicalize(&expanded)
        .map_err(|error| labeled_error(format!("{}: {error}", expanded.display()), head))
}
