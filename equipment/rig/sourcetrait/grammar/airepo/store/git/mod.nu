# Git tools for the store: status reads, sync prepares, submit publishes.
#
# The store carries no draft pair, so grammar:git/relayed cannot drive it. Each
# call takes the store's checkout as `repo` and works the line the `srv` bare's
# HEAD names, so a new line needs no edit here. Fast-forward only and
# break-on-error throughout: a divergence stops the tool. Call targets:
# grammar:airepo/store/git:{status,sync,submit}; shared helpers live in common.
export module status
export use status
export module sync
export use sync
export module submit
export use submit
export module common
