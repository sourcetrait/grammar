# Box-side relay tools: sync prepares a repo, submit commits + ff-pushes.
#
# Fast-forward only and break-on-error; named apart from the user's `ai relay`
# host toolkit on purpose. Branches: the principal's `draft/<handle>`, an
# agent's `draft/ai/<handle>/<agent>` with the agent segment from the repo's
# `relay.agent` git config. Call targets: grammar:git/relayed:{sync,submit};
# shared helpers live in common.
export module sync
export use sync
export module submit
export use submit
export module common
