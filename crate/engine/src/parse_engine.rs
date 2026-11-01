use crate::*;

pub struct ParseEngine {
    engine_state: nu::EngineState,
    extra_lib_dir: Option<PathBuf>,
}

/// The parse-time constant naming the lib dirs.
pub(crate) const LIB_DIRS_CONST: &str = "$NU_LIB_DIRS";

/// The parse-time constant naming the plugin dirs.
pub(crate) const PLUGIN_DIRS_CONST: &str = "$NU_PLUGIN_DIRS";

/// Set the parse-time list constant `name` to `dirs`.
pub(crate) fn set_dirs_const(engine_state: &mut nu::EngineState, name: &str, dirs: &[PathBuf]) {
    let mut ws = nu::StateWorkingSet::new(engine_state);
    let var_id = ws.add_variable(
        name.as_bytes().to_vec(),
        nu::Span::unknown(),
        nu::Type::List(Box::new(nu::Type::String)),
        false,
    );
    ws.set_variable_const_val(var_id, dirs_value(dirs));
    let delta = ws.render();
    let _ = engine_state.merge_delta(delta);
}

/// Dirs as a nushell list of strings.
pub(crate) fn dirs_value(dirs: &[PathBuf]) -> nu::Value {
    let vals: Vec<nu::Value> = dirs
        .iter()
        .map(|d| nu::Value::string(d.to_string_lossy().into_owned(), nu::Span::unknown()))
        .collect();
    nu::Value::list(vals, nu::Span::unknown())
}

impl ParseEngine {
    /// A parse engine started as the eval engines are, with each eval's `grimm` decls.
    pub fn new_full() -> Self {
        let mut engine_state = startup(Mode::Stateless);
        let mut working_set = nu::StateWorkingSet::new(&engine_state);
        register_nuapi(&mut working_set, None);
        let delta = working_set.render();
        let _ = engine_state.merge_delta(delta);
        Self {
            engine_state,
            extra_lib_dir: None,
        }
    }

    pub fn with_extra_lib_dir(&self, dir: PathBuf) -> Self {
        Self {
            engine_state: self.engine_state.clone(),
            extra_lib_dir: Some(dir),
        }
    }

    pub fn engine_state(&self) -> &nu::EngineState {
        &self.engine_state
    }

    pub fn engine_state_for_file(&self, file_parent: &std::path::Path) -> nu::EngineState {
        let mut clone = self.engine_state.clone();
        clone.add_env_var(
            "PWD".to_string(),
            nu::Value::string(
                file_parent.to_string_lossy().into_owned(),
                nu::Span::unknown(),
            ),
        );
        let mut dirs = Vec::new();
        if let Some(extra) = &self.extra_lib_dir {
            dirs.push(extra.clone());
        }
        dirs.push(data_home());
        set_dirs_const(&mut clone, LIB_DIRS_CONST, &dirs);
        clone
    }
}

/// The lint and validator engine, rebuilt when the plugin registry moves.
pub struct LintEngine {
    inner: std::sync::Mutex<LintHolder>,
}

struct LintHolder {
    engine: Arc<ParseEngine>,
    registry_mtime: Option<SystemTime>,
}

impl Default for LintEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LintEngine {
    pub fn new() -> Self {
        Self {
            inner: std::sync::Mutex::new(LintHolder {
                engine: Arc::new(ParseEngine::new_full()),
                registry_mtime: registry_mtime(),
            }),
        }
    }

    /// The engine to lint or validate with, rebuilt first if the registry moved.
    pub fn current(&self) -> Arc<ParseEngine> {
        let mut holder = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let now = registry_mtime();
        if now != holder.registry_mtime {
            holder.engine = Arc::new(ParseEngine::new_full());
            holder.registry_mtime = now;
        }
        holder.engine.clone()
    }
}

/// Where `fname`'s text begins in the span space its working set shares.
pub fn file_start(working_set: &nu::StateWorkingSet, fname: &str) -> usize {
    working_set
        .get_span_for_filename(fname)
        .map_or(0, |span| span.start)
}

/// The text `span` covers in `text`, which begins at `text_start` in span space.
pub fn span_text(text: &str, text_start: usize, span: nu::Span) -> Option<&str> {
    let start = span.start.checked_sub(text_start)?;
    let end = span.end.checked_sub(text_start)?;
    text.get(start..end)
}

pub fn wrap_as_module(source: &str, wrapper_name: &str) -> (String, usize) {
    let prefix = format!("module {wrapper_name} {{\n");
    let prefix_len = prefix.len();
    let wrapped = format!("{prefix}{source}\n}}\n");
    (wrapped, prefix_len)
}
