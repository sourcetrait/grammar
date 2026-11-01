# purview.rs

The session's side of the purviews, a view on each engine. The stored rows and
their resolution are each engine's (`crate/engine/src/equip/purview.rs`), the
MCP's own or a worker's, reached through its requests; what stays here is the
view the session holds on each.

## fn pattern_delta
What a change of view revealed, from the two resolutions' expanded patterns, so
a view change answers the signatures newly in view alone.

## struct CurrentPurview
Session-resident by design - it is a view rather than a configuration, so it belongs in
memory and dies with the host. Confirmed across a real restart, where the configured rows
persisted and the view reset.

### fn set
An empty list means `default`, so the view always names at least one purview and there is
no looking-at-nothing state to reason about - which is why there is no separate reset:
resetting is just setting the view to nothing in particular.

### fn retain_known
So a rig uninstall or a purview deletion cannot leave the session pointing at something
gone. The rows are the engine's answer to the write that changed them.

## struct Views
One view on the MCP's own engine and one on each remote engine, made at that
engine's first command and dying with the host as a view does. A remote
engine's purviews are its own namespace's rows, so the view on one engine never
names another's ids.

## fn unknown_purview
The refusal a view naming an id no purview answers to has always given.

## fn NuSh::purview_on
A purview request whole, answered by the routed engine: the MCP's own through
its controller, a worker's as the session's purview request, a worker's refusal
and the transport's errors under their `remote::engine::` kinds.

## fn NuSh::resolve_purviews
Every view's resolution is the routed engine's, one request for a set of ids, its failure
answering in the command's envelope as the read's always did.

## fn NuSh::purviews_on
## fn NuSh::purview_rows_on
The routed engine's purviews, and the rows a configure or a rig's change
leaves there.
