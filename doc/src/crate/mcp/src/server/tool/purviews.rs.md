# purviews.rs

## struct PurviewsResult
`purviews` reports the configured table VERBATIM with `@` references unexpanded, because this
is a report rather than a filter - a reader here wants the configuration as written.

`current` is the KEYS alone, since `purviews` above already says what each one resolves to.

## fn purview_list
Lists what is CONFIGURED on the routed engine, which is not the same set as what is in view:
an id can be configured and not current, and `*` can be current without being a row. The
current ids are the session's view on that engine.
