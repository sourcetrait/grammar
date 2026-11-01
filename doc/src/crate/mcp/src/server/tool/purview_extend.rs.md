# purview_extend.rs

## struct PurviewDeltaResult
The two halves are asymmetric on purpose: extending normally only adds, so a `hidden` field
would report an empty list that reads like a result.

`revealed` is `interface info`'s block rendered for the patterns that were newly added -
not a diff of two whole blocks. So when the view already carries `*`, what it shows was
visible before as well: the patterns are what changed, and the block says what they name.
The delta is computed on expanded values on both sides, the engine's resolution of each
view, so a `@ref` to something already in view reveals nothing.

## fn purview_extend
The view extended is the session's on the routed engine. An unknown id is refused rather
than ignored. Contributing nothing in silence would turn a
typo into a view that simply fails to widen, which looks identical to a purview that is
genuinely empty. The ids asked are resolved first, so the refusal names the first unknown
one before the view changes.
