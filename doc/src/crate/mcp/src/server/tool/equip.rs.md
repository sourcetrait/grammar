# equip.rs

## enum EquipSummary
Untagged: the caller knows the variant from the command it ran, and the shapes are distinct.
`uninstall` returns no summary at all - void, matching its idempotent success.

`install`'s summary is its commit's result, the validator's warning rows in its `warnings`
as `interface equip commit`'s carry them. `check` keeps both buckets, `errors` and
`warnings`: it is the validator's report, so an error there is a finding rather than a
refusal.

## fn equip_install
Each action runs on the routed engine, a source path being that host's own. The install is
the engine's and the view the session's, so the purview update follows the engine's
answer, on the same engine, outside the equip's lock (the engine's `serve.rs`). An uninstall landing
in between leaves nothing behind: the update's own prune judges by registration, and drops
the pattern it just added for a rig no longer there.

## fn equip_uninstall
An unknown equip returns success with no summary rather than an error, because absent is
the requested state.

## fn NuSh::purview_rig
The rows are the engine's to adjust (`crate/engine/src/equip/purview.rs`'s
`purview_add_rig` and `purview_remove_rig`), the session's view going with the request,
since a rig installed joins each purview in view. An install brings a rig into view only
when the equip has one; a lib alone has no calls for a purview to show. An uninstall's
answer is the rows after, so the session drops any purview the prune removed, as a
purview deletion does.

Non-fatal: the install or uninstall already succeeded and is not rolled back, so a
bookkeeping failure is reported to stderr rather than turned into a failed command. It is
worth reading though, because a `default` that failed to gain the pattern leaves the new
rig installed but out of view.
