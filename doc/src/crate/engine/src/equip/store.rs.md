# store.rs

The equip substrate is a full nu ECOSYSTEM, not merely a `call()` registry. A rig's call
targets are the indexed "bins", but EVERY export - a rig's call target or helper, a lib's
module - is importable from a body and from external nu scripting.

## struct IndexFunction
The call's contract as typed data, the types `contract_signature` read from `main` at
commit, so a call holds its data and result to them and the signature block renders them
without parsing the rig's source again. Each type is written as its text.

## enum StoredType / enum EarlierType / fn stored_type
The build before nutype wrote each type as the serde structure of its own model,
`{Record: {fields: [...]}}`. An entry reads either form, the earlier converted, so a
namespace's installed rigs keep answering after an upgrade; the next commit writes the
text. The forms can't collide: no type text is a record, and the earlier form's one
string, `Nothing`, names no type.

## struct IndexSource
The index's source path alone, which an older build's index carries as this one's does.
An index an older build wrote, its entries in another shape, fails to decode whole until
its rig is committed again, and the commit reads the source from it first; this reading
is what lets that commit happen.

## const META_FILE
This file is ALSO the rig-detection marker: an installed equip whose `rig/` carries it HAS a
rig, and `LIB_META_FILE` marks its lib the same way. Detection keys on the exact filenames.

## const EQUIP_KINDS
The lib comes FIRST because the rig may import it: validation and the copy run in this order,
and a lib-only equip is as legitimate as a rig-only one. `cli` is absent by design - the MCP
never takes an equip's interactive kind.

## fn is_valid_equip
EXACTLY `<author>/<name>`: two parts, each a valid ident, neither `main`. A BARE equip with no
slash is REJECTED: there is one spelling of an equip everywhere, with no default-author
shorthand to disambiguate.

## fn equip_namepath_of
The namepath comes from the PATH, so the source's location is the one statement of the
equip's name: `.../equip/<author>/<name>` and nothing else. A `.` or `..` component is refused
rather than normalized, because the registered source is compared component-wise later and a
normalized alias would name a different directory than the one it spells. A Windows path's
drive prefix, `C:`, is passed over as the root is.

## fn require_rig
Two refusals, kept apart: an equip that is not installed is `equip::not_registered`, while an
installed one with no rig is `equip::no_rig`. A call or a scaffold into a lib-only equip is a
misunderstanding of what was installed, not a typo in its name.

## struct RigLocks
The outer mutex guards only MAP MUTATION; the per-equip `RwLock` is what serializes work.
Call takes a READ lock, and commit / new / install / uninstall take WRITE, so different equips
are fully parallel on the read/validate side.

### fn hydrate_from_disk
Pre-creates a lock per installed equip at startup, keying on either kind's meta file, so a
`lookup` during serving never has to decide whether a directory is an equip.

## static EQUIP_REPO_LOCK / fn equip_repo_lock
The repo-wide write lock the per-equip locks cannot supply. The belief that "git's index
locking serializes the subprocess commits" was WRONG and measured so: git fails a second
writer on an existing `.git/index.lock` immediately rather than waiting, so two concurrent
lifecycle ops on DIFFERENT rigs through one host spuriously failed (five of five parallel
integration sweeps, `index.lock` in each). One repo, one lock, held only across the
canonical-tree wipe/copy plus the git section - validation is the expensive half and stays
outside the hold, so different-equip validations still parallelize. The host's purview
table rides the same lock (mcp's `purview_add_rig` and `purview_remove_rig`): it is
namespace meta beside the repo whose load-mutate-save is not atomic. Namespace isolation per `(id, namespace)` still means no cross-process race to
guard; this lock is for one process's own concurrency.

## fn ensure_keypair
`allowed_signers` is rewritten from the current pubkey on EVERY startup rather than only at
creation, so it cannot drift out of step with the key it authorizes.

## fn ensure_equip_repo
The empty init commit exists so HEAD is valid immediately - a fresh repo with no commit
makes the first `git status --porcelain` comparison meaningless.

`configure_repo` runs on every startup, not just creation, so a moved keypair path
re-applies rather than leaving a repo that silently cannot sign.

It reports whether it made the repository. A namespace whose repository this startup made
is NEW, and a new namespace is the one moment the host installs anything on a user's
behalf.

## fn configure_repo
PER-REPO configuration only. Nothing here leaks into the user's global git config, and
`commit.gpgSign` being set locally is what makes every lifecycle commit signed implicitly
rather than by remembering a flag.

## fn git_command
Every git the store and the lifecycle run starts here. On Windows it passes
`core.longpaths=true` on the command line, so git writes objects past
`MAX_PATH`, 260 characters, under a deep data home, and so `git init` takes it
too, before the repository has a config to carry it.

## fn load_rig_source
Every reader that needs the source alone, the commit, the source check, and the scaffold,
reads it here rather than through `load_index`, so an index whose entries this build no
longer reads never blocks the commit that writes them anew.

## fn registered_equip_names
The plain sort of the compound `<author>/<name>` gives author-then-name ordering for free,
since the author is the leading segment. The repository's own `.git` is skipped by its dot,
and a directory carrying neither kind's meta is not an equip.

## fn registered_rig_names
The installed equips WITH a rig - the only ones that have calls, so the signature block and
the purview pruning read this set rather than every equip.

## fn ensure_equip_store
The keypair and the repository under the repository's lock, the store's half of the host's
substrate. Its answer, whether this startup made the repository, is what the host keys the
builtins on.

## fn bundled_equip_dir
The deploy's own `equip/`, beside `bin/` as the plugins' `plugin/` is, so a deploy carries
the builtins' sources and the host finds them without being told where.

## fn install_builtins
Each builtin installs as `equip install` would, validated whole, its source the deploy's
copy. One the deploy doesn't hold is skipped, and a failure goes to stderr while the host
serves on: a builtin is a convenience, never a condition of serving. The names that
installed come back, so the host seeds their purviews.
