# lifecycle.rs

## fn skeleton_function_source
The skeleton is refused at commit as it stands: its `record<>`s are open, and the contract
holds an open record only nested, so a scaffolded stub cannot commit as if it were
finished. Its comment names the two ways out, declared fields or `data: nothing`.

## fn establish_equip
Writes an establish-time rig index recording `<source_dir>/rig` BEFORE any validation,
because that recorded path is what every later `commit` re-reads, by its parent, and what
`check_source_dir` cross-checks. The equip exists as an addressable empty rig first, and
gains content on its first commit.

## fn scaffold_leaf
EMITS THE CASCADE CONVENTION: a call gets BOTH `export module` and `export use`, while a
module-path segment gets `export module` alone. That is nushell's own prescription from the
0.114.0 release notes (PR #18303, "Submodules are no longer implicitly imported"), not a
house rule - importing a module no longer implicitly imports its exported submodules, and
the documented fix is to keep `export module` and ADD `export use`.

Verified against 0.114.1: with the inline `export module sub` alone, `use foo` then
`foo sub baz` fails; adding `export use sub` restores it.

The LEAF-GUARD refuses an existing terminal namepath, so scaffolding can never write over
authored work. Intermediate module dirs are additively wired and may already exist.

Its answer names the files it wrote, a new module's `mod.nu` and a call's, never a
directory it walked through: a batch scaffolding a module and a call under it
otherwise named the module twice and its `mod.nu` not at all.

## fn registered_source
The equip's source is the PARENT of either kind's recorded source, the rig's first. A
registration is the per-kind meta alone, so there is no equip-level record to drift from the
kinds it describes. The rig's is read by `load_rig_source`, the source alone, so a rig an
older build committed commits again.

## fn promote_equip
VALIDATES, never regenerates - author content and docs survive a commit untouched.

The equip's canonical subtree is WIPED before the copy, both kinds with it, so a kind gone
from the source leaves the namespace and each `.meta/` is rebuilt from this validation
rather than merged with the last one.

An empty porcelain means idempotent no-op: grouped-but-empty result and NO commit, so
re-committing an unchanged equip does not accumulate history.

The validator's WARNING rows ride in the result, the no-op's included: a source that
validates commits past its warnings, and the agent still reads them.

## fn install_equip
ATOMIC by order: the source is validated whole before anything is written, so a failed
install leaves nothing registered and nothing to roll back.

## fn copy_rig_tree
Mirrors the validator's carried set exactly. Root `.assets/` and `.docs/` copy VERBATIM
including dotfiles, while everything else skips dotfile entries at every level - which is
what keeps a client `.git` out of the signed repo.

`.gitignore` is honored only by commit's `git add` at the STAGING step, so it governs what
enters the repo and is NOT a validation exemption: the walk covers the full filesystem, so
an ignored-but-invalid file still blocks the commit.
