use crate::*;

/// A file or a symlink kept for rendering; a symlink has no size of its own.
pub(crate) struct Leaf {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) size: Option<u64>,
}

/// A directory kept for rendering.
pub(crate) struct Branch {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
}

/// A directory's kept entries: its files and symlinks, and its directories.
#[derive(Default)]
pub(crate) struct Listing {
    pub(crate) leaves: Vec<Leaf>,
    pub(crate) branches: Vec<Branch>,
}

/// Renders `dir`'s compact tree, `list` giving each directory's entries.
pub(crate) fn render(
    dir: &Path,
    list: &mut dyn FnMut(&Path) -> TreeResult<Listing>,
) -> TreeResult<String> {
    let root = dir.display().to_string();
    let mut lines = vec![format!("{}/", root.trim_end_matches('/'))];
    render_level(dir, 1, list, &mut lines)?;
    Ok(lines.join("\n") + "\n")
}

/// Appends `dir`'s lines at `depth`, files and symlinks before directories.
fn render_level(
    dir: &Path,
    depth: usize,
    list: &mut dyn FnMut(&Path) -> TreeResult<Listing>,
    lines: &mut Vec<String>,
) -> TreeResult<()> {
    let Listing { mut leaves, mut branches } = list(dir)?;
    leaves.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    branches.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));

    let indent = " ".repeat(depth);
    for leaf in &leaves {
        match leaf.size {
            Some(size) => lines.push(format!("{indent}{} {}", wrap(&leaf.name), size_label(size))),
            None => lines.push(format!("{indent}{}", symlink_label(&leaf.name, &leaf.path))),
        }
    }
    for branch in &branches {
        lines.push(format!("{indent}{}/", wrap(&branch.name)));
        render_level(&branch.path, depth + 1, list, lines)?;
    }
    Ok(())
}

/// A size's label in the largest 1024-based unit at least one, floored.
pub(crate) fn size_label(bytes: u64) -> String {
    const KB: u64 = 1 << 10;
    const MB: u64 = 1 << 20;
    const GB: u64 = 1 << 30;
    const TB: u64 = 1 << 40;
    if bytes == 0 {
        "0".to_string()
    } else if bytes < KB {
        format!("{bytes}b")
    } else if bytes < MB {
        format!("{}kb", bytes / KB)
    } else if bytes < GB {
        format!("{}mb", bytes / MB)
    } else if bytes < TB {
        format!("{}gb", bytes / GB)
    } else {
        format!("{}tb", bytes / TB)
    }
}

/// A name holding a space, backtick-wrapped; any other unchanged.
pub(crate) fn wrap(name: &str) -> String {
    if name.contains(' ') {
        format!("`{name}`")
    } else {
        name.to_string()
    }
}

/// A symlink's line: its canonical target, a file's with its size.
fn symlink_label(name: &str, path: &Path) -> String {
    let name = wrap(name);
    match fs::canonicalize(path) {
        Ok(target) => match fs::metadata(&target) {
            Ok(metadata) if metadata.is_dir() => format!("{name} -> {}/", target.display()),
            Ok(metadata) => {
                format!("{name} -> {} {}", target.display(), size_label(metadata.len()))
            }
            Err(_) => broken_symlink_label(&name, path),
        },
        Err(_) => broken_symlink_label(&name, path),
    }
}

/// A broken symlink's line: its raw target, no size; `name` already wrapped.
fn broken_symlink_label(name: &str, path: &Path) -> String {
    match fs::read_link(path) {
        Ok(target) => format!("{name} -> {}", target.display()),
        Err(_) => format!("{name} ->"),
    }
}
