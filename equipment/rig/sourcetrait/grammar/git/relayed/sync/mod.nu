use rig/sourcetrait/grammar/git/relayed/common

# Prepare a relay repo to work in.
#
# Make both local branches current and my branch safely rebased on the
# principal's, verified fast-forward-pushable - call before starting work. cd
# into the repo, require the `relayed` remote, the `relay.agent` git config, and
# a clean tree, then run the shared sync core (fetch bare -> ff their branch ->
# rebase mine onto it -> assert ff). My branch is `draft/ai/<handle>/<agent>`.
# Breaks on the first failed git hop; never forces. Returns the resolved state:
# handle and agent, branch names, short tips, and how far my branch is
# ahead/behind theirs.
export def main [args: record<repo: directory>]: nothing -> record<handle: string, agent: string, mine_branch: string, their_branch: string, mine_tip: string, their_tip: string, bare_mine_tip: string, ahead: int, behind: int> {
    cd $args.repo
    common relay_ensure_remote
    let h = (common relay_handle)
    let agent = (common relay_agent)
    common relay_ensure_clean
    let t = (common relay_sync_core $h $agent)
    {
        handle: $h,
        agent: $agent,
        mine_branch: $t.mine_branch,
        their_branch: $t.their_branch,
        mine_tip: $t.mine_tip,
        their_tip: $t.their_tip,
        bare_mine_tip: $t.bare_mine_tip,
        ahead: $t.ahead,
        behind: $t.behind,
    }
}
