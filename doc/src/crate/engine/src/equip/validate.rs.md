# validate.rs

## fn cap_diagnostics
Caps each SEVERITY independently, so a flood of warnings cannot crowd out the errors that
actually block a commit.

## fn extract_doc
The first-blank-line split is nushell's OWN convention for a doc comment - the parser's
`build_desc` does the same thing to produce `description` and `extra_description` - so a
mod.nu leading comment and a `main` doc comment divide identically.

## fn check_summary_length
A WARNING rather than an error, so an over-long summary COMMITS and then surfaces in
`interface equip check`. Only Error-severity rows block.

## const NAME_DENYLIST
These names would collide with conventional directory meanings inside a namespace, so they
are refused wherever an equip is named rather than being allowed to produce a confusing
tree.

## const HELP_WORDS
`interface help interface` and `interface help grimm` are answered before any namepath is
read, so an equip under either word could never be helped by its own name. Each is held
from an author as well as a name (User), the same refusal as a denied name.

## fn is_executable
Any execute bit on Unix. Windows keeps no execute bit, and an extension in
`PATHEXT` is the only mark it gives a runnable file, so that is the test off
Unix. A `PATHEXT` naming `.NU` would mark every `.nu` file executable, each
then refused in every zone. The refusal's message names the execute bit,
which on Windows is never the cause: the extension is.

## struct SelfView
A temp `<tmp>/equip/<author>/<name>/<kind>` symlink per source kind, prepended to
`NU_LIB_DIRS`, so an equip's OWN equip-prefixed `use` - a rig importing its own lib or its
own modules - resolves against the source during its FIRST commit, before it has been placed
in the namespace at all. ONE view serves both kinds, which is what lets a rig's import of its
lib see the lib installed with it rather than a stale committed copy.

The symlink is needed because a call target's `main` parses DURING the rig's own load,
before the equip finishes registering: a bare in-tree self-reference would resolve to
nothing and surface as "External command failed", so the view gives its equip-prefixed
`use` something to resolve against.

The sequence number is what keeps two concurrent validations from sharing a view directory,
and the view exists before its first link, so a failed link drops it whole.

Each link is a symlink on Unix and a directory symlink, `symlink_dir`, on
Windows, where making one is a privileged act: without Developer Mode,
`SeCreateSymbolicLinkPrivilege`, or an administrator's token the first link
fails, no view is made, and an equip's own equip-prefixed `use` resolves as if
none existed: to nothing before the equip is placed in the namespace, and to
its committed copy once it is. The view's drop removes the links, never what
they point at, since `remove_dir_all` follows no symbolic link.

## fn validate_equip_source
Reports ALL findings across both kinds AND builds the rig's index and docs in the SAME walk,
so a commit never parses the tree twice. A finding's path carries `<equip>/<kind>/`, so it
names the kind it belongs to.

`ValidationResult::is_empty()` means "no ERROR-severity diagnostics" - errors gate a commit,
warnings advise.

## fn validate_lib_tree
A lib is plain nushell: no cascade, no call rules, no reserved term. Its one structural
rule is `.nu` alone, with no exemption for a `.gitignore` or a README, and nothing
executable.

## fn validate_lib_file
Each file parses as a module of its own, against the same engine and self-view a rig
gets, so a lib module importing a sibling or another equip resolves it as nushell would at
serve. A failed parse is checked for a cycle before its errors are reported, as in
`validate_flat_file`.

## fn validate_walk
A CALL IS AN EDGE MODULE: the walk validates it in place and never recurses into it, which
is what makes `rig::call_leaf` a rule rather than an accident.

Each child is tested against the edges NAMING it, and each check asks for the PRESENCE of a
required kind rather than keying on whichever edge was written first - a child may legally
carry more than one edge, which is exactly the case the convention prescribes.

THE VALIDATOR IS LOOSER THAN THE CONVENTION, deliberately for now: it requires an
`export use` edge on a call and SOME edge on a pure module, but does not require
`export module` on a call nor reject `export use` on a pure module. So a hand-authored call
wired `export use`-only still commits. The estate is now conformant, so tightening would
invalidate nothing; the ordering is the constraint - an enforcement change that turns valid
artifacts into violations cannot precede the sweep that makes them valid.

## fn parse_module_has_main
Classification by nushell's OWN `Module.main` sentinel rather than by grepping for
`export def main`, so the question "is this a call target" is answered by the parser that
will actually run it.

## fn extract_module_edges
A parse failure yields NO edges rather than an error, because the cascade validator reports
that parse error separately - returning edges from a broken parse would produce a second,
misleading orphan diagnostic on top of the real one.

## fn validate_flat_file
A FAILED parse is checked for a cycle BEFORE its errors are reported, as the eval path
does: a cycle's own parse errors are unstable in variant and payload and useless in every
form. A clean parse is never checked, since a diamond-shaped import graph registers its
shared modules many times without any cycle.

## fn scan_reserved_terms
`main` may appear ONLY as a call target's exported sentinel. The scan checks path components
first, then walks `flatten_block` FlatShape tokens. The components are the platform's own
reading of the path, so a Windows path holding either separator yields each name; a split on
`/` alone read `main\mod.nu` there as one name and let a module named `main` through.

Why those two shapes and no others: `VarDecl` catches let/mut/const names and `String`
catches def/module/alias names, record keys, cell-path members and barewords - EXCEPT a
`String` immediately after an `export def` InternalCall token, which is the valid sentinel
export. Quoted string VALUES keep their quotes in the token so they never match, and command
references parse as InternalCall or External so they skip. That leaves exactly
identifier-position `main` outside the sentinel.

A token's text is read through `span_text`, from the file's start in the span space,
never by the raw span: the engine's own files, the standard library's first, sit ahead of
this one, and a raw read returns nothing, which would pass `main` anywhere silently.

Every parse site in this file registers the file's ABSOLUTE path as the parse fname AND
pushes it onto `working_set.files` before parsing. The PUSH is the load-bearing half: `path self` reads
`working_set.files.top()`, not the parse fname - the fname only feeds span mapping - so
without the push a parse-time `const SELF = (path self)` const-resolves at serve and fails
at validate. This mirrors nushell's own module-file loader.

## fn validate_mod_nu_ast
A private `use` or `overlay use` parses to a Call to its decl on nu 0.116.1, its import
pattern kept as the call's parser info, so the decl-name allowlist admits it whether its
module resolved or not. The arm for a bare `Expr::ImportPattern` or `Expr::Overlay`
admits a form 0.116.1 does not produce at a module's top level.

`let` at module body level is a nu parse error and surfaces before this structural walk ever
runs.

## fn validate_function_file_ast
THE CONTRACT IS EXECUTE'S, User's ruling: `main`'s parsed signature is held by
`contract_signature`, the check an `execute` definition's signature passes, each refusal a
`rig::contract` row at `main`'s line. A call and a definition therefore take data, a
timeout, and a return alike, and the typedefs the check answers go into the index as
they are. The scaffold's `record<>` skeleton falls to the same check, an open record
standing only nested, which is what stops a stub from committing as if it were finished.

Every position here is measured from the file's start in the span space (`file_start`),
never from zero: the engine's own files, the standard library's first, sit ahead of this
one.
