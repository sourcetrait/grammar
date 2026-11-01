# error.rs

## enum SoakError
`Cork` is transparent: its message and its labeled form are the filesystem
error's own. Without it snafu printed the variant's name, so every path error
read `Cork`.

The overlap and destination errors label the `dst` argument; the others
label the call.

## enum NuType
The nushell types a fill cannot carry into Liquid: binary that is not UTF-8,
a custom value without a plain base value, a closure, a date Liquid's parser
refuses, and an error.
