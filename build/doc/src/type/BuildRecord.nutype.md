# BuildRecord.nutype

The record `build.nu --tree <dir>` writes beside its tree, `<dir>/build.nuon`,
and `build.nu --pack <dir>` reads: the package's name, its semver, and the
target's os and arch in Rust's names, `_` written `-`, the four fields of the
OptZip's file name.
