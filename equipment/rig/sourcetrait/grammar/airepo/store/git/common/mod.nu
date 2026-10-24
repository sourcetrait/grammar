# Shared helpers for the store's git tools (grammar:airepo/store/git).
#
# The store is an AIREPO-CLASS repository: it carries no draft pair, and work
# rides the one line its bare's HEAD names. That is why the project-side relay
# tools cannot drive it - they resolve a `draft/<handle>` ref that does not
# exist here. Every helper operates on the cwd repo and BREAKS ON ERROR, so a
# non-fast-forward stops the tool rather than tempting a force.

# run a git hop; error make (with context + stderr) on a non-zero exit.
export def grun [args: list<string>, what: string]: nothing -> nothing {
    let r = (^git ...$args | complete)
    if $r.exit_code != 0 {
        error make { msg: $"store git stopped at ($what): ($r.stderr | str trim)" }
    }
}

# the store checkout the caller named: refused unless it is a git work tree's
# root, so a stray path never reaches a git hop.
export def checkout [repo: directory]: nothing -> string {
    let dir = ($repo | path expand)
    if (($dir | path join ".git" | path type) == null) {
        error make { msg: $"store git: no git checkout at ($dir)" }
    }
    $dir
}

# error unless the repo has the `srv` remote, its bare, configured.
export def ensure_remote []: nothing -> nothing {
    if not ("srv" in (^git remote | lines)) {
        error make { msg: "store git: no `srv` remote configured in this repo" }
    }
}

# the working branch: the line the `srv` bare's HEAD names, so a new line is
# followed by every tool without an edit.
export def working_branch []: nothing -> string {
    let r = (^git ls-remote --symref srv HEAD | complete)
    if $r.exit_code != 0 {
        error make { msg: $"store git: cannot read the bare's HEAD: ($r.stderr | str trim)" }
    }
    let named = ($r.stdout | parse --regex 'ref: refs/heads/(?<branch>\S+)\s+HEAD')
    if ($named | is-empty) {
        error make { msg: "store git: the bare's HEAD names no branch" }
    }
    $named | first | get branch
}

# ahead/behind counts of <b> relative to <a>: {behind, ahead} (a...b left/right).
export def rev_delta [a: string, b: string]: nothing -> record<behind: int, ahead: int> {
    let c = (^git rev-list --left-right --count $"($a)...($b)" | str trim | split row --regex '\s+')
    { behind: ($c | get 0 | into int), ahead: ($c | get 1 | into int) }
}

# the working tree, split the way a caller acts on it: what is ALREADY staged
# (the shared-tree check - a pre-staged path is not mine to sweep into a commit),
# what is dirty, and what is untracked.
export def worktree []: nothing -> record<staged: list<string>, dirty: int, untracked: int> {
    let rows = (^git status --porcelain | lines)
    let staged = ($rows
        | where {|l| ($l | str substring 0..1) not-in [" " "?"] }
        | each {|l| $l | str substring 3.. })
    let dirty = ($rows | where {|l| ($l | str substring 1..2) in ["M" "D"] } | length)
    let untracked = ($rows | where {|l| $l | str starts-with "??" } | length)
    { staged: $staged, dirty: $dirty, untracked: $untracked }
}
