# optzip/mod.nu

The OptZip convention, from the signed bundle infra-k3 ships: an outer zstd
tar of exactly three files, so a consumer checks the record and the signature
before it trusts a byte of the inner tar.

## const REPRODUCIBLE
GNU tar's own clock, owners, and directory order would make two builds of the
same tree differ. Sorted names, a fixed mtime, numeric owner and group 0, and
pax headers stripped of atime and ctime, with a stable header name, leave the
bytes depending on content alone, so the sha256 record means something across
builds.

## def file_name
An underscore separates the fields, so a field holding one would make the
name ambiguous; the fields' own separators are dots and dashes. The version
may hold dots, and still parses: the name comes first, the os and arch last.

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
`.`, so no member carries a `./` prefix.

THE PRODUCER VERIFIES ITS OWN SIGNATURE: the signer's public key is exported
into the stage and gpgv checks the detached signature against it before the
package exists. That keyring is a producer check alone; a consumer brings its
own trust.

The package is written as a `.partial` beside its final name and moved into
place only after the verify, so a failed signing leaves the last good package
where it was. The stage is ours by construction, removed at the start of the
next pack if a failure left it behind.

zstd runs single-threaded at level 10, as infra-k3 runs it: a multi-threaded
zstd frames its output differently.
