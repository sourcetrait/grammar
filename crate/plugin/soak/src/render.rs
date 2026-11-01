use crate::*;

/// Partials held in memory, each compiled when the parser is built.
type MemoryPartials = liquid::partials::EagerCompiler<liquid::partials::InMemorySource>;

/// A nushell value, as a fill holds it.
type NuValue = nu::Value;

/// A Liquid value, as a template reads it.
type LiquidValue = liquid::model::Value;

/// The partials directory's name, directly below the source.
const LIQUID: &str = "liquid";

/// A template's file suffix, dropped from its output's name.
const DOT_LIQUID: &str = ".liquid";

/// A planned output's source: a template to render or a file to copy.
enum Output {
    Render(PathBuf),
    Copy(PathBuf),
}

/// A staged output's content: rendered text or the file to copy.
enum Content {
    Text(String),
    Copy(PathBuf),
}

/// A soak's plan: the directories to make, then each output by its path.
struct Plan {
    dirs: Vec<PathBuf>,
    outputs: Vec<(PathBuf, Output)>,
}

/// Renders `src`'s templates into `dst` and copies its other files.
pub(crate) fn render_dir(
    src: PathSpan<'_>,
    dst: PathSpan<'_>,
    fill: &nu::Record,
) -> SoakResult<nu::Value> {
    if !src.path.is_dir() {
        return CorkError::err_path_not_found(src.path, PathKind::Dir, src.span).map_err(SoakError::from);
    }
    if dst.path.exists() && !dst.path.is_dir() {
        return Err(SoakError::DestinationNotDirectory { dst: dst.path.to_path_buf(), span: dst.span });
    }
    let source = dunce::canonicalize(src.path)
        .map_err(|e| CorkError::path_io(e, src.path, PathOp::DirMeta, src.span))?;
    let destination = resolve(dst.path)
        .map_err(|e| CorkError::path_io(e, dst.path, PathOp::DirMeta, dst.span))?;
    if destination.starts_with(&source) {
        return Err(SoakError::DestinationInSource { dst: destination, span: dst.span });
    }

    let plan = plan(&source, &destination)?;
    let partials = source.join(LIQUID);
    let parser = {
        let mut builder = liquid::ParserBuilder::with_stdlib();
        if partials.is_dir() {
            builder = builder.partials(build_memory_partials(&partials)?);
        }
        builder.build().map_err(|source| SoakError::LiquidParse { source })?
    };
    let fill = record_to_liquid_object(fill)?;
    let staged = stage(plan.outputs, &parser, &fill)?;
    write(&destination, &plan.dirs, staged, dst.span)?;

    Ok(nu::Value::nothing(consts::NOSPAN))
}

/// Every output of soaking `source` into `destination`, each one checked.
fn plan(source: &Path, destination: &Path) -> SoakResult<Plan> {
    let mut plan = Plan { dirs: Vec::new(), outputs: Vec::new() };
    let mut claimed: HashMap<PathBuf, PathBuf> = HashMap::new();
    let walker = walk(source)
        .filter_entry(|entry| undotted(entry) && !(entry.depth() == 1 && entry.file_name() == LIQUID));
    for entry in walker {
        let entry = entry.map_err(|source| SoakError::Walk { source })?;
        let relative = relative(entry.path(), source);
        let file_type = entry.file_type();
        let (relative, output) = if file_type.is_dir() {
            (relative, None)
        } else if !file_type.is_file() {
            return Err(SoakError::Unsupported { path: entry.path().to_path_buf() });
        } else if relative.extension().is_some_and(|ext| ext == LIQUID) {
            (relative.with_extension(""), Some(Output::Render(entry.path().to_path_buf())))
        } else {
            (relative, Some(Output::Copy(entry.path().to_path_buf())))
        };

        let path = destination.join(&relative);
        if path.starts_with(source) {
            return Err(SoakError::OutputInSource { output: path });
        }
        if let Some(first) = claimed.insert(relative, entry.path().to_path_buf()) {
            return Err(SoakError::Collision { output: path, first, second: entry.path().to_path_buf() });
        }
        match output {
            None => plan.dirs.push(path),
            Some(output) => plan.outputs.push((path, output)),
        }
    }

    Ok(plan)
}

/// Each template rendered into memory, a failure naming its file.
fn stage(
    outputs: Vec<(PathBuf, Output)>,
    parser: &liquid::Parser,
    fill: &liquid::Object,
) -> SoakResult<Vec<(PathBuf, Content)>> {
    outputs
        .into_iter()
        .map(|(path, output)| match output {
            Output::Render(template) => {
                let text = parser.parse_file(&template)
                    .and_then(|parsed| parsed.render(fill))
                    .map_err(|source| SoakError::Template { path: template, source })?;
                Ok((path, Content::Text(text)))
            },
            Output::Copy(file) => Ok((path, Content::Copy(file))),
        })
        .collect()
}

/// The destination, its directories, and each staged output written.
fn write(
    destination: &Path,
    dirs: &[PathBuf],
    staged: Vec<(PathBuf, Content)>,
    span: nu::Span,
) -> SoakResult<()> {
    for dir in std::iter::once(destination).chain(dirs.iter().map(PathBuf::as_path)) {
        fs::create_dir_all(dir).map_err(|e| CorkError::path_io(e, dir, PathOp::CreateDir, span))?;
    }
    for (path, content) in staged {
        match content {
            Content::Text(text) => {
                fs::write(&path, text).map_err(|e| CorkError::path_io(e, &path, PathOp::WriteFile, span))?;
            },
            Content::Copy(file) => {
                fs::copy(&file, &path)
                    .map_err(|e| CorkError::path_copy(e, PathKind::File, &file, &path, None))?;
            },
        }
    }

    Ok(())
}

/// A walk below `dir` in name order, following symlinks.
fn walk(dir: &Path) -> walkdir::IntoIter {
    walkdir::WalkDir::new(dir)
        .follow_links(true)
        .sort_by_file_name()
        .min_depth(1)
        .into_iter()
}

/// Whether an entry's own name is undotted.
fn undotted(entry: &walkdir::DirEntry) -> bool {
    !entry.file_name().as_encoded_bytes().starts_with(b".")
}

/// `path` below `base`: the components past `base`'s own.
fn relative(path: &Path, base: &Path) -> PathBuf {
    path.components().skip(base.components().count()).collect()
}

/// `path` with its longest existing ancestor canonicalized, the rest kept.
fn resolve(path: &Path) -> io::Result<PathBuf> {
    let mut missing: Vec<&std::ffi::OsStr> = Vec::new();
    let mut existing = path;
    loop {
        match dunce::canonicalize(existing) {
            Ok(canonical) => {
                return Ok(missing.iter().rev().fold(canonical, |resolved, name| resolved.join(name)));
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let (Some(parent), Some(name)) = (existing.parent(), existing.file_name()) else {
                    return Err(e);
                };
                missing.push(name);
                existing = parent;
            },
            Err(e) => return Err(e),
        }
    }
}

/// Each partial in `dir`, named by its path below it, `.liquid` trimmed.
fn build_memory_partials(dir: &Path) -> SoakResult<MemoryPartials> {
    let mut partials = MemoryPartials::empty();
    for entry in walk(dir).filter_entry(undotted) {
        let entry = entry.map_err(|source| SoakError::Walk { source })?;
        let path = entry.path();
        if !entry.file_type().is_file() || path.extension().is_some_and(|ext| ext != LIQUID) {
            continue;
        }

        let relative = relative(path, dir);
        let name = relative.to_str()
            .ok_or_else(|| CorkError::utf8(relative.as_os_str(), None))?
            .trim_end_matches(DOT_LIQUID)
            .to_string();
        let contents = fs::read_to_string(path)
            .map_err(|e| CorkError::path_io(e, path, PathOp::ReadFile, None))?;
        partials.add(name, contents);
    }

    Ok(partials)
}

/// Renders the template string `s` with `fill`.
pub(crate) fn render_str(
    s: &str,
    fill: &nu::Record,
) -> SoakResult<nu::Value> {
    let template = liquid::ParserBuilder::with_stdlib()
        .build()
        .map_err(|source| SoakError::LiquidParse { source })?
        .parse(s)
        .map_err(|source| SoakError::LiquidParse { source })?;

    let fill = record_to_liquid_object(fill)?;
    let out = template.render(&fill)
        .map_err(|source| SoakError::LiquidParse { source })?;

    Ok(nu::Value::string(out, consts::NOSPAN))
}

/// A record as a Liquid object, each value converted.
fn record_to_liquid_object(record: &nu::Record) -> SoakResult<liquid::Object> {
    let mut object = liquid::Object::new();
    for (key, value) in record.iter() {
        let k = key.clone().into();
        let v = value_to_liquid_value(value)?;
        object.insert(k, v);
    }

    Ok(object)
}

/// A nushell value as a Liquid value, refusing what Liquid cannot hold.
fn value_to_liquid_value(value: &NuValue) -> SoakResult<LiquidValue> {
    match value {
        NuValue::Nothing { .. } => Ok(LiquidValue::Nil),
        NuValue::Int { val,.. } => Ok(LiquidValue::scalar(*val)),
        NuValue::Float { val,.. } => Ok(LiquidValue::scalar(*val)),
        NuValue::Bool { val,.. } => Ok(LiquidValue::scalar(*val)),
        NuValue::String { val,.. } => Ok(LiquidValue::scalar(val.clone())),
        NuValue::Glob { val,.. } => Ok(LiquidValue::scalar(val.clone())),
        NuValue::Filesize { val,.. } => Ok(LiquidValue::scalar(val.to_string())),
        NuValue::Range { val,.. } => Ok(LiquidValue::scalar(val.to_string())),
        NuValue::CellPath { val,.. } => Ok(LiquidValue::scalar(val.to_string())),
        v @ NuValue::Binary { .. } => {
            let utf8 = v.coerce_string()
                .map_err(|_| SoakError::LiquidValue { invalid: NuType::Binary, span: v.span() })?;
            Ok(LiquidValue::scalar(utf8))
        },
        NuValue::List { vals,.. } => {
            let vals: Vec<LiquidValue> = vals.iter()
                .map(value_to_liquid_value)
                .collect::<SoakResult<_>>()?;
            Ok(LiquidValue::array(vals))
        },
        NuValue::Record { val, .. } => {
            let v = record_to_liquid_object(val)?;
            Ok(LiquidValue::Object(v))
        },
        NuValue::Duration { val,.. } => {
            let v = Duration::from_nanos(*val as u64);
            let v = humantime::Duration::from(v);
            Ok(LiquidValue::scalar(v.to_string()))
        },
        NuValue::Date { val, internal_span,.. } => {
            let v = val.format("%Y-%m-%d %H:%M:%S.%f %z").to_string();
            let v = liquid::model::DateTime::from_str(&v)
                .ok_or_else(|| SoakError::LiquidValue { invalid: NuType::Date, span: *internal_span })?;
            Ok(LiquidValue::scalar(v))
        },
        NuValue::Custom { val, internal_span,.. } => {
            let v = val.to_base_value(*internal_span)
                .map_err(|_| SoakError::LiquidValue { invalid: NuType::Custom, span: *internal_span })?;
            if matches!(v, NuValue::Custom {..}) {
                return Err(SoakError::LiquidValue { invalid: NuType::Custom, span: *internal_span });
            }
            let v = value_to_liquid_value(&v)?;
            Ok(v)
        },
        NuValue::Closure { internal_span,.. } => Err(SoakError::LiquidValue { invalid: NuType::Closure, span: *internal_span }),
        NuValue::Error { internal_span,.. } => Err(SoakError::LiquidValue { invalid: NuType::Error, span: *internal_span }),
    }
}
