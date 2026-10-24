use rig/sourcetrait/grammar/airepo/store/git/common

# Get onto the working branch and fast-forward it from the bare.
#
# Call before working, on the checkout `repo` names. Requires the `srv` remote
# and a clean tree, fetches the bare, switches to the line its HEAD names, and
# fast-forwards it - ff-only, so a divergence STOPS rather than being rebased
# or forced past. Returns the branch, its tip, the bare's tip, and how far the
# two sit apart.
export def main [
    args: record<repo: directory>
]: nothing -> record<repo: string, working_branch: string, tip: string, bare_tip: string, ahead: int, behind: int> {
    let repo = (common checkout $args.repo)
    cd $repo
    common ensure_remote
    let wt = (common worktree)
    if (($wt.staged | length) > 0) or ($wt.dirty > 0) {
        error make { msg: "store sync: working tree not clean - commit first" }
    }
    let working = (common working_branch)
    common grun ["fetch" "srv"] "fetch the bare"
    common grun ["switch" $working] $"switch to ($working)"
    let bare = $"srv/($working)"
    if (^git rev-parse --verify --quiet $bare | complete | get exit_code) == 0 {
        common grun ["merge" "--ff-only" $bare] $"fast-forward ($working) from the bare"
    }
    let d = (common rev_delta $bare $working)
    {
        repo: $repo,
        working_branch: $working,
        tip: (^git rev-parse --short HEAD | str trim),
        bare_tip: (^git rev-parse --short $bare | str trim),
        ahead: $d.ahead,
        behind: $d.behind,
    }
}
