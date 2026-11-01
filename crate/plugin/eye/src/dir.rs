use crate::*;

/// Renders `dir`'s tree, skipping what an ignore glob matches unless regarded.
pub(crate) fn dir_tree(dir: &Path, ignore: &[String], regard: &[String]) -> TreeResult<String> {
    let mut ignore_globs = vec![compile("**/.git")?];
    for pattern in ignore {
        ignore_globs.push(compile(pattern)?);
    }
    let mut regard_globs = Vec::with_capacity(regard.len());
    for pattern in regard {
        regard_globs.push(compile(pattern)?);
    }

    render(dir, &mut |at| list(dir, at, &ignore_globs, &regard_globs))
}

/// Compiles one ignore or regard pattern.
fn compile(pattern: &str) -> TreeResult<nu_glob::Pattern> {
    nu_glob::Pattern::new(pattern).context(GlobSnafu { pattern: pattern.to_string() })
}

/// The entries of `dir` that no glob hides, `root` the tree's own.
fn list(
    root: &Path,
    dir: &Path,
    ignore: &[nu_glob::Pattern],
    regard: &[nu_glob::Pattern],
) -> TreeResult<Listing> {
    let mut listing = Listing::default();
    for entry in fs::read_dir(dir).context(ReadSnafu { path: dir.to_path_buf() })? {
        let entry = entry.context(ReadSnafu { path: dir.to_path_buf() })?;
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(&path);
        if ignored(relative, ignore, regard) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let metadata = fs::symlink_metadata(&path).context(ReadSnafu { path: path.clone() })?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            listing.leaves.push(Leaf { name, path, size: None });
        } else if file_type.is_dir() {
            listing.branches.push(Branch { name, path });
        } else {
            listing.leaves.push(Leaf { name, path, size: Some(metadata.len()) });
        }
    }
    Ok(listing)
}

/// Whether `relative` is hidden: an ignore glob matches it and no regard does.
fn ignored(relative: &Path, ignore: &[nu_glob::Pattern], regard: &[nu_glob::Pattern]) -> bool {
    if !ignore.iter().any(|glob| glob.matches_path(relative)) {
        return false;
    }
    !regard.iter().any(|glob| glob.matches_path(relative))
}

/// Lists a directory as a compact tree, each file with its size.
pub(crate) struct EyeDirCommand;

impl nu::SimplePluginCommand for EyeDirCommand {
    type Plugin = EyePlugin;

    fn name(&self) -> &str {
        "eye dir"
    }

    fn description(&self) -> &str {
        "Compact file tree listing."
    }

    fn extra_description(&self) -> &str {
        r#"```fs
/path/dir
/path/dir/.gitignore
/path/dir/subdir
/path/dir/subdir/file1.txt
/path/dir/subdir/somedir/.file2
/path/dir/subdir/otherdir
```
```tree
/path/dir/
 .gitignore 0
 subdir/
  file1.txt 32mb
  otherdir/
  somedir/
   .file2 40b
```

Ignored unless regarded: ['.git']

@ignore Skip the specified globs; deny-list
@regard Render the specified globs; allow-list, exceptional

Globs match each path below the directory: `**/target` skips every `target`,
`target` the top one alone; a trailing slash matches nothing."#
    }

    fn signature(&self) -> nu::Signature {
        nu::Signature::build("eye dir")
            .required("dir", nu::SyntaxShape::Directory, "Root directory to render.")
            .optional(
                "ignore",
                nu::SyntaxShape::List(Box::new(nu::SyntaxShape::GlobPattern)),
                "Globs to skip; deny-list (extends the default `**/.git`).",
            )
            .optional(
                "regard",
                nu::SyntaxShape::List(Box::new(nu::SyntaxShape::GlobPattern)),
                "Globs to render even when ignored; allow-list, exceptional.",
            )
            .category(nu::Category::FileSystem)
    }

    fn examples(&self) -> Vec<nu::Example<'_>> {
        vec![nu::Example {
            example: "eye dir /path/to/dir",
            description: "Render a compact file tree of a directory.",
            result: None,
        }]
    }

    fn run(
        &self,
        _plugin: &EyePlugin,
        engine: &nu::EngineInterface,
        call: &nu::EvaluatedCall,
        _input: &nu::Value,
    ) -> Result<nu::Value, nu::LabeledError> {
        let dir = path::canonical(engine, &call.req::<PathBuf>(0)?, call.head)?;
        let ignore = globs(call, 1)?;
        let regard = globs(call, 2)?;
        let rendered =
            dir_tree(&dir, &ignore, &regard).map_err(|error| labeled_error(error, call.head))?;
        Ok(nu::Value::string(rendered, call.head))
    }
}

/// The glob strings at optional positional `pos`, empty when it is absent.
fn globs(call: &nu::EvaluatedCall, pos: usize) -> Result<Vec<String>, nu::LabeledError> {
    let Some(value): Option<nu::Value> = call.opt(pos)? else {
        return Ok(Vec::new());
    };
    let nu::Value::List { vals, .. } = &value else {
        return Err(labeled_error("expected a list of globs", call.head));
    };
    vals.iter()
        .map(|item| item.coerce_string().map_err(|error| labeled_error(error, call.head)))
        .collect()
}
