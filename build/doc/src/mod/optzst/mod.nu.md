# optzst/mod.nu

The OptZst convention, from the signed bundle infra-k3 ships: an outer zstd tar
of exactly six files, two tars each with its record and its signature, so a
consumer checks both before it trusts a byte of either. `<name>.tar` is the
program's tree; `package-<name>.tar` is the package's own part, its
`package.nuon` and the install scripts that place the tree. The module and the
scripts are generic to any package: grammar prototypes OptZst until it becomes
a project of its own.

## const REPRODUCIBLE
GNU tar's own clock, owners, and directory order would make two builds of the
same tree differ. Sorted names, a fixed mtime, numeric owner and group 0, and
pax headers stripped of atime and ctime, with a stable header name, leave the
bytes depending on content alone, so the sha256 records mean something across
builds.

## const SCRIPT_DIR
The scripts live beside this module, found from its own path, so a package's
build names no path for them.

## const SCRIPTS
`install.bash` for Linux and macOS, `install.ps1` for Windows and for any host
with PowerShell; each package carries both.

## const PACKAGE_FIELDS
A `package.nuon` holds `name`, `title`, and `summary` and nothing else, so a
field misspelled or added is refused rather than carried.

## const SUMMARY_MAX
A package's summary is one line of 80 characters at most.

## def file_name
An underscore separates the fields, so a field holding one would make the
name ambiguous; the fields' own separators are dots and dashes. The version
may hold dots, and still parses: the name comes first, the os and arch last.

## def package_record
The record is read and checked before anything is built from it: its three
fields and no others, each a non-empty string, its summary within the limit.
The name is the package's home's, `opt/<name>`, the inner tar's stem, and the
name the install scripts read.

## def target
Rust's `target_os` and `target_arch` rather than `uname`: the names cargo built
for, the same when a cross build arrives. `_` becomes `-` because the file name
reserves it; x86_64 is the case that needs it.

## def signer
A repository that signs its commits with gpg already names a key, so the
package takes it. One signing with SSH, or not at all, names no OpenPGP key, so
the build refuses until `gpg.signingkey` holds a FULL fingerprint: a short key
id can match more than one key. `GNUPGHOME` wins over `gpg.homedir`, as it does
for gpg itself.

## def pack
The inner tar's members are the tree's top-level entries, listed rather than
`.`, so no member carries a `./` prefix. The package tar is made from a stage
of its own three files, the record at 0644 and the scripts at 0755, under the
same reproducible options.

THE PRODUCER VERIFIES ITS OWN SIGNATURES: the signer's public key is exported
into the stage and gpgv checks each detached signature against it before the
package exists. That keyring is a producer check alone; a consumer brings its
own trust.

The package is written as a `.partial` beside its final name and moved into
place only after both verify, so a failed signing leaves the last good package
where it was. The stage is ours by construction, removed at the start of the
next pack if a failure left it behind.

zstd runs single-threaded at level 10, as infra-k3 runs it: a multi-threaded
zstd frames its output differently.
