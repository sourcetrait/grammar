use crate::*;

/// The name of git's own directory, never rendered.
const DOT_GIT: &str = ".git";

/// Renders `dir`'s tree as git sees it: `.git` and gitignored paths skipped.
pub(crate) fn git_tree(dir: &Path) -> TreeResult<String> {
    let mut listings: HashMap<PathBuf, Listing> = HashMap::new();
    let walk = ignore::WalkBuilder::new(dir)
        .standard_filters(false)
        .git_ignore(true)
        .parents(true)
        .follow_links(false)
        .sort_by_file_name(|a, b| a.as_encoded_bytes().cmp(b.as_encoded_bytes()))
        .filter_entry(|entry| entry.file_name() != DOT_GIT)
        .build();
    for result in walk {
        let entry = result.context(WalkSnafu { path: dir.to_path_buf() })?;
        let (Some(parent), true) = (entry.path().parent(), entry.depth() > 0) else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path().to_path_buf();
        let listing = listings.entry(parent.to_path_buf()).or_default();
        if entry.path_is_symlink() {
            listing.leaves.push(Leaf { name, path, size: None });
        } else if entry.file_type().is_some_and(|file_type| file_type.is_dir()) {
            listing.branches.push(Branch { name, path });
        } else {
            let metadata = entry.metadata().context(WalkSnafu { path: path.clone() })?;
            listing.leaves.push(Leaf { name, path, size: Some(metadata.len()) });
        }
    }

    render(dir, &mut |at| Ok(listings.remove(at).unwrap_or_default()))
}

/// Lists a directory as git sees it: a compact tree, gitignored paths skipped.
pub(crate) struct EyeGitCommand;

impl nu::SimplePluginCommand for EyeGitCommand {
    type Plugin = EyePlugin;

    fn name(&self) -> &str {
        "eye git"
    }

    fn description(&self) -> &str {
        "Compact file tree listing, skipping `.git` and what `.gitignore` files ignore."
    }

    fn extra_description(&self) -> &str {
        "The tree `eye dir` renders, with git's ignore rules: inside a repository \
         every `.gitignore` below the directory, and those above it up to the \
         repository's root, apply as git applies them; outside one none does."
    }

    fn signature(&self) -> nu::Signature {
        nu::Signature::build("eye git")
            .required("dir", nu::SyntaxShape::Directory, "Root directory to render.")
            .category(nu::Category::FileSystem)
    }

    fn examples(&self) -> Vec<nu::Example<'_>> {
        vec![nu::Example {
            example: "eye git /path/to/repo",
            description: "Render a compact file tree of a repository as git sees it.",
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
        let rendered = git_tree(&dir).map_err(|error| labeled_error(error, call.head))?;
        Ok(nu::Value::string(rendered, call.head))
    }
}
